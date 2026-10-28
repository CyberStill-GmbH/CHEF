//! Normalización offline de respuestas archivadas de OTX Passive DNS.
//!
//! El hostname y sus fechas pertenecen a registros pasivos históricos. No
//! prueban resolución actual ni se convierten aquí en observaciones CHEF.
//! Este módulo no consulta OTX ni maneja su API key.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_RECORDS: usize = 4_096;
const MAX_CANDIDATES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OtxPassiveDnsPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Extrae `passive_dns[].hostname` de una respuesta OTX archivada.
///
/// Los campos de dirección, fecha, ASN y metadatos no se importan. Los
/// hostnames inválidos o fuera de la raíz se omiten; JSON/estructura/límites
/// inválidos devuelven un error sin candidatos parciales.
pub fn parse_otx_passive_dns(query_domain: &str, response: &[u8]) -> ChefResult<OtxPassiveDnsPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OTX supera el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta OTX debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta OTX no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta OTX no contiene un objeto JSON",
    ))?;
    let Some(records) = object.get("passive_dns") else {
        return Ok(OtxPassiveDnsPage {
            query_domain,
            candidates: Vec::new(),
        });
    };
    let records = records.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta OTX contiene una lista passive_dns inválida",
    ))?;
    if records.len() > MAX_RECORDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OTX supera el límite CHEF de registros DNS",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for record in records {
        let record = record.as_object().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta OTX contiene un registro inválido",
        ))?;
        let Some(hostname) = record.get("hostname") else {
            continue;
        };
        let hostname = hostname.as_str().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta OTX contiene un hostname no textual",
        ))?;
        let hostname = hostname.trim_end_matches('.');
        if hostname.is_empty() || hostname.len() > 253 || !valid_dns_hostname(hostname) {
            continue;
        }
        let Some(hostname) = crate::host::canonical_domain(hostname) else {
            continue;
        };
        if hostname != query_domain && hostname.ends_with(&suffix) {
            candidates.insert(hostname);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta OTX supera el límite CHEF de candidatos",
                ));
            }
        }
    }

    Ok(OtxPassiveDnsPage {
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
