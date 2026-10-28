//! Candidatos de afinidad nominal entre dominios de eventos DNS importados.
//!
//! La regla es local y explicable: requiere raíces DNS de análisis explícitas,
//! compara dominios registrables ICANN y devuelve evidencia fuente. No infiere
//! propiedad, no fusiona activos y no autoriza consultas de red.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    BbotEventGraph, ChefError, ChefResult, ErrorCode, identity, validate_bbot_event_graph,
};

const MAX_ANALYSIS_ROOTS: usize = 64;
const MAX_CANDIDATES: usize = 512;
const MAX_EVIDENCE_PER_CANDIDATE: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DomainAffinityCandidate {
    /// ID estable para este par de raíz explícita y dominio candidato.
    pub id: String,
    pub analysis_root: String,
    pub candidate_domain: String,
    pub reason: &'static str,
    pub evidence: Vec<DomainAffinityEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DomainAffinityEvidence {
    pub event_id: String,
    pub event_type: &'static str,
    pub locator: String,
    pub observed_at: String,
}

/// Agrupa nombres `DNS_NAME` importados bajo raíces ICANN diferentes a las
/// raíces de análisis entregadas explícitamente por el llamador.
///
/// La afinidad es solo una sugerencia para revisión. `scope_distance` no se usa
/// como permiso, filtro de propiedad ni ponderación. Los eventos `URL`, pistas,
/// nombres no registrables y los dominios que comparten raíz con el análisis
/// quedan fuera. El resultado conserva IDs y locators para explicar cada par.
pub fn summarize_domain_affinity(
    graph: &BbotEventGraph,
    analysis_roots: &[String],
) -> ChefResult<Vec<DomainAffinityCandidate>> {
    validate_bbot_event_graph(graph)?;
    if analysis_roots.is_empty() || analysis_roots.len() > MAX_ANALYSIS_ROOTS {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La lista de raíces DNS de análisis está vacía o supera el máximo",
        ));
    }

    let mut roots = analysis_roots
        .iter()
        .map(|root| canonical_analysis_root(root))
        .collect::<ChefResult<Vec<_>>>()?;
    roots.sort();
    roots.dedup();
    if roots.is_empty() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "No hay raíces DNS de análisis válidas",
        ));
    }
    let root_set = roots
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();

    // BTreeMaps fijan el orden y deduplican solo la misma evidencia exacta
    // (mismo evento y locator), sin colapsar observaciones distintas.
    let mut grouped = BTreeMap::<
        (String, String),
        BTreeMap<(String, String), DomainAffinityEvidence>,
    >::new();
    for event in graph.events.iter().filter(|event| event.event_type == "DNS_NAME") {
        let Some(host) = event.host.as_deref().and_then(crate::host::canonical_domain) else {
            continue;
        };
        let Some(candidate_domain) = registrable_domain(&host) else {
            continue;
        };
        if root_set.contains(candidate_domain.as_str()) {
            continue;
        }

        for root in &roots {
            if root == &candidate_domain {
                continue;
            }
            let evidence = DomainAffinityEvidence {
                event_id: event.id.clone(),
                event_type: "DNS_NAME",
                locator: event.locator.clone(),
                observed_at: event.timestamp.clone(),
            };
            let pair = (root.clone(), candidate_domain.clone());
            if !grouped.contains_key(&pair) && grouped.len() == MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El resumen de afinidad de dominios supera los límites",
                ));
            }
            let candidate_evidence = grouped.entry(pair).or_default();
            let evidence_key = (evidence.event_id.clone(), evidence.locator.clone());
            if !candidate_evidence.contains_key(&evidence_key)
                && candidate_evidence.len() == MAX_EVIDENCE_PER_CANDIDATE
            {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El resumen de afinidad de dominios supera los límites",
                ));
            }
            candidate_evidence.entry(evidence_key).or_insert(evidence);
        }
    }

    Ok(grouped
        .into_iter()
        .map(
            |((analysis_root, candidate_domain), evidence)| DomainAffinityCandidate {
                id: identity::id(
                    "domain-affinity",
                    &[analysis_root.as_str(), candidate_domain.as_str()],
                ),
                analysis_root,
                candidate_domain,
                reason: "dns-name-importado-en-dominio-registrable-distinto; afinidad-nominal-sin-atribucion-de-propiedad",
                evidence: evidence.into_values().collect(),
            },
        )
        .collect())
}

fn canonical_analysis_root(value: &str) -> ChefResult<String> {
    let Some(root) = crate::host::canonical_domain(value) else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Una raíz DNS de análisis no es un dominio canónico válido",
        ));
    };
    if !crate::host::is_registrable_domain(&root) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Cada raíz DNS de análisis debe ser un dominio registrable ICANN explícito",
        ));
    }
    Ok(root)
}

fn registrable_domain(host: &str) -> Option<String> {
    if crate::host::is_registrable_domain(host) {
        return Some(host.to_owned());
    }
    crate::host::domain_parent_candidates(host, true)
        .into_iter()
        .last()
}
