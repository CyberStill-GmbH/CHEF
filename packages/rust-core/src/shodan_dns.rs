//! Normalización offline de páginas archivadas de Shodan DNS.
//!
//! La API devuelve prefijos y el módulo fuente los combina con el dominio
//! consultado. Este parser no pagina, consulta Shodan ni maneja API keys.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_PREFIXES: usize = 2_048;
const MAX_CANDIDATES: usize = 1_024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShodanDnsSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Combina `subdomains[]` con la raíz consultada y valida el hostname final.
///
/// Cada respuesta archivada corresponde a una página; el número de página y
/// la continuación remota pertenecen al transporte y no se siguen aquí.
pub fn parse_shodan_dns_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ShodanDnsSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La página Shodan DNS supera el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Shodan DNS debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La página Shodan DNS no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La página Shodan DNS no contiene un objeto JSON",
    ))?;
    let Some(prefixes) = object.get("subdomains") else {
        return Ok(ShodanDnsSubdomainPage {
            query_domain,
            candidates: Vec::new(),
        });
    };
    let prefixes = prefixes.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La página Shodan DNS contiene una lista subdomains inválida",
    ))?;
    if prefixes.len() > MAX_PREFIXES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La página Shodan DNS supera el límite CHEF de prefijos",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for prefix in prefixes {
        let Some(prefix) = prefix.as_str() else {
            continue;
        };
        if prefix.is_empty() || prefix.len() > 253 || !valid_dns_hostname(prefix) {
            continue;
        }
        let Some(hostname) = crate::host::canonical_domain(&format!("{prefix}{suffix}")) else {
            continue;
        };
        if hostname != query_domain && hostname.ends_with(&suffix) {
            candidates.insert(hostname);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La página Shodan DNS supera el límite CHEF de candidatos",
                ));
            }
        }
    }

    Ok(ShodanDnsSubdomainPage {
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
