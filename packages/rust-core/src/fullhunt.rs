//! Normalización offline de respuestas JSON archivadas de FullHunt.
//!
//! Los resultados del proveedor son candidatos históricos. Este parser no
//! consulta la API, no usa credenciales y no afirma resolución actual.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_HOSTS: usize = 3_000;
const MAX_CANDIDATES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullHuntSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Extrae los hosts `hosts[]` de FullHunt que descienden de la raíz consultada.
///
/// El JSON archivado se rechaza de forma atómica si su estructura o sus
/// límites no son válidos; nombres inválidos o fuera de raíz se omiten.
pub fn parse_fullhunt_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<FullHuntSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta FullHunt supera el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta FullHunt debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta FullHunt no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta FullHunt no contiene un objeto JSON",
    ))?;
    let Some(hosts) = object.get("hosts") else {
        return Ok(FullHuntSubdomainPage {
            query_domain,
            candidates: Vec::new(),
        });
    };
    let hosts = hosts.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta FullHunt contiene una lista hosts inválida",
    ))?;
    if hosts.len() > MAX_HOSTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta FullHunt supera el límite CHEF de hosts",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for host in hosts {
        let Some(host) = host.as_str() else {
            continue;
        };
        let host = host.trim_end_matches('.');
        if host.is_empty() || host.len() > 253 || !valid_dns_hostname(host) {
            continue;
        }
        let Some(host) = crate::host::canonical_domain(host) else {
            continue;
        };
        if host != query_domain && host.ends_with(&suffix) {
            candidates.insert(host);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta FullHunt supera el límite CHEF de candidatos",
                ));
            }
        }
    }

    Ok(FullHuntSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

fn valid_dns_hostname(value: &str) -> bool {
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
