//! Clasificación CHEF de respuestas wildcard obtenidas por consultas autorizadas.
//!
//! Este módulo solo compara resultados entregados por la aplicación. No crea
//! nombres aleatorios, consulta resolvers, cambia el scope ni modifica activos.

use std::collections::{BTreeMap, BTreeSet};

use chef_core::{dns_parent_zones, extract_dns_targets};

use crate::{
    DnsAnswer, DnsAuthorizationPort, DnsCancellation, DnsLookupOutcome, DnsPlanTermination,
    DnsQueryRequest, DnsResolutionError, DnsResolutionResult, DnsResolverPort, ResolveDns,
};

const MAX_WILDCARD_PROBES_PER_ZONE: usize = 8;
const MAX_WILDCARD_ZONES: usize = 32;
const MAX_WILDCARD_PROBE_SETS: usize = 256;
const MAX_WILDCARD_ANSWER_COUNT: usize = 8192;
const MAX_WILDCARD_BASELINE_COUNT: usize = 4096;
const MAX_WILDCARD_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_WILDCARD_PROBE_REQUESTS: usize = 256;
const WILDCARD_SAMPLE_COUNT: usize = 5;
const WILDCARD_SAMPLE_LABEL_LENGTH: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsWildcardPlanStatus {
    Ready,
    SeedSkipped,
    NoParentZones,
    AllZonesExcluded,
}

/// Solicitudes para un par zona/tipo; cada solicitud usa un nombre de muestra
/// diferente bajo la misma zona para reducir respuestas de caché compartidas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardProbeGroup {
    zone: String,
    record_type: String,
    requests: Vec<DnsQueryRequest>,
}

impl DnsWildcardProbeGroup {
    pub fn zone(&self) -> &str {
        &self.zone
    }

    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn requests(&self) -> &[DnsQueryRequest] {
        &self.requests
    }
}

/// Plan puro de muestras wildcard. Las etiquetas impredecibles las entrega el
/// caller; crear este plan no consulta DNS ni implica autorización de alcance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardProbePlan {
    target: String,
    status: DnsWildcardPlanStatus,
    groups: Vec<DnsWildcardProbeGroup>,
}

