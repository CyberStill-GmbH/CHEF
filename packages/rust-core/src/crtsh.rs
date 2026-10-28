//! Normalización offline de respuestas JSON archivadas de crt.sh.
//!
//! Los SAN de certificados son candidatos de nombres publicados en
//! Certificate Transparency; no acreditan resolución, disponibilidad ni
//! control actual del dominio. Este módulo no consulta crt.sh.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CERTIFICATES: usize = 2_048;
const MAX_CANDIDATES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrtShSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Valida y extrae candidatos de la raíz consultada desde un cuerpo crt.sh.
///
/// Los registros sin ID o sin `name_value` se ignoran como en el adaptador
/// BBOT fuente. La respuesta completa debe ser JSON válido y estar dentro de
/// límites; se deduplican certificados y nombres con orden determinista.
pub fn parse_crtsh_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<CrtShSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta crt.sh supera el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta crt.sh debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta crt.sh no contiene JSON válido",
        )
    })?;
    let entries = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta crt.sh no contiene una lista de certificados",
    ))?;
    if entries.len() > MAX_CERTIFICATES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta crt.sh supera el límite CHEF de certificados",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut certificate_ids = BTreeSet::new();
    let mut candidates = BTreeSet::new();
    for entry in entries {
        let Some(certificate_id) = entry.get("id").and_then(certificate_id) else {
            continue;
        };
        if !certificate_ids.insert(certificate_id) {
            continue;
        }
        let Some(names) = entry.get("name_value").and_then(Value::as_str) else {
            continue;
        };
        for name in names.lines().map(str::trim).filter(|name| !name.is_empty()) {
            if name.len() > 253 || !valid_dns_hostname(name) {
                continue;
            }
            let Some(name) = crate::host::canonical_domain(name) else {
                continue;
            };
            if name != query_domain && name.ends_with(&suffix) {
                candidates.insert(name);
                if candidates.len() > MAX_CANDIDATES {
                    return Err(ChefError::new(
                        ErrorCode::LimitExceeded,
                        "La respuesta crt.sh supera el límite CHEF de subdominios",
                    ));
                }
            }
        }
    }

    Ok(CrtShSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

fn certificate_id(value: &Value) -> Option<String> {
    match value {
        Value::String(value) if !value.is_empty() => Some(value.clone()),
        Value::Number(value) if value.as_u64().is_some_and(|id| id != 0) => Some(value.to_string()),
        _ => None,
    }
}

fn valid_dns_hostname(value: &str) -> bool {
    let value = value.strip_suffix('.').unwrap_or(value);
    value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}
