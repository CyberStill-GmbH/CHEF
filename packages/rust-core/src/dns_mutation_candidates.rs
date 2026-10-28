//! Generación local de candidatos DNS derivados de nombres observados.
//!
//! Este módulo no resuelve nombres, no accede a la red ni transforma candidatos
//! en observaciones. Cada salida conserva los IDs de eventos usados como
//! evidencia para la regla CHEF determinista.

use std::collections::{BTreeMap, BTreeSet};

use crate::{BbotEvent, ChefResult, identity};

const RULE_ID: &str = "dns.offline_mutation";
const RULE_VERSION: &str = "4";
const MAX_INPUT_EVENTS: usize = 10_000;
const MAX_ROOTS: usize = 50;
const MAX_CANDIDATES: usize = 10_000;
const MAX_CANDIDATES_PER_ROOT: usize = 100;
const MAX_PROPOSALS_PER_ROOT: usize = 2_000;

/// Límites para una generación local de candidatos; nunca habilitan consultas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsMutationCandidateOptions {
    pub max_candidates: usize,
    pub max_candidates_per_root: usize,
}

impl Default for DnsMutationCandidateOptions {
    fn default() -> Self {
        Self {
            max_candidates: MAX_CANDIDATES,
            max_candidates_per_root: MAX_CANDIDATES_PER_ROOT,
        }
    }
}

/// Motivo local y auditable por el que se propuso un nombre.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DnsMutationReason {
    /// La primera etiqueta fue observada bajo otro dominio del mismo conjunto.
    CrossRootLabel,
    /// Se extrajo una palabra ASCII de una etiqueta observada.
    ExtractedWord,
    /// Se añadió un sufijo numérico corto a una etiqueta observada en la raíz.
    NumericSuffix,
    /// Se ajustó un bloque numérico ya presente en una etiqueta.
    NumberAdjustment,
    /// Se añadió un modificador operativo o una letra al inicio/final.
    WordCloudModifier,
    /// La etiqueta base se conservó como posible mutación local.
    WordCloudBase,
    /// Nombre de servicio o entorno propuesto desde el vocabulario estático local.
    StaticWordlist,
}

/// Nombre propuesto por una regla CHEF; no representa una resolución DNS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMutationCandidate {
    /// Identidad determinista del candidato dentro de raíz y versión de regla.
    pub id: String,
    pub name: String,
    pub root: String,
    pub rule_id: String,
    pub rule_version: String,
    pub reasons: Vec<DnsMutationReason>,
    /// IDs representativos de la derivación (máximo dos; no es una lista exhaustiva).
    pub source_event_ids: Vec<String>,
}

/// Resultado acotado y estable ante cambios en el orden de entrada.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMutationCandidateBatch {
    pub candidates: Vec<DnsMutationCandidate>,
    /// Candidatos repetidos por más de una regla o fuente, fusionados conservando procedencia.
    pub duplicates_suppressed: usize,
    /// Raíces no evaluadas como destino por el máximo de 50; pueden aportar etiquetas fuente.
    pub roots_truncated: usize,
    /// Indica que el límite total o por raíz dejó candidatos fuera.
    pub truncated: bool,
    /// Eventos DNS ignorados por etiquetas PTR o excesos numéricos.
    pub numeric_sources_skipped: usize,
}

#[derive(Default)]
struct CandidateAccumulator {
    reasons: BTreeSet<DnsMutationReason>,
    root_source_id: Option<String>,
    mutation_source_id: Option<String>,
    static_wordlist: bool,
}

#[derive(Default)]
struct RootObservations {
    event_ids: BTreeSet<String>,
    labels: BTreeMap<String, BTreeSet<String>>,
}

struct ProposalEvidence<'a> {
    root_source_id: Option<&'a str>,
    mutation_source_id: &'a str,
}

