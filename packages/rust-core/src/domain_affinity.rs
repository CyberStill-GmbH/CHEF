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
const RULE_ID: &str = "domain-affinity.external-registrable-domain";
const RULE_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DomainAffinityCandidate {
    /// ID estable para contexto, regla y dominio candidato.
    pub id: String,
    /// Contexto global seleccionado para el análisis; no atribuye cada
    /// observación a una raíz particular.
    pub analysis_roots: Vec<String>,
    pub candidate_domain: String,
    pub rule_id: &'static str,
    pub rule_version: &'static str,
    pub reason: &'static str,
    /// Heurística de priorización heredada del reporte fuente. No es riesgo,
    /// probabilidad de propiedad ni confirmación de relación.
    pub score: f64,
    pub evidence: Vec<DomainAffinityEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DomainAffinityEvidence {
    pub event_id: String,
    pub event_type: &'static str,
    pub locator: String,
    pub observed_at: String,
    pub scope_distance: u16,
    pub affiliate_tagged: bool,
}

/// Agrupa dominios registrables ICANN externos observados en la misma ejecución
/// que las raíces explícitas entregadas por el llamador.
///
/// La distancia de scope solo sirve como señal del reporte de origen y no como
/// permiso de consulta. La etiqueta `affiliate` suma prioridad heurística; no
/// prueba propiedad ni vínculo. Los eventos `URL`, pistas, nombres no
/// registrables y dominios dentro de las raíces quedan fuera. El resultado
/// conserva IDs y locators para explicar cada candidato.
pub fn summarize_domain_affinity(
    graph: &BbotEventGraph,
    analysis_roots: &[String],
) -> ChefResult<Vec<DomainAffinityCandidate>> {
    validate_bbot_event_graph(graph)?;
    let roots = canonicalize_domain_analysis_roots(analysis_roots)?;

    let root_set = roots.iter().map(String::as_str).collect::<BTreeSet<_>>();

    // La solicitud constituye el contexto de la ejecución; no fabricamos una
    // relación candidato-raíz cuando el grafo no la contiene.
    // BTreeMaps fijan el orden y deduplican solo la misma evidencia exacta.
    let mut grouped = BTreeMap::<String, BTreeMap<(String, String), DomainAffinityEvidence>>::new();
    for event in graph
        .events
        .iter()
        .filter(|event| event.event_type == "DNS_NAME")
    {
        if event.scope_distance == 0 {
            continue;
        }
        let Some(host) = event
            .host
            .as_deref()
            .and_then(crate::host::canonical_domain)
        else {
            continue;
        };
        let Some(candidate_domain) = registrable_domain(&host) else {
            continue;
        };
        if root_set.contains(candidate_domain.as_str()) {
            continue;
        }

        let evidence = DomainAffinityEvidence {
            event_id: event.id.clone(),
            event_type: "DNS_NAME",
            locator: event.locator.clone(),
            observed_at: event.timestamp.clone(),
            scope_distance: event.scope_distance,
            affiliate_tagged: event.tags.iter().any(|tag| tag == "affiliate"),
        };
        let candidate_evidence = grouped.entry(candidate_domain.clone()).or_default();
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

    if grouped.len() > MAX_CANDIDATES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El resumen de afinidad de dominios supera los límites",
        ));
    }
    let analysis_roots_key = roots.join("|");
    let mut candidates = grouped
        .into_iter()
        .map(|(candidate_domain, evidence)| {
            let evidence = evidence.into_values().collect::<Vec<_>>();
            let score = evidence
                .iter()
                .map(|item| {
                    1.0 / f64::from(item.scope_distance)
                        + if item.affiliate_tagged { 1.0 } else { 0.0 }
                })
                .sum();
            DomainAffinityCandidate {
                id: identity::id(
                    "domain-affinity",
                    &[
                        RULE_ID,
                        RULE_VERSION,
                        analysis_roots_key.as_str(),
                        candidate_domain.as_str(),
                    ],
                ),
                analysis_roots: roots.clone(),
                candidate_domain,
                rule_id: RULE_ID,
                rule_version: RULE_VERSION,
                reason: "Evento DNS_NAME fuera de las raíces analizadas; distancia y etiqueta affiliate solo priorizan la revisión, sin afirmar propiedad.",
                score,
                evidence,
            }
        })
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .score
            .total_cmp(&left.score)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(candidates)
}

/// Devuelve raíces canónicas registrables ICANN, ordenadas y sin duplicados.
/// Esta normalización se comparte con la respuesta IPC para que el contexto
/// mostrado sea el mismo usado por el cálculo.
pub fn canonicalize_domain_analysis_roots(analysis_roots: &[String]) -> ChefResult<Vec<String>> {
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
    Ok(roots)
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
