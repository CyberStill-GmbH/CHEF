//! Clasificación CHEF de respuestas wildcard obtenidas por consultas autorizadas.
//!
//! Este módulo solo compara resultados entregados por la aplicación. No crea
//! nombres aleatorios, consulta resolvers, cambia el scope ni modifica activos.

use std::collections::{BTreeMap, BTreeSet};

use chef_core::{dns_parent_zones, extract_dns_targets};

use crate::{DnsAnswer, DnsLookupOutcome, DnsQueryRequest, DnsResolutionError};

const MAX_WILDCARD_PROBES_PER_ZONE: usize = 8;
const MAX_WILDCARD_ZONES: usize = 32;
const MAX_WILDCARD_PROBE_SETS: usize = 256;
const MAX_WILDCARD_ANSWER_COUNT: usize = 8192;
const MAX_WILDCARD_BASELINE_COUNT: usize = 4096;
const MAX_WILDCARD_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Respuestas ya obtenidas para un tipo consultado sobre el nombre objetivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardBaseline {
    record_type: String,
    answers: Vec<DnsAnswer>,
}

impl DnsWildcardBaseline {
    pub fn new(record_type: &str, answers: Vec<DnsAnswer>) -> Result<Self, DnsResolutionError> {
        let record_type = crate::DnsRecordType::parse(record_type)?;
        if answers.is_empty() {
            return Err(DnsResolutionError::InvalidQuery);
        }
        if answers.len() > MAX_WILDCARD_BASELINE_COUNT {
            return Err(DnsResolutionError::LimitExceeded);
        }
        Ok(Self {
            record_type: record_type.as_str().to_owned(),
            answers,
        })
    }

    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn answers(&self) -> &[DnsAnswer] {
        &self.answers
    }
}

/// Respuestas de una muestra de nombres de prueba para una zona y RR type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardProbeSet {
    zone: String,
    record_type: String,
    probes: Vec<DnsLookupOutcome>,
}

impl DnsWildcardProbeSet {
    /// Construye un conjunto acotado de resultados por zona y tipo.
    pub fn new(
        zone: &str,
        record_type: &str,
        probes: Vec<DnsLookupOutcome>,
    ) -> Result<Self, DnsResolutionError> {
        let request = DnsQueryRequest::new(zone, record_type, 0)?;
        if request.host().parse::<std::net::IpAddr>().is_ok() {
            return Err(DnsResolutionError::InvalidQuery);
        }
        if probes.is_empty() {
            return Err(DnsResolutionError::InvalidQuery);
        }
        if probes.len() > MAX_WILDCARD_PROBES_PER_ZONE {
            return Err(DnsResolutionError::LimitExceeded);
        }
        Ok(Self {
            zone: request.host().to_owned(),
            record_type: request.record_type().as_str().to_owned(),
            probes,
        })
    }

    pub fn zone(&self) -> &str {
        &self.zone
    }

    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn probes(&self) -> &[DnsLookupOutcome] {
        &self.probes
    }
}

/// Resultado independiente por tipo de registro; nunca es una vulnerabilidad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsWildcardDisposition {
    Confirmed { zone: String },
    Possible { zone: String },
    NotDetected,
    Inconclusive,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardRecordAssessment {
    record_type: String,
    disposition: DnsWildcardDisposition,
}

impl DnsWildcardRecordAssessment {
    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn disposition(&self) -> &DnsWildcardDisposition {
        &self.disposition
    }
}

/// Clasificación determinista de los RR baselines para un nombre observado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardAssessment {
    target: String,
    records: Vec<DnsWildcardRecordAssessment>,
    tags: Vec<String>,
    suggested_event_host: Option<String>,
}

