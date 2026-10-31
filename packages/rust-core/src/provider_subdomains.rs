//! Extrae subdominios candidatos de respuestas pasivas ya guardadas.
//!
//! Adapta localmente la extracción de nombres compartida por BBOT. No obtiene
//! tokens, no hace solicitudes HTTP y no convierte resultados en DNS actual.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_HOST_TOKENS: usize = 16_384;
const MAX_CANDIDATES: usize = 512;
const MAX_JSON_ENTRIES: usize = 2_048;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

pub type DnsDumpsterSubdomainPage = ProviderSubdomainPage;
pub type RapidDnsSubdomainPage = ProviderSubdomainPage;
pub type HackerTargetSubdomainPage = ProviderSubdomainPage;
pub type SecurityTrailsSubdomainPage = ProviderSubdomainPage;
pub type SubdomainCenterPage = ProviderSubdomainPage;

/// Adapta offline la respuesta JSON `subdomains[]` de SecurityTrails.
///
/// El archivo debe haber sido obtenido con autorización por una capa externa.
/// CHEF solo consume prefijos archivados, los combina con la raíz consultada
/// y devuelve candidatos históricos; no gestiona API keys, HTTP ni DNS.
pub fn parse_securitytrails_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<SecurityTrailsSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta SecurityTrails supera el límite de bytes CHEF",
        ));
    }
    let query_domain = canonical_query_domain(query_domain)?;
    let value: serde_json::Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta SecurityTrails no contiene JSON válido",
        )
    })?;
    let prefixes = value
        .as_object()
        .and_then(|object| object.get("subdomains"))
        .and_then(serde_json::Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta SecurityTrails no contiene una lista subdomains válida",
        ))?;
    if prefixes.len() > MAX_JSON_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta SecurityTrails supera el límite CHEF de prefijos",
        ));
    }

    let mut candidates = BTreeSet::new();
    for prefix in prefixes.iter().filter_map(serde_json::Value::as_str) {
        let prefix = prefix.trim().trim_end_matches('.');
        if prefix.is_empty()
            || prefix.contains('*')
            || prefix.len() > 253
            || !valid_ascii_hostname(prefix)
        {
            continue;
        }
        let Some(candidate) = crate::host::canonical_domain(&format!("{prefix}.{query_domain}"))
        else {
            continue;
        };
        if candidate != query_domain && candidate.ends_with(&format!(".{query_domain}")) {
            candidates.insert(candidate);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta SecurityTrails supera el límite CHEF de subdominios",
                ));
            }
        }
    }
    Ok(ProviderSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

/// Adapta offline la lista JSON de nombres de Subdomain Center.
///
/// Solo conserva FQDN que sean descendientes de la raíz consultada. Una
/// coincidencia no confirma resolución, disponibilidad ni control actual.
pub fn parse_subdomaincenter_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<SubdomainCenterPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Subdomain Center supera el límite de bytes CHEF",
        ));
    }
    let query_domain = canonical_query_domain(query_domain)?;
    let names: serde_json::Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Subdomain Center no contiene JSON válido",
        )
    })?;
    let names = names.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta Subdomain Center debe ser una lista JSON",
    ))?;
    if names.len() > MAX_JSON_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Subdomain Center supera el límite CHEF de nombres",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for name in names.iter().filter_map(serde_json::Value::as_str) {
        let name = name.trim().trim_end_matches('.');
        if name.contains('*') || name.len() > 253 || !valid_ascii_hostname(name) {
            continue;
        }
        let Some(candidate) = crate::host::canonical_domain(name) else {
            continue;
        };
        if candidate != query_domain && candidate.ends_with(&suffix) {
            candidates.insert(candidate);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta Subdomain Center supera el límite CHEF de subdominios",
                ));
            }
        }
    }
    Ok(ProviderSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

/// Adapta offline las filas `host,ip` del módulo HackerTarget de BBOT.
///
/// La columna IP se ignora: su presencia en una respuesta histórica no
/// demuestra que el DNS siga resolviendo ni crea una observación CHEF.
pub fn parse_hackertarget_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<HackerTargetSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta HackerTarget supera el límite de bytes CHEF",
        ));
    }
    let query_domain = canonical_query_domain(query_domain)?;
    let text = std::str::from_utf8(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta HackerTarget no contiene texto UTF-8 válido",
        )
    })?;

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for (line_index, line) in text.lines().enumerate() {
        if line_index >= MAX_HOST_TOKENS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta HackerTarget supera el límite CHEF de filas",
            ));
        }

        // BBOT toma el primer campo de cada fila CSV y valida el hostname.
        // CHEF conserva únicamente descendientes de la raíz consultada.
        let host = line.split(',').next().unwrap_or_default().trim();
        if host.is_empty() || host.contains('*') || host.len() > 253 {
            continue;
        }
        if !valid_ascii_hostname(host) {
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
                    "La respuesta HackerTarget supera el límite CHEF de subdominios",
                ));
            }
        }
    }

    Ok(ProviderSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

/// Adapta la extracción in-scope del módulo DNSDumpster de BBOT.
pub fn parse_dnsdumpster_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<DnsDumpsterSubdomainPage> {
    parse_provider_subdomains(query_domain, response)
}

/// Adapta la extracción in-scope del módulo RapidDNS de BBOT.
pub fn parse_rapiddns_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<RapidDnsSubdomainPage> {
    parse_provider_subdomains(query_domain, response)
}

/// Extrae nombres descendientes de la raíz desde el cuerpo textual archivado.
///
/// La respuesta se procesa como texto no confiable, con límite total, límite
/// de tokens y salida determinista. Una entrada malformada o fuera de los
/// límites se rechaza completa; no se publican resultados parciales.
pub fn parse_provider_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ProviderSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta del proveedor supera el límite de bytes CHEF",
        ));
    }
    let query_domain = canonical_query_domain(query_domain)?;
    let text = std::str::from_utf8(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta del proveedor no contiene texto UTF-8 válido",
        )
    })?;

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    let mut token_start = None;
    let mut token_count = 0;
    for (index, byte) in text.bytes().enumerate() {
        if is_host_token_byte(byte) {
            token_start.get_or_insert(index);
        } else if let Some(start) = token_start.take() {
            collect_candidate(
                &text[start..index],
                &suffix,
                &mut candidates,
                &mut token_count,
            )?;
        }
    }
    if let Some(start) = token_start {
        collect_candidate(&text[start..], &suffix, &mut candidates, &mut token_count)?;
    }

    Ok(ProviderSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

fn canonical_query_domain(query_domain: &str) -> ChefResult<String> {
    crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta debe ser una raíz registrable ICANN",
        ))
}

fn is_host_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')
}

fn valid_ascii_hostname(value: &str) -> bool {
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

fn collect_candidate(
    token: &str,
    suffix: &str,
    candidates: &mut BTreeSet<String>,
    token_count: &mut usize,
) -> ChefResult<()> {
    if !token.contains('.') {
        return Ok(());
    }
    *token_count += 1;
    if *token_count > MAX_HOST_TOKENS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta del proveedor supera el límite CHEF de nombres examinados",
        ));
    }

    let candidate = token.strip_suffix('.').unwrap_or(token);
    if candidate.len() > 253 {
        return Ok(());
    }
    if !valid_ascii_hostname(candidate) {
        return Ok(());
    }
    let Some(candidate) = crate::host::canonical_domain(candidate) else {
        return Ok(());
    };
    if candidate != suffix[1..] && candidate.ends_with(suffix) {
        candidates.insert(candidate);
        if candidates.len() > MAX_CANDIDATES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta del proveedor supera el límite CHEF de subdominios",
            ));
        }
    }
    Ok(())
}
