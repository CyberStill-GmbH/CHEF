//! Proyección offline de mutaciones DNS sobre evidencia importada.

use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BbotEventGraph, ChefResult, DnsMutationCandidate, DnsWordCloud,
    generate_dns_mutation_candidates_with_wordcloud,
};

const MAX_HINTS: usize = 10_000;

/// Candidatos DNS ligados a sus eventos semilla, separados del grafo observado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMutationHintBatch {
    pub candidates: Vec<DnsMutationCandidate>,
    pub roots_processed: usize,
    pub events_skipped: usize,
    pub observed_candidates_suppressed: usize,
    pub truncated: bool,
}

/// Genera sugerencias a partir de `DNS_NAME` que tienen `scope_distance == 0`.
/// No ejecuta módulos, no consulta DNS y no modifica el grafo de entrada.
pub fn generate_dns_mutation_hints(
    graph: &BbotEventGraph,
    max_candidates: usize,
) -> ChefResult<DnsMutationHintBatch> {
    crate::event_engine::validate_event_graph(graph)?;
    let limit = max_candidates.min(MAX_HINTS);
    let mut groups: BTreeMap<String, BTreeMap<String, BTreeSet<String>>> = BTreeMap::new();
    let mut observed = BTreeSet::new();
    let mut events_skipped = 0;

    for event in &graph.events {
        if event.event_type != "DNS_NAME" {
            continue;
        }
        let Some(host) = event
            .host
            .as_deref()
            .and_then(crate::host::canonical_domain)
        else {
            events_skipped += 1;
            continue;
        };
        observed.insert(host.clone());
        if event.scope_distance != 0 {
            continue;
        }
        let Some((label, root)) = host.split_once('.') else {
            events_skipped += 1;
            continue;
        };
        let label = label.to_owned();
        let root = root.to_owned();
        if !crate::host::domain_parent_candidates(&host, false)
            .iter()
            .any(|parent| parent == &root)
        {
            events_skipped += 1;
            continue;
        }
        groups
            .entry(root)
            .or_default()
            .entry(label)
            .or_default()
            .insert(event.id.clone());
    }

    let mut candidates_by_name: BTreeMap<String, DnsMutationCandidate> = BTreeMap::new();
    let mut roots_processed = 0;
    let mut observed_candidates_suppressed = 0;
    let mut truncated = false;

    for (root, seeds) in groups {
        if candidates_by_name.len() >= limit {
            truncated = true;
            break;
        }
        roots_processed += 1;
        let mut cloud = DnsWordCloud::default();
        let mut labels = Vec::with_capacity(seeds.len());
        for label in seeds.keys() {
            cloud.absorb_text(label, true);
            labels.push(label.clone());
        }
        let remaining = limit.saturating_sub(candidates_by_name.len());
        let batch =
            generate_dns_mutation_candidates_with_wordcloud(&root, &labels, &cloud, 64, remaining)?;
        truncated |= batch.truncated;
        for mut candidate in batch.candidates {
            if observed.contains(&candidate.name) {
                observed_candidates_suppressed += 1;
                continue;
            }
            candidate.source_event_ids = seeds
                .get(&candidate.seed_label)
                .into_iter()
                .flat_map(|ids| ids.iter().cloned())
                .collect();
            if let Some(existing) = candidates_by_name.get_mut(&candidate.name) {
                existing.source_event_ids.extend(candidate.source_event_ids);
                existing.source_event_ids.sort();
                existing.source_event_ids.dedup();
                continue;
            }
            if candidates_by_name.len() == limit {
                truncated = true;
                break;
            }
            candidates_by_name.insert(candidate.name.clone(), candidate);
        }
    }

    let candidates = candidates_by_name.into_values().collect();
    Ok(DnsMutationHintBatch {
        candidates,
        roots_processed,
        events_skipped,
        observed_candidates_suppressed,
        truncated,
    })
}
