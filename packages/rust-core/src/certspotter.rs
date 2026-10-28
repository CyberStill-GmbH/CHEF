//! Normalización offline de respuestas JSON archivadas de CertSpotter.
//!
//! Los nombres de certificados son candidatos publicados en Certificate
//! Transparency; no prueban resolución DNS, disponibilidad ni control actual.
//! Este módulo no consulta CertSpotter ni utiliza claves de API.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_ISSUANCES: usize = 2_048;
const MAX_DNS_NAMES: usize = 16_384;
const MAX_CANDIDATES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertSpotterSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Extrae candidatos DNS de `dns_names` sin seguir el cursor remoto `after`.
///
/// Las entradas sin `dns_names` se omiten, como en el adaptador de origen.
/// El cuerpo completo debe ser JSON válido y todos los límites se comprueban
/// antes de ampliar el resultado; un error no entrega candidatos parciales.
pub fn parse_certspotter_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<CertSpotterSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta CertSpotter supera el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta CertSpotter debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta CertSpotter no contiene JSON válido",
        )
    })?;
    let entries = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta CertSpotter no contiene una lista de emisiones",
    ))?;
    if entries.len() > MAX_ISSUANCES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta CertSpotter supera el límite de emisiones",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut name_count = 0usize;
    let mut candidates = BTreeSet::new();
    for entry in entries {
        let entry = entry.as_object().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta CertSpotter contiene una emisión inválida",
        ))?;
        let Some(names) = entry.get("dns_names") else {
            continue;
        };
        let names = names.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Una emisión CertSpotter contiene una lista dns_names inválida",
        ))?;
        name_count = name_count.saturating_add(names.len());
        if name_count > MAX_DNS_NAMES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta CertSpotter supera el límite CHEF de nombres DNS",
            ));
        }

        for raw_name in names {
            let raw_name = raw_name.as_str().ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "Una emisión CertSpotter contiene un nombre DNS inválido",
            ))?;
            let name = raw_name
                .strip_prefix("*.")
                .unwrap_or(raw_name)
                .trim_end_matches('.');
            if name.is_empty() || name.len() > 253 || !valid_dns_hostname(name) {
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
                        "La respuesta CertSpotter supera el límite CHEF de candidatos",
                    ));
                }
            }
        }
    }

    Ok(CertSpotterSubdomainPage {
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