/// Deriva propuestas acotadas usando exclusivamente eventos `DNS_NAME` ya
/// observados de un único lote/proyecto. El caller no debe mezclar proyectos:
/// sus vocabularios se combinan para generar candidatos entre raíces ICANN.
/// Nunca ejecuta los candidatos ni crea eventos DNS observados.
pub fn generate_dns_mutation_candidates(
    events: &[BbotEvent],
    options: DnsMutationCandidateOptions,
) -> ChefResult<DnsMutationCandidateBatch> {
    if events.len() > MAX_INPUT_EVENTS {
        return Err(crate::ChefError::new(
            crate::ErrorCode::LimitExceeded,
            "La entrada de eventos DNS excede el límite CHEF",
        ));
    }

    // Raíz registrable ICANN -> evidencia y primera etiqueta -> IDs.
    // observed evita proponer de nuevo activos ya importados.
    let mut roots: BTreeMap<String, RootObservations> = BTreeMap::new();
    let mut observed = BTreeSet::new();
    let mut numeric_sources_skipped = 0;

    for event in events.iter().filter(|event| event.event_type == "DNS_NAME") {
        let value = event.host.as_deref().or_else(|| event.data.as_str());
        let Some(host) = value.and_then(crate::host::canonical_domain) else {
            continue;
        };
        let Some(root) = crate::host::domain_parent_candidates(&host, false)
            .last()
            .cloned()
        else {
            continue;
        };
        observed.insert(host.clone());
        let source_id = if event.id.is_empty() {
            crate::event_engine::event_id(event)?
        } else {
            event.id.clone()
        };
        let root_observations = roots.entry(root.clone()).or_default();
        root_observations.event_ids.insert(source_id.clone());
        if host == root {
            continue;
        }
        let Some(label) = host
            .strip_suffix(&format!(".{root}"))
            .and_then(|prefix| prefix.split('.').next())
        else {
            continue;
        };
        if event_is_ptr(event) || has_excessive_digits(label) {
            numeric_sources_skipped += 1;
            continue;
        }
        root_observations
            .labels
            .entry(label.to_owned())
            .or_default()
            .insert(source_id);
    }

    let mut ranked_roots = roots.keys().cloned().collect::<Vec<_>>();
    ranked_roots.sort_by(|left, right| {
        let left_count = roots[left].labels.len();
        let right_count = roots[right].labels.len();
        right_count.cmp(&left_count).then_with(|| left.cmp(right))
    });

    let eligible_roots = ranked_roots
        .iter()
        .take(MAX_ROOTS)
        .cloned()
        .collect::<Vec<_>>();

    let mut all_candidates: BTreeMap<(String, String), CandidateAccumulator> = BTreeMap::new();
    let mut duplicates_suppressed = 0;
    let mut truncated = false;

    for root in &eligible_roots {
        let root_sources = &roots[root];
        let Some(root_source_id) = root_sources.event_ids.iter().next() else {
            continue;
        };
        let mut proposals: BTreeMap<String, CandidateAccumulator> = BTreeMap::new();

        // Cruza etiquetas/palabras de otras raíces de este mismo lote.
        for donor_root in &ranked_roots {
            if donor_root == root {
                continue;
            }
            for (label, source_ids) in &roots[donor_root].labels {
                if has_excessive_digits(label) {
                    continue;
                }
                let Some(donor_source_id) = source_ids.iter().next() else {
                    continue;
                };
                duplicates_suppressed += usize::from(add_proposal(
                    &mut proposals,
                    label,
                    root,
                    DnsMutationReason::CrossRootLabel,
                    ProposalEvidence {
                        root_source_id: Some(root_source_id),
                        mutation_source_id: donor_source_id,
                    },
                    MAX_PROPOSALS_PER_ROOT,
                    &mut truncated,
                ));
                for word in crate::dns_word_segmentation::extract_words(label) {
                    duplicates_suppressed += usize::from(add_proposal(
                        &mut proposals,
                        &word,
                        root,
                        DnsMutationReason::ExtractedWord,
                        ProposalEvidence {
                            root_source_id: Some(root_source_id),
                            mutation_source_id: donor_source_id,
                        },
                        MAX_PROPOSALS_PER_ROOT,
                        &mut truncated,
                    ));
                }
            }
        }

        // Adapta mutaciones locales de números, letras y modificadores.
        for (label, source_ids) in &root_sources.labels {
            let Some(source_id) = source_ids.iter().next() else {
                continue;
            };
            for (mutation, reasons) in crate::wordcloud::mutations(label) {
                for reason in reasons {
                    duplicates_suppressed += usize::from(add_proposal(
                        &mut proposals,
                        &mutation,
                        root,
                        reason,
                        ProposalEvidence {
                            root_source_id: None,
                            mutation_source_id: source_id,
                        },
                        MAX_PROPOSALS_PER_ROOT,
                        &mut truncated,
                    ));
                }
            }
        }

        // Propone etiquetas habituales desde el vocabulario local de DevOps;
        // una raíz apex importada basta como evidencia. No se consultan.
        for label in crate::wordcloud::common_dns_labels() {
            duplicates_suppressed += usize::from(add_proposal(
                &mut proposals,
                label,
                root,
                DnsMutationReason::StaticWordlist,
                ProposalEvidence {
                    root_source_id: Some(root_source_id),
                    mutation_source_id: root_source_id,
                },
                MAX_PROPOSALS_PER_ROOT,
                &mut truncated,
            ));
        }

        let per_root_limit = options.max_candidates_per_root.min(MAX_CANDIDATES_PER_ROOT);
        let mut ranked_proposals = proposals.into_iter().collect::<Vec<_>>();
        ranked_proposals.sort_by(|(left_name, left), (right_name, right)| {
            right
                .static_wordlist
                .cmp(&left.static_wordlist)
                .then_with(|| left_name.cmp(right_name))
        });
        let mut root_candidate_count = 0;
        for (name, accumulator) in ranked_proposals {
            if root_sources
                .labels
                .keys()
                .any(|label| format!("{label}.{root}") == name)
                || observed.contains(&name)
            {
                continue;
            }
            if all_candidates.len() >= options.max_candidates.min(MAX_CANDIDATES) {
                truncated = true;
                break;
            }
            if root_candidate_count >= per_root_limit {
                truncated = true;
                break;
            }
            let key = (root.clone(), name);
            if all_candidates.insert(key, accumulator).is_some() {
                duplicates_suppressed += 1;
            } else {
                root_candidate_count += 1;
            }
        }
    }

    let candidates = all_candidates
        .into_iter()
        .map(|((root, name), accumulator)| {
            let id = identity::id(RULE_ID, &[RULE_VERSION, &root, &name]);
            let source_event_ids = representative_source_ids(&accumulator);
            DnsMutationCandidate {
                id,
                name,
                root,
                rule_id: RULE_ID.to_owned(),
                rule_version: RULE_VERSION.to_owned(),
                reasons: accumulator.reasons.into_iter().collect(),
                source_event_ids,
            }
        })
        .collect::<Vec<_>>();

    let roots_truncated = ranked_roots.len().saturating_sub(MAX_ROOTS);
    truncated |= roots_truncated > 0;

    Ok(DnsMutationCandidateBatch {
        candidates,
        duplicates_suppressed,
        roots_truncated,
        truncated,
        numeric_sources_skipped,
    })
}