impl DnsWildcardProbePlan {
    /// Construye cinco muestras por zona y tipo desde baselines existentes.
    /// `sample_labels` debe contener cinco etiquetas frescas, únicas, de diez
    /// letras minúsculas ASCII; se reutilizan entre pares zona/tipo.
    /// `excluded_zones` contiene zonas canónicas que el plan debe omitir.
    pub fn for_target(
        target: &str,
        baseline: &[DnsWildcardBaseline],
        sample_labels: &[String],
        excluded_zones: &[String],
        is_seed: bool,
    ) -> Result<Self, DnsResolutionError> {
        let request = DnsQueryRequest::new(target, "A", 0)?;
        if request.host().parse::<std::net::IpAddr>().is_ok() {
            return Err(DnsResolutionError::InvalidQuery);
        }
        let target = request.host().to_owned();

        if is_seed {
            return Ok(Self {
                target,
                status: DnsWildcardPlanStatus::SeedSkipped,
                groups: Vec::new(),
            });
        }

        let zones = dns_parent_zones(&target);
        if zones.len() > MAX_WILDCARD_ZONES {
            return Err(DnsResolutionError::LimitExceeded);
        }
        if zones.is_empty() {
            return Ok(Self {
                target,
                status: DnsWildcardPlanStatus::NoParentZones,
                groups: Vec::new(),
            });
        }

        if excluded_zones.len() > MAX_WILDCARD_ZONES {
            return Err(DnsResolutionError::LimitExceeded);
        }
        let excluded_zones = excluded_zones
            .iter()
            .map(|zone| {
                let request = DnsQueryRequest::new(zone, "A", 0)?;
                if request.host().parse::<std::net::IpAddr>().is_ok() {
                    return Err(DnsResolutionError::InvalidQuery);
                }
                Ok(request.host().to_owned())
            })
            .collect::<Result<BTreeSet<_>, DnsResolutionError>>()?;
        let zones = zones
            .into_iter()
            .filter(|zone| {
                !excluded_zones
                    .iter()
                    .any(|excluded| zone == excluded || zone.ends_with(&format!(".{excluded}")))
            })
            .collect::<Vec<_>>();
        if zones.is_empty() {
            return Ok(Self {
                target,
                status: DnsWildcardPlanStatus::AllZonesExcluded,
                groups: Vec::new(),
            });
        }

        if baseline.is_empty() || baseline.len() > MAX_WILDCARD_PROBE_SETS {
            return Err(if baseline.is_empty() {
                DnsResolutionError::InvalidQuery
            } else {
                DnsResolutionError::LimitExceeded
            });
        }
        if zones.len().saturating_mul(baseline.len()) > MAX_WILDCARD_PROBE_SETS {
            return Err(DnsResolutionError::LimitExceeded);
        }
        validate_sample_labels(sample_labels)?;

        let mut record_types = BTreeSet::new();
        for item in baseline {
            if !record_types.insert(item.record_type().to_owned()) {
                return Err(DnsResolutionError::InvalidQuery);
            }
        }
        let request_count = zones
            .len()
            .checked_mul(record_types.len())
            .and_then(|groups| groups.checked_mul(WILDCARD_SAMPLE_COUNT))
            .ok_or(DnsResolutionError::LimitExceeded)?;
        if request_count > MAX_WILDCARD_PROBE_REQUESTS {
            return Err(DnsResolutionError::LimitExceeded);
        }

        let mut groups = Vec::with_capacity(zones.len() * record_types.len());
        for zone in zones {
            for record_type in &record_types {
                let requests = sample_labels
                    .iter()
                    .map(|label| DnsQueryRequest::new(&format!("{label}.{zone}"), record_type, 0))
                    .collect::<Result<Vec<_>, _>>()?;
                groups.push(DnsWildcardProbeGroup {
                    zone: zone.clone(),
                    record_type: record_type.clone(),
                    requests,
                });
            }
        }

        Ok(Self {
            target,
            status: DnsWildcardPlanStatus::Ready,
            groups,
        })
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn status(&self) -> DnsWildcardPlanStatus {
        self.status
    }

    pub fn groups(&self) -> &[DnsWildcardProbeGroup] {
        &self.groups
    }
}

fn validate_sample_labels(labels: &[String]) -> Result<(), DnsResolutionError> {
    if labels.len() != WILDCARD_SAMPLE_COUNT {
        return Err(DnsResolutionError::InvalidQuery);
    }
    let mut unique = BTreeSet::new();
    for label in labels {
        if label.len() != WILDCARD_SAMPLE_LABEL_LENGTH
            || !label.bytes().all(|byte| byte.is_ascii_lowercase())
            || !unique.insert(label)
        {
            return Err(DnsResolutionError::InvalidQuery);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardProbeExecution {
    zone: String,
    record_type: String,
    expected_query_count: usize,
    results: Vec<DnsResolutionResult>,
}

impl DnsWildcardProbeExecution {
    pub fn zone(&self) -> &str {
        &self.zone
    }

    pub fn record_type(&self) -> &str {
        &self.record_type
    }

    pub fn results(&self) -> &[DnsResolutionResult] {
        &self.results
    }

    /// Solo los grupos completos se convierten en muestras clasificables.
    /// Una autorización rechazada o cancelación deja visible el resultado
    /// parcial, pero no lo presenta como una evaluación negativa.
    pub fn complete_probe_set(&self) -> Result<Option<DnsWildcardProbeSet>, DnsResolutionError> {
        if self.results.len() != self.expected_query_count {
            return Ok(None);
        }
        let outcomes = self
            .results
            .iter()
            .map(|result| result.outcome.clone())
            .collect();
        DnsWildcardProbeSet::new(&self.zone, &self.record_type, outcomes).map(Some)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWildcardPlanExecution {
    groups: Vec<DnsWildcardProbeExecution>,
    termination: DnsPlanTermination,
}

impl DnsWildcardPlanExecution {
    pub fn groups(&self) -> &[DnsWildcardProbeExecution] {
        &self.groups
    }

    pub fn termination(&self) -> DnsPlanTermination {
        self.termination
    }

    /// Devuelve únicamente grupos completos para que el clasificador no
    /// confunda consultas omitidas con ausencia de wildcard.
    pub fn complete_probe_sets(&self) -> Result<Vec<DnsWildcardProbeSet>, DnsResolutionError> {
        let mut sets = Vec::with_capacity(self.groups.len());
        for group in &self.groups {
            if let Some(set) = group.complete_probe_set()? {
                sets.push(set);
            }
        }
        Ok(sets)
    }
}

pub struct ExecuteDnsWildcardPlan<'a, R, A> {
    resolver: &'a R,
    authorization: &'a A,
}

impl<'a, R, A> ExecuteDnsWildcardPlan<'a, R, A>
where
    R: DnsResolverPort,
    A: DnsAuthorizationPort,
{
    pub fn new(resolver: &'a R, authorization: &'a A) -> Self {
        Self {
            resolver,
            authorization,
        }
    }

    /// Ejecuta muestras en serie. Cada nombre vuelve a validar autorización
    /// mediante `ResolveDns`; cancelación/rechazo termina conservando resultados
    /// anteriores para el caller, sin clasificarlos como negativos.
    pub async fn execute(
        &self,
        plan: &DnsWildcardProbePlan,
        cancellation: &DnsCancellation,
    ) -> DnsWildcardPlanExecution {
        let mut groups = Vec::with_capacity(plan.groups().len());
        let resolver = ResolveDns::new(self.resolver, self.authorization);

        for planned_group in plan.groups() {
            let mut group = DnsWildcardProbeExecution {
                zone: planned_group.zone().to_owned(),
                record_type: planned_group.record_type().to_owned(),
                expected_query_count: planned_group.requests().len(),
                results: Vec::with_capacity(planned_group.requests().len()),
            };
            for request in planned_group.requests() {
                match resolver.execute(request.clone(), cancellation).await {
                    Ok(result) => group.results.push(result),
                    Err(error) => {
                        groups.push(group);
                        return DnsWildcardPlanExecution {
                            groups,
                            termination: DnsPlanTermination::Stopped(error),
                        };
                    }
                }
            }
            groups.push(group);
        }

        DnsWildcardPlanExecution {
            groups,
            termination: DnsPlanTermination::Completed,
        }
    }
}

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
