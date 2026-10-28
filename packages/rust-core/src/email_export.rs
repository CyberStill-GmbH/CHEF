//! Exportación local y explícita de direcciones de correo observadas.
//!
//! Solo exporta eventos existentes de tipo `EMAIL_ADDRESS`; no deriva correo desde
//! usuarios ni afirma que la observación esté dentro del scope CHEF.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    BbotEventGraph, ChefError, ChefReconGraph, ChefResult, ErrorCode, validate_bbot_event_graph,
    validate_chef_recon_graph,
};

const MAX_EMAILS: usize = 10_000;
const MAX_EXPORT_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefObservedEmailExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub file_name: String,
    pub content_type: &'static str,
    pub scope_status: &'static str,
    pub email_count: usize,
    pub emails: Vec<ChefObservedEmail>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefObservedEmail {
    pub address: String,
    pub evidence: Vec<ChefEmailEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefEmailEvidence {
    pub id: String,
    pub parent_evidence_id: Option<String>,
    pub observed_at: String,
}

/// Construye una exportación JSON deduplicada desde eventos directos y su
/// proyección de evidencia CHEF. La escritura/descarga se realiza en Tauri/UI.
pub fn export_observed_emails(
    run_id: &str,
    event_graph: &BbotEventGraph,
    recon_graph: &ChefReconGraph,
) -> ChefResult<ChefObservedEmailExport> {
    validate_run_id(run_id)?;
    validate_bbot_event_graph(event_graph)?;
    validate_chef_recon_graph(recon_graph)?;

    let evidence_by_source = recon_graph
        .evidence
        .iter()
        .map(|item| (item.source_event_id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut emails = BTreeMap::<String, BTreeSet<ChefEmailEvidence>>::new();
    for event in event_graph
        .events
        .iter()
        .filter(|event| event.event_type == "EMAIL_ADDRESS")
    {
        let Some(address) = event.data.as_str().map(str::trim) else {
            continue;
        };
        if !valid_address(address) {
            continue;
        }
        let source = evidence_by_source.get(event.id.as_str()).ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "Falta evidencia CHEF para un evento de correo",
            )
        })?;
        emails
            .entry(address.to_owned())
            .or_default()
            .insert(ChefEmailEvidence {
                id: source.id.clone(),
                parent_evidence_id: source.parent_evidence_id.clone(),
                observed_at: source.observed_at.clone(),
            });
        if emails.len() > MAX_EMAILS {
            return Err(limit_error());
        }
    }

    let email_count = emails.len();
    let emails = emails
        .into_iter()
        .map(|(address, evidence)| ChefObservedEmail {
            address,
            evidence: evidence.into_iter().collect(),
        })
        .collect::<Vec<_>>();
    let export = ChefObservedEmailExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        file_name: format!("chef-correos-observados-{}.json", run_id.replace(':', "-")),
        content_type: "application/json; charset=utf-8",
        scope_status: "not_asserted",
        email_count,
        emails,
    };
    let serialized = serde_json::to_vec(&export)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "No se pudo crear la exportación"))?;
    if serialized.len() > MAX_EXPORT_BYTES {
        return Err(limit_error());
    }
    Ok(export)
}

pub(crate) fn valid_address(address: &str) -> bool {
    if address.is_empty()
        || address.len() > 254
        || address.trim() != address
        || address.chars().any(char::is_control)
        || address.chars().any(char::is_whitespace)
    {
        return false;
    }
    let Some((local, domain)) = address.split_once('@') else {
        return false;
    };
    !local.is_empty() && !domain.is_empty() && !domain.contains('@') && local.len() <= 64
}

fn validate_run_id(run_id: &str) -> ChefResult<()> {
    if !run_id.starts_with("run:")
        || run_id.len() == "run:".len()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El identificador de importación CHEF no es válido",
        ));
    }
    Ok(())
}

fn limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "La exportación de correos CHEF supera el máximo de registros o bytes",
    )
}