pub(crate) fn is_generated_dns_mutation_hint(event: &BbotEvent) -> bool {
    if event.event_type != "DNS_NAME_HINT"
        || event.module.as_deref() != Some("chef_dns_mutation_hints")
        || event.source_id.is_some()
        || event.source_uuid.is_some()
        || !event
            .tags
            .iter()
            .any(|tag| tag == "chef:derived-dns-mutation-hint")
    {
        return false;
    }

    let Some(root) = event
        .metadata
        .get("root")
        .and_then(serde_json::Value::as_str)
    else {
        return false;
    };
    let Some(name) = event
        .host
        .as_deref()
        .and_then(crate::host::canonical_domain)
    else {
        return false;
    };
    let Some(label) = name.strip_suffix(&format!(".{root}")) else {
        return false;
    };
    if label.is_empty()
        || crate::host::domain_parent_candidates(&name, false)
            .last()
            .map(String::as_str)
            != Some(root)
        || event.data.as_str() != Some(name.as_str())
    {
        return false;
    }

    let expected_candidate_id = identity::id(RULE_ID, &[RULE_VERSION, root, &name]);
    let expected_event_id = identity::id("chef-dns-mutation-event", &[&expected_candidate_id]);
    if event.id != expected_event_id
        || event
            .metadata
            .get("candidateId")
            .and_then(serde_json::Value::as_str)
            != Some(expected_candidate_id.as_str())
        || event
            .metadata
            .get("ruleId")
            .and_then(serde_json::Value::as_str)
            != Some(RULE_ID)
        || event
            .metadata
            .get("ruleVersion")
            .and_then(serde_json::Value::as_str)
            != Some(RULE_VERSION)
    {
        return false;
    }

    let Some(source_ids) = event
        .metadata
        .get("sourceEventIds")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    if source_ids.is_empty()
        || source_ids.len() > 2
        || source_ids
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect::<BTreeSet<_>>()
            .len()
            != source_ids.len()
        || source_ids
            .iter()
            .any(|source_id| source_id.as_str().is_none_or(str::is_empty))
        || event.parent_id.as_deref().is_none_or(|parent_id| {
            !source_ids
                .iter()
                .any(|source_id| source_id.as_str() == Some(parent_id))
        })
    {
        return false;
    }

    let Some(reasons) = event
        .metadata
        .get("reasons")
        .and_then(serde_json::Value::as_array)
    else {
        return false;
    };
    let reason_names = reasons
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    !reason_names.is_empty()
        && reason_names.len() == reasons.len()
        && reason_names.iter().all(|reason| {
            matches!(
                *reason,
                "cross_root_label"
                    | "extracted_word"
                    | "numeric_suffix"
                    | "number_adjustment"
                    | "wordcloud_modifier"
                    | "wordcloud_base"
                    | "static_wordlist"
            )
        })
        && reason_names.iter().all(|reason| {
            event
                .tags
                .iter()
                .any(|tag| tag == &format!("chef:dns-mutation:{reason}"))
        })
        && event.locator
            == format!(
                "rule:{RULE_ID}@{RULE_VERSION};reason={}",
                reason_names.join(",")
            )
}

