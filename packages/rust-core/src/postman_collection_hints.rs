//! Extrae orígenes de requests desde una colección Postman archivada.
//!
//! Se inspeccionan solo `item[].request.url`; auth, headers, variables, cuerpos,
//! ejemplos y nombres se ignoran. Los orígenes son candidatos bajo una raíz
//! explícita, no prueba de propiedad ni autorización para conectar.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_COLLECTION_BYTES: usize = 1024 * 1024;
const MAX_DEPTH: usize = 32;
const MAX_VISITED_ITEMS: usize = 8192;
const MAX_URL_FIELDS: usize = 8192;
const MAX_URL_BYTES: usize = 4096;
const MAX_ORIGINS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedPostmanUrlHints {
    pub query_domain: String,
    pub origins: Vec<String>,
}

/// Analiza una exportación JSON de colección Postman sin conservar secretos ni
/// paths. Solo acepta orígenes HTTP(S) dentro de la raíz indicada.
pub fn parse_archived_postman_collection(
    query_domain: &str,
    collection: &[u8],
) -> ChefResult<ArchivedPostmanUrlHints> {
    if collection.len() > MAX_COLLECTION_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La colección Postman archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz Postman no es un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Postman debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(collection).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La colección Postman archivada no contiene JSON válido",
        )
    })?;

    let mut stack = vec![(&document, 0usize)];
    let mut visited_items = 0usize;
    let mut url_fields = 0usize;
    let mut origins = BTreeSet::new();
    let suffix = format!(".{query_domain}");
    while let Some((value, depth)) = stack.pop() {
        if depth > MAX_DEPTH {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La colección Postman supera la profundidad CHEF",
            ));
        }
        match value {
            Value::Array(items) => {
                for item in items {
                    visited_items += 1;
                    if visited_items > MAX_VISITED_ITEMS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La colección Postman supera el máximo de elementos",
                        ));
                    }
                    stack.push((item, depth + 1));
                }
            }
            Value::Object(object) => {
                if let Some(url) = object
                    .get("request")
                    .and_then(Value::as_object)
                    .and_then(|request| request.get("url"))
                    .and_then(postman_url_value)
                {
                    url_fields += 1;
                    if url_fields > MAX_URL_FIELDS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La colección Postman supera el máximo de campos URL",
                        ));
                    }
                    if let Some(origin) = scoped_origin(url, &query_domain, &suffix) {
                        origins.insert(origin);
                        if origins.len() > MAX_ORIGINS {
                            return Err(ChefError::new(
                                ErrorCode::LimitExceeded,
                                "La colección Postman supera el máximo de orígenes CHEF",
                            ));
                        }
                    }
                }
                if let Some(items) = object.get("item") {
                    if !items.is_array() {
                        return Err(ChefError::new(
                            ErrorCode::InvalidInput,
                            "Cada campo item Postman debe contener una lista",
                        ));
                    }
                    stack.push((items, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(ArchivedPostmanUrlHints {
        query_domain,
        origins: origins.into_iter().collect(),
    })
}

fn postman_url_value(value: &Value) -> Option<&str> {
    value.as_str().or_else(|| {
        value
            .as_object()
            .and_then(|object| object.get("raw"))
            .and_then(Value::as_str)
    })
}

fn scoped_origin(value: &str, root: &str, suffix: &str) -> Option<String> {
    if value.len() > MAX_URL_BYTES || value.chars().any(char::is_control) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str().and_then(canonical_dns_name)?;
    if host != root && !host.ends_with(suffix) {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_postman_collection;
    use crate::ErrorCode;

    #[test]
    fn extracts_scoped_origins_and_discards_paths_queries_and_secrets() {
        let collection = br#"{"info":{"name":"secret collection name"},"auth":{"bearer":[{"key":"token","value":"secret"}]},"item":[{"name":"request one","request":{"method":"POST","header":[{"key":"Authorization","value":"secret"}],"body":{"raw":"secret body"},"url":"https://api.example.com/v1?token=secret#frag"}},{"request":{"url":{"raw":"http://www.example.com/contact?key=value"}}},{"request":{"url":"https://example.com.attacker.test/path"}},{"request":{"url":"https://user@example.com/private"}}]}"#;
        let result = parse_archived_postman_collection("example.com", collection).unwrap();
        assert_eq!(
            result.origins,
            ["http://www.example.com", "https://api.example.com"]
        );
        assert!(
            result
                .origins
                .iter()
                .all(|origin| !origin.contains("secret"))
        );
        assert!(result.origins.iter().all(|origin| {
            origin
                .split_once("://")
                .is_some_and(|(_, authority)| !authority.contains('/'))
        }));
    }

    #[test]
    fn rejects_invalid_roots_and_malformed_item_shapes() {
        assert_eq!(
            parse_archived_postman_collection("co.uk", br#"{"item":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_postman_collection("example.com", br#"{"item":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_collection_size_depth_items_and_origin_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_postman_collection("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let mut deeply_nested = serde_json::json!({"request":{"url":"https://api.example.com"}});
        for _ in 0..33 {
            deeply_nested = serde_json::json!({"item":[deeply_nested]});
        }
        let response = serde_json::to_vec(&deeply_nested).unwrap();
        assert_eq!(
            parse_archived_postman_collection("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let items = (0..513)
            .map(|index| {
                serde_json::json!({"request":{"url":format!("https://host{index}.example.com")}})
            })
            .collect::<Vec<_>>();
        let response = serde_json::to_vec(&serde_json::json!({"item": items})).unwrap();
        assert_eq!(
            parse_archived_postman_collection("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
