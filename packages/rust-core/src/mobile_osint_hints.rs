//! Normaliza respuestas OSINT móviles archivadas en pistas de activos CHEF.
//!
//! Adapta las listas de nombres y URL del módulo origen sin API key ni red.
//! Las salidas son referencias históricas y no prueban disponibilidad actual.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ARRAY_ENTRIES: usize = 4096;
const MAX_HINTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedMobileOsintHints {
    pub query_domain: String,
    pub subdomain_candidates: Vec<String>,
    /// URL bajo el dominio consultado, sin query ni fragmento.
    pub url_hints: Vec<String>,
}

/// Lee `subdomains` y `urls` de una respuesta JSON archivada.
///
/// Solo conserva nombres y URL dentro de la raíz ICANN consultada. Los campos
/// ausentes son válidos porque la fuente usa respuestas separadas por recurso.
pub fn parse_archived_mobile_osint_response(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedMobileOsintHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OSINT móvil archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta OSINT móvil debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta OSINT móvil debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta OSINT móvil no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta OSINT móvil debe ser un objeto JSON",
    ))?;
    if object.len() > 64 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OSINT móvil supera el máximo de campos",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut subdomains = BTreeSet::new();
    if let Some(values) = object.get("subdomains") {
        for value in bounded_array(values, "subdomains")? {
            let Some(value) = value.as_str() else {
                continue;
            };
            let Some(hostname) = canonical_dns_name(value) else {
                continue;
            };
            if hostname != query_domain && hostname.ends_with(&suffix) {
                subdomains.insert(hostname);
                ensure_hint_limit(subdomains.len())?;
            }
        }
    }

    let mut urls = BTreeSet::new();
    if let Some(values) = object.get("urls") {
        for value in bounded_array(values, "urls")? {
            let Some(value) = value.as_str() else {
                continue;
            };
            let Ok(mut url) = Url::parse(value) else {
                continue;
            };
            if !matches!(url.scheme(), "http" | "https")
                || !url.username().is_empty()
                || url.password().is_some()
            {
                continue;
            }
            let Some(host) = url.host_str().and_then(canonical_dns_name) else {
                continue;
            };
            if host != query_domain && !host.ends_with(&suffix) {
                continue;
            }
            url.set_query(None);
            url.set_fragment(None);
            urls.insert(url.to_string());
            ensure_hint_limit(urls.len())?;
        }
    }

    Ok(ArchivedMobileOsintHints {
        query_domain,
        subdomain_candidates: subdomains.into_iter().collect(),
        url_hints: urls.into_iter().collect(),
    })
}

fn bounded_array<'a>(value: &'a Value, field: &str) -> ChefResult<&'a [Value]> {
    let values = value.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        if field == "subdomains" {
            "subdomains debe ser una lista JSON"
        } else {
            "urls debe ser una lista JSON"
        },
    ))?;
    if values.len() > MAX_ARRAY_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OSINT móvil supera el máximo de entradas",
        ));
    }
    Ok(values)
}

fn ensure_hint_limit(count: usize) -> ChefResult<()> {
    if count > MAX_HINTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta OSINT móvil supera el máximo de pistas",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_mobile_osint_response;
    use crate::ErrorCode;

    #[test]
    fn extracts_scoped_subdomains_and_sanitized_urls() {
        let response = br#"{"subdomains":["API.Example.com","example.com","evil-example.com",7],"urls":["https://app.example.com/login?token=secret#form","https://outside.test/path","javascript:alert(1)"]}"#;
        let result = parse_archived_mobile_osint_response("EXAMPLE.COM.", response).unwrap();
        assert_eq!(result.subdomain_candidates, vec!["api.example.com"]);
        assert_eq!(result.url_hints, vec!["https://app.example.com/login"]);
    }

    #[test]
    fn accepts_separate_empty_sections_and_rejects_wrong_shapes() {
        let result =
            parse_archived_mobile_osint_response("example.com", br#"{"urls":[]}"#).unwrap();
        assert!(result.subdomain_candidates.is_empty());
        assert_eq!(
            parse_archived_mobile_osint_response("example.com", br#"{"urls":"not-a-list"}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_entry_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_mobile_osint_response("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let values = vec!["a.example.com"; 4097];
        let response = serde_json::to_vec(&serde_json::json!({"subdomains": values})).unwrap();
        assert_eq!(
            parse_archived_mobile_osint_response("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