fn add_proposal(
    proposals: &mut BTreeMap<String, CandidateAccumulator>,
    label: &str,
    root: &str,
    reason: DnsMutationReason,
    evidence: ProposalEvidence<'_>,
    max_proposals: usize,
    truncated: &mut bool,
) -> bool {
    let Some(name) = crate::host::canonical_domain(&format!("{label}.{root}")) else {
        return false;
    };
    let Some(candidate_label) = name.strip_suffix(&format!(".{root}")) else {
        return false;
    };
    if candidate_label.is_empty() {
        return false;
    }
    let duplicate = proposals.contains_key(&name);
    if !duplicate && proposals.len() >= max_proposals {
        *truncated = true;
        let Some((worst_name, worst)) =
            proposals
                .iter()
                .max_by(|(left_name, left), (right_name, right)| {
                    (!left.static_wordlist)
                        .cmp(&(!right.static_wordlist))
                        .then_with(|| left_name.cmp(right_name))
                })
        else {
            return false;
        };
        let candidate_priority = u8::from(reason != DnsMutationReason::StaticWordlist);
        let worst_priority = u8::from(!worst.static_wordlist);
        if (candidate_priority, name.as_str()) >= (worst_priority, worst_name.as_str()) {
            return false;
        }
        let worst_name = worst_name.clone();
        proposals.remove(&worst_name);
    }
    let accumulator = proposals.entry(name).or_default();
    accumulator.reasons.insert(reason);
    accumulator.static_wordlist |= reason == DnsMutationReason::StaticWordlist;
    if let Some(source_id) = evidence.root_source_id {
        retain_lexicographically_first(&mut accumulator.root_source_id, source_id);
    }
    retain_lexicographically_first(
        &mut accumulator.mutation_source_id,
        evidence.mutation_source_id,
    );
    duplicate
}

fn retain_lexicographically_first(slot: &mut Option<String>, source_id: &str) {
    if slot.as_deref().is_none_or(|existing| source_id < existing) {
        *slot = Some(source_id.to_owned());
    }
}

fn representative_source_ids(accumulator: &CandidateAccumulator) -> Vec<String> {
    let mut source_ids = BTreeSet::new();
    source_ids.extend(accumulator.root_source_id.iter().cloned());
    source_ids.extend(accumulator.mutation_source_id.iter().cloned());
    source_ids.into_iter().collect()
}

/// Replica acotada del patrón PTR de la fuente; no requiere regex ni red.
fn is_ptr_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() < 7 {
        return false;
    }
    (0..bytes.len()).any(|start| {
        let mut cursor = start;
        for part in 0..4 {
            let digit_start = cursor;
            while cursor < bytes.len() && bytes[cursor].is_ascii_digit() && cursor - digit_start < 4
            {
                cursor += 1;
            }
            let digits = cursor - digit_start;
            if digits == 0 || digits > 3 {
                return false;
            }
            if part < 3 {
                if cursor >= bytes.len() || !matches!(bytes[cursor], b'-' | b'_' | b'.') {
                    return false;
                }
                cursor += 1;
            }
        }
        true
    })
}

fn event_is_ptr(event: &BbotEvent) -> bool {
    event.tags.iter().any(|tag| {
        let tag = tag.to_ascii_lowercase();
        tag == "ptr" || tag == "dns:ptr" || tag.ends_with(":ptr")
    })
}

/// Excluye del entrenamiento de mutaciones las etiquetas con PTR o números
/// de alta cardinalidad; el evento observado continúa intacto en el grafo.
fn has_excessive_digits(value: &str) -> bool {
    if is_ptr_like(value) {
        return true;
    }
    let mut runs = 0;
    let mut run_length = 0;
    for character in value.chars() {
        if character.is_ascii_digit() {
            run_length += 1;
        } else if run_length > 0 {
            runs += 1;
            if runs > 2 || run_length > 3 {
                return true;
            }
            run_length = 0;
        }
    }
    run_length > 3 || runs + usize::from(run_length > 0) > 2
}
