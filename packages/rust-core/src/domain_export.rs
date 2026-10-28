//! Exportación CHEF de nombres DNS observados en evidencia importada.
//! No infiere alcance, resolución actual ni propiedad del dominio.

use serde::Serialize;

use crate::{
    ChefError, ChefReconGraph, ChefResult, ErrorCode, ReconAssetKind, validate_chef_recon_graph,
};

const MAX_DOMAIN_EXPORT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefObservedDomainExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub scope_status: &'static str,
    pub domains: Vec<ChefObservedDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefObservedDomain {
    pub asset_id: String,
    pub name: String,
    pub qualification: &'static str,
    pub evidence: Vec<ChefObservedDomainEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefObservedDomainEvidence {
    pub id: String,
    pub source_event_id: String,
    pub event_type: &'static str,
    pub observed_at: String,
}

/// Construye un JSON CHEF ordenado con dominios sustentados por eventos
/// `DNS_NAME` directos. Excluye `DNS_NAME_UNRESOLVED` y pistas derivadas.
pub fn export_observed_domain_list(
    run_id: &str,
    graph: &ChefReconGraph,
) -> ChefResult<ChefObservedDomainExport> {
    if !run_id.starts_with("run:")
        || run_id.len() > 128
        || run_id.len() == "run:".len()
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El identificador de importación CHEF no es válido",
        ));
    }
    validate_chef_recon_graph(graph)?;

    let direct_dns_evidence = graph
        .evidence
        .iter()
        .filter(|evidence| evidence.event_type == "DNS_NAME")
        .map(|evidence| {
            (
                evidence.id.as_str(),
                ChefObservedDomainEvidence {
                    id: evidence.id.clone(),
                    source_event_id: evidence.source_event_id.clone(),
                    event_type: "DNS_NAME",
                    observed_at: evidence.observed_at.clone(),
                },
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut domains = graph
        .assets
        .iter()
        .filter(|asset| asset.kind == ReconAssetKind::Domain && asset.qualification.is_none())
        .filter_map(|asset| {
            let mut evidence = asset
                .evidence_ids
                .iter()
                .filter_map(|id| direct_dns_evidence.get(id.as_str()).cloned())
                .collect::<Vec<_>>();
            if evidence.is_empty() {
                return None;
            }
            evidence.sort_by(|left, right| {
                (&left.observed_at, &left.source_event_id)
                    .cmp(&(&right.observed_at, &right.source_event_id))
            });
            Some(ChefObservedDomain {
                asset_id: asset.id.clone(),
                name: asset.key.clone(),
                qualification: "observed",
                evidence,
            })
        })
        .collect::<Vec<_>>();
    domains.sort_by(|left, right| left.name.cmp(&right.name));

    let export = ChefObservedDomainExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        scope_status: "not_asserted",
        domains,
    };
    let serialized = serde_json::to_vec(&export)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La exportación CHEF no es válida"))?;
    if serialized.len() > MAX_DOMAIN_EXPORT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La exportación CHEF de dominios supera el máximo de bytes",
        ));
    }
    Ok(export)
}