impl DnsWildcardAssessment {
    /// Compara baselines agrupados por RR con muestras ya obtenidas por el llamador.
    ///
    /// `probe_sets` debe incluir las consultas que se ejecutaron. Un conjunto
    /// ausente o una muestra fallida no se interpreta como ausencia de wildcard.
    pub fn classify(
        target: &str,
        baseline: &[DnsWildcardBaseline],
        probe_sets: &[DnsWildcardProbeSet],
        is_seed: bool,
    ) -> Result<Self, DnsResolutionError> {
        let request = DnsQueryRequest::new(target, "A", 0)?;
        let target = request.host().to_owned();
        if target.parse::<std::net::IpAddr>().is_ok() {
            return Err(DnsResolutionError::InvalidQuery);
        }
        if baseline.len() > MAX_WILDCARD_BASELINE_COUNT
            || probe_sets.len() > MAX_WILDCARD_PROBE_SETS
        {
            return Err(DnsResolutionError::LimitExceeded);
        }

        let zones = dns_parent_zones(&target);
        if zones.len() > MAX_WILDCARD_ZONES {
            return Err(DnsResolutionError::LimitExceeded);
        }

        let mut probe_index = BTreeMap::new();
        let mut answer_count = 0usize;
        let mut response_bytes = 0usize;
        let mut baseline_by_type = BTreeMap::new();
        for item in baseline {
            if baseline_by_type
                .insert(item.record_type().to_owned(), item.answers())
                .is_some()
            {
                return Err(DnsResolutionError::InvalidQuery);
            }
            for answer in item.answers() {
                answer_count = answer_count.saturating_add(1);
                response_bytes = response_bytes.saturating_add(answer.rdata().len());
            }
        }
        for probe_set in probe_sets {
            if !zones.iter().any(|zone| zone == probe_set.zone())
                || probe_index
                    .insert(
                        (
                            probe_set.record_type().to_owned(),
                            probe_set.zone().to_owned(),
                        ),
                        probe_set,
                    )
                    .is_some()
            {
                return Err(DnsResolutionError::InvalidQuery);
            }
            for probe in probe_set.probes() {
                if let DnsLookupOutcome::Answers(answers) = probe {
                    answer_count = answer_count.saturating_add(answers.len());
                    for answer in answers {
                        response_bytes = response_bytes.saturating_add(answer.answer.rdata().len());
                    }
                    if answer_count > MAX_WILDCARD_ANSWER_COUNT {
                        return Err(DnsResolutionError::LimitExceeded);
                    }
                }
            }
        }
        if answer_count > MAX_WILDCARD_ANSWER_COUNT || response_bytes > MAX_WILDCARD_RESPONSE_BYTES
        {
            return Err(DnsResolutionError::LimitExceeded);
        }

        let mut records = Vec::with_capacity(baseline_by_type.len());
        for (record_type, answers) in baseline_by_type {
            let disposition = classify_record(&record_type, answers, &zones, &probe_index)?;
            records.push(DnsWildcardRecordAssessment {
                record_type,
                disposition,
            });
        }

        let all_confirmed = !records.is_empty()
            && records.iter().all(|record| {
                matches!(record.disposition, DnsWildcardDisposition::Confirmed { .. })
            });
        let suggested_event_host = if all_confirmed && !is_seed {
            records.iter().find_map(|record| {
                if let DnsWildcardDisposition::Confirmed { zone } = &record.disposition {
                    Some(format!("_wildcard.{zone}"))
                } else {
                    None
                }
            })
        } else {
            None
        };

        let mut tags = BTreeSet::new();
        for record in &records {
            let record_tag = record.record_type.to_ascii_lowercase();
            match &record.disposition {
                DnsWildcardDisposition::Confirmed { .. } => {
                    tags.insert("wildcard".to_owned());
                    tags.insert(format!("{record_tag}-wildcard"));
                }
                DnsWildcardDisposition::Possible { .. } => {
                    tags.insert("wildcard-POSSIBLE".to_owned());
                    tags.insert(format!("{record_tag}-wildcard-POSSIBLE"));
                }
                DnsWildcardDisposition::NotDetected
                | DnsWildcardDisposition::Inconclusive
                | DnsWildcardDisposition::NotEvaluated => {}
            }
        }

        Ok(Self {
            target,
            records,
            tags: tags.into_iter().collect(),
            suggested_event_host,
        })
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn records(&self) -> &[DnsWildcardRecordAssessment] {
        &self.records
    }

    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Alias candidato para la UI; no reescribe la identidad CHEF guardada.
    pub fn suggested_event_host(&self) -> Option<&str> {
        self.suggested_event_host.as_deref()
    }
}

fn classify_record(
    record_type: &str,
    baseline: &[DnsAnswer],
    zones: &[String],
    probe_index: &BTreeMap<(String, String), &DnsWildcardProbeSet>,
) -> Result<DnsWildcardDisposition, DnsResolutionError> {
    if baseline.is_empty() || zones.is_empty() {
        return Ok(DnsWildcardDisposition::NotEvaluated);
    }

    let baseline_raw = baseline
        .iter()
        .map(|answer| answer.rdata().to_owned())
        .collect::<BTreeSet<_>>();
    let mut baseline_targets = BTreeSet::new();
    for answer in baseline {
        baseline_targets.extend(
            extract_dns_targets(answer.record_type().as_str(), answer.rdata())
                .map_err(|_| DnsResolutionError::LimitExceeded)?
                .into_iter()
                .map(|target| target.value),
        );
    }

    let mut had_any_failure = false;
    for zone in zones {
        let Some(probe_set) = probe_index.get(&(record_type.to_owned(), zone.clone())) else {
            return Ok(DnsWildcardDisposition::Inconclusive);
        };
        let mut wildcard_raw = BTreeSet::new();
        let mut wildcard_targets = BTreeSet::new();
        for probe in probe_set.probes() {
            match probe {
                DnsLookupOutcome::Answers(answers) => {
                    for evidence in answers {
                        let answer = &evidence.answer;
                        wildcard_raw.insert(answer.rdata().to_owned());
                        wildcard_targets.extend(
                            extract_dns_targets(answer.record_type().as_str(), answer.rdata())
                                .map_err(|_| DnsResolutionError::LimitExceeded)?
                                .into_iter()
                                .map(|target| target.value),
                        );
                    }
                }
                DnsLookupOutcome::Failed(_) => had_any_failure = true,
                DnsLookupOutcome::NoAnswers => {}
            }
        }

        if !wildcard_raw.is_empty() {
            let matches = baseline_raw
                .iter()
                .any(|answer| wildcard_raw.contains(answer))
                || baseline_targets
                    .iter()
                    .any(|target| wildcard_targets.contains(target));
            return Ok(if matches {
                DnsWildcardDisposition::Confirmed { zone: zone.clone() }
            } else {
                DnsWildcardDisposition::Possible { zone: zone.clone() }
            });
        }
    }

    Ok(if had_any_failure {
        DnsWildcardDisposition::Inconclusive
    } else {
        DnsWildcardDisposition::NotDetected
    })
}
