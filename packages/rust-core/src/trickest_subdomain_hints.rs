//! Normaliza nombres desde una respuesta Trickest archivada.
//!
//! Los nombres son pistas históricas del conjunto de datos, no observaciones
//! DNS ni confirmación de que sigan resolviendo.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_RESULT_ENTRIES: usize = 4096;
const MAX_SUBDOMAIN_HINTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedTrickestSubdomains {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Extrae `results[].hostname` de una página JSON archivada.
///
/// Se conservan solo descendientes estrictos de una raíz registrable ICANN,
/// normalizados, ordenados y deduplicados.
pub fn parse_archived_trickest_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedTrickestSubdomains> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Trickest archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta Trickest debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Trickest debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Trickest no contiene JSON válido",
        )
    })?;
    let results = document
        .as_object()
        .and_then(|object| object.get("results"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Trickest debe contener una lista results",
        ))?;
    if results.len() > MAX_RESULT_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Trickest supera el máximo de filas",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for result in results {
        let Some(hostname) = result
            .as_object()
            .and_then(|object| object.get("hostname"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(hostname) = canonical_dns_name(hostname) else {
            continue;
        };
        if hostname.ends_with(&suffix) {
            candidates.insert(hostname);
            if candidates.len() > MAX_SUBDOMAIN_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta Trickest supera el máximo CHEF de candidatos",
                ));
            }
        }
    }
    Ok(ArchivedTrickestSubdomains {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_trickest_subdomains;
    use crate::ErrorCode;

    #[test]
    fn extracts_unique_descendants_from_archived_page() {
        let response = br#"{"results":[{"hostname":"API.Example.com"},{"hostname":"example.com"},{"hostname":"api.example.com"},{"hostname":"evil-example.com"},{"hostname":"other.net"},{}]}"#;
        let result = parse_archived_trickest_subdomains("EXAMPLE.COM.", response).unwrap();
        assert_eq!(result.candidates, vec!["api.example.com"]);
    }

    #[test]
    fn rejects_invalid_scope_and_response_shape() {
        assert_eq!(
            parse_archived_trickest_subdomains("not a domain", br#"{"results":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_trickest_subdomains("example.com", br#"{"results":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_row_and_candidate_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_trickest_subdomains("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let many_rows = vec![serde_json::json!({"hostname":"a.example.com"}); 4097];
        let response = serde_json::to_vec(&serde_json::json!({"results": many_rows})).unwrap();
        assert_eq!(
            parse_archived_trickest_subdomains("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
