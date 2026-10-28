//! Generación local de candidatos DNS derivados de nombres observados.
//!
//! Este módulo no resuelve nombres, no accede a la red ni transforma candidatos
//! en observaciones. Cada salida conserva los IDs de eventos usados como
//! evidencia para la regla CHEF determinista.

use std::collections::{BTreeMap, BTreeSet};

use crate::{BbotEvent, ChefResult, identity};

const RULE_ID: &str = "dns.cross_root_mutation";
const RULE_VERSION: &str = "1";
const MAX_INPUT_EVENTS: usize = 10_000;
const MAX_ROOTS: usize = 50;
const MAX_CANDIDATES: usize = 10_000;
const MAX_CANDIDATES_PER_ROOT: usize = 100;

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

    // raíz registrable ICANN -> primera etiqueta -> IDs; observed evita repetir activos.
    let mut roots: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut observed = BTreeSet::new();
    let mut numeric_sources_skipped = 0;

    for event in events.iter().filter(|event| event.event_type == "DNS_NAME") {
        let value = event.host.as_deref().or_else(|| event.data.as_str());
        let Some(host) = value.and_then(crate::host::canonical_domain) else {
            continue;
        };
        if !crate::host::is_subdomain(&host) {
            continue;
        }
        let Some((label, _)) = host.split_once('.') else {
            continue;
        };
        let Some(root) = crate::host::domain_parent_candidates(&host, false)
            .last()
            .cloned()
        else {
            continue;
        };
        observed.insert(host.clone());
        if event_is_ptr(event) || has_excessive_digits(label) {
            numeric_sources_skipped += 1;
            continue;
        }

        let source_id = if event.id.is_empty() {
            crate::event_engine::event_id(event)?
        } else {
            event.id.clone()
        };
        roots
            .entry(root)
            .or_default()
            .entry(label.to_owned())
            .or_default()
            .insert(source_id);
    }

    let mut ranked_roots = roots.keys().cloned().collect::<Vec<_>>();
    ranked_roots.sort_by(|left, right| {
        let left_count = roots[left].len();
        let right_count = roots[right].len();
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
        let Some(root_source_id) = root_sources.values().flat_map(BTreeSet::iter).min() else {
            continue;
        };
        let mut proposals: BTreeMap<String, CandidateAccumulator> = BTreeMap::new();

        // Port offline del cruce de etiquetas de otras raíces.
        for donor_root in &ranked_roots {
            if donor_root == root {
                continue;
            }
            for (label, source_ids) in &roots[donor_root] {
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
                    Some(root_source_id),
                    donor_source_id,
                ));
                for word in extract_words(label) {
                    duplicates_suppressed += usize::from(add_proposal(
                        &mut proposals,
                        &word,
                        root,
                        DnsMutationReason::ExtractedWord,
                        Some(root_source_id),
                        donor_source_id,
                    ));
                }
            }
        }

        // Mutación local limitada: sufijos 1–3 de etiquetas de la raíz.
        for (label, source_ids) in root_sources {
            let Some(source_id) = source_ids.iter().next() else {
                continue;
            };
            for word in extract_words(label) {
                for suffix in 1..=3 {
                    let mutation = format!("{word}{suffix}");
                    duplicates_suppressed += usize::from(add_proposal(
                        &mut proposals,
                        &mutation,
                        root,
                        DnsMutationReason::NumericSuffix,
                        None,
                        source_id,
                    ));
                }
            }
        }

        let per_root_limit = options.max_candidates_per_root.min(MAX_CANDIDATES_PER_ROOT);
        for (name, accumulator) in proposals {
            if root_sources
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
            if all_candidates
                .keys()
                .filter(|(candidate_root, _)| candidate_root == root)
                .count()
                >= per_root_limit
            {
                truncated = true;
                break;
            }
            let key = (root.clone(), name);
            if all_candidates.insert(key, accumulator).is_some() {
                duplicates_suppressed += 1;
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
        || label.contains('.')
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
                "cross_root_label" | "extracted_word" | "numeric_suffix"
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
    root_source_id: Option<&str>,
    mutation_source_id: &str,
) -> bool {
    let Some(name) = crate::host::canonical_domain(&format!("{label}.{root}")) else {
        return false;
    };
    let Some(candidate_label) = name.strip_suffix(&format!(".{root}")) else {
        return false;
    };
    if candidate_label.is_empty() || candidate_label.contains('.') {
        return false;
    }
    let duplicate = proposals.contains_key(&name);
    let accumulator = proposals.entry(name).or_default();
    accumulator.reasons.insert(reason);
    if let Some(source_id) = root_source_id {
        retain_lexicographically_first(&mut accumulator.root_source_id, source_id);
    }
    retain_lexicographically_first(&mut accumulator.mutation_source_id, mutation_source_id);
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

fn extract_words(label: &str) -> BTreeSet<String> {
    label
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| word.len() >= 2 && word.len() <= 63)
        .map(str::to_ascii_lowercase)
        .collect()
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
