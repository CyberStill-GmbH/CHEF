//! Extrae referencias de origen de una respuesta Hunter archivada.
//!
//! Los valores de correo y el resto de datos personales se ignoran. Solo se
//! conservan dominios/orígenes HTTP(S) bajo la raíz consultada; no se atribuye
//! afiliación, relación laboral ni propiedad.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_EMAIL_ENTRIES: usize = 4096;
const MAX_SOURCES_PER_ENTRY: usize = 128;
const MAX_TOTAL_SOURCES: usize = 8192;
const MAX_HINTS: usize = 512;
const MAX_SOURCE_VALUE_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedHunterSourceHints {
    pub query_domain: String,
    pub source_domains: Vec<String>,
    pub source_url_origins: Vec<String>,
}

/// Extrae solo fuentes asociadas a filas de Hunter; no conserva direcciones de
/// correo ni otros campos de cada entrada.
pub fn parse_archived_hunter_sources(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedHunterSourceHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Hunter archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta Hunter no es un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Hunter debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Hunter archivada no contiene JSON válido",
        )
    })?;
    let emails = document
        .as_object()
        .and_then(|object| object.get("data"))
        .and_then(Value::as_object)
        .and_then(|data| data.get("emails"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Hunter debe contener data.emails como lista",
        ))?;
    if emails.len() > MAX_EMAIL_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Hunter supera el máximo de filas",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut source_domains = BTreeSet::new();
    let mut source_url_origins = BTreeSet::new();
    let mut total_sources = 0usize;
    for email_entry in emails {
        let Some(sources) = email_entry
            .as_object()
            .and_then(|entry| entry.get("sources"))
            .and_then(Value::as_array)
        else {
            continue;
        };
        if sources.len() > MAX_SOURCES_PER_ENTRY {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Una fila Hunter supera el máximo de fuentes",
            ));
        }
        total_sources += sources.len();
        if total_sources > MAX_TOTAL_SOURCES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta Hunter supera el máximo total de fuentes",
            ));
        }
        for source in sources {
            let Some(source) = source.as_object() else {
                continue;
            };
            if let Some(domain) = source.get("domain").and_then(Value::as_str)
                && domain.len() <= 253
                && !domain.contains('*')
                && let Some(domain) = canonical_dns_name(domain)
                && is_in_root(&domain, &query_domain, &suffix)
            {
                source_domains.insert(domain);
            }
            if let Some(uri) = source.get("uri").and_then(Value::as_str)
                && let Some(origin) = scoped_origin(uri, &query_domain, &suffix)
            {
                source_url_origins.insert(origin);
            }
            if source_domains.len() > MAX_HINTS || source_url_origins.len() > MAX_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta Hunter supera el máximo CHEF de referencias",
                ));
            }
        }
    }
    Ok(ArchivedHunterSourceHints {
        query_domain,
        source_domains: source_domains.into_iter().collect(),
        source_url_origins: source_url_origins.into_iter().collect(),
    })
}

fn is_in_root(host: &str, root: &str, suffix: &str) -> bool {
    host == root || host.ends_with(suffix)
}

fn scoped_origin(value: &str, root: &str, suffix: &str) -> Option<String> {
    if value.len() > MAX_SOURCE_VALUE_BYTES || value.chars().any(char::is_control) {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str().and_then(canonical_dns_name)?;
    if !is_in_root(&host, root, suffix) {
        return None;
    }
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_hunter_sources;
    use crate::ErrorCode;

    #[test]
    fn keeps_only_scoped_source_references_and_discards_emails() {
        let response = br#"{"data":{"emails":[{"value":"private.person@example.com","sources":[{"domain":"WWW.Example.com","uri":"https://www.example.com/team?email=private.person%40example.com#bio"},{"domain":"evil-example.com","uri":"https://example.com.attacker.test/page"},{"domain":"example.com","uri":"https://user@example.com/private"}]},{"value":"another@example.com","sources":[{"domain":"example.com","uri":"http://example.com/contact"}]}]}}"#;
        let result = parse_archived_hunter_sources("example.com", response).unwrap();
        assert_eq!(result.source_domains, ["example.com", "www.example.com"]);
        assert_eq!(
            result.source_url_origins,
            ["http://example.com", "https://www.example.com"]
        );
        assert!(
            result
                .source_domains
                .iter()
                .all(|value| !value.contains('@'))
        );
        assert!(
            result
                .source_url_origins
                .iter()
                .all(|value| !value.contains("private.person") && !value.contains("another@"))
        );
    }

    #[test]
    fn rejects_invalid_query_and_response_shape() {
        assert_eq!(
            parse_archived_hunter_sources("co.uk", br#"{"data":{"emails":[]}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_hunter_sources("example.com", br#"{"data":{"emails":{}}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_record_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_hunter_sources("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_sources = serde_json::json!({"data":{"emails":[{"sources":vec![serde_json::json!({"domain":"example.com"}); 129]}]}});
        let response = serde_json::to_vec(&too_many_sources).unwrap();
        assert_eq!(
            parse_archived_hunter_sources("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
