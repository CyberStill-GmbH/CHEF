//! Adapta offline una lista archivada de nombres candidatos de subdominio.
//!
//! Conserva el filtro de profundidad del proveedor, pero no consulta el servicio
//! ni confirma que un nombre exista o resuelva en la actualidad.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_JSON_ENTRIES: usize = 4096;
const MAX_RESULT_LIMIT: usize = 1000;
const MAX_DNS_LABELS: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSubdomainCandidates {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Filtra una respuesta JSON archivada de nombres bajo una raíz ICANN.
///
/// El resultado contiene solo subdominios estrictos, sin nombres PTR, con un
/// máximo de cinco etiquetas y ordenados/deduplicados como en la adaptación.
pub fn parse_archived_subdomain_candidates(
    query_domain: &str,
    response: &[u8],
    result_limit: usize,
) -> ChefResult<ArchivedSubdomainCandidates> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta archivada supera el límite de bytes",
        ));
    }
    if result_limit == 0 || result_limit > MAX_RESULT_LIMIT {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El límite de resultados debe estar entre 1 y 1000",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta debe ser una raíz registrable ICANN",
        ));
    }

    let values: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta archivada no contiene JSON válido",
        )
    })?;
    let values = values.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta archivada debe ser una lista JSON",
    ))?;
    if values.len() > MAX_JSON_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta archivada supera el máximo de entradas JSON",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for value in values {
        let Some(value) = value.as_str() else {
            continue;
        };
        let Some(hostname) = canonical_dns_name(value) else {
            continue;
        };
        if !hostname.ends_with(&suffix)
            || hostname.ends_with(".in-addr.arpa")
            || hostname.ends_with(".ip6.arpa")
            || hostname.split('.').count() > MAX_DNS_LABELS
        {
            continue;
        }
        candidates.insert(hostname);
    }

    Ok(ArchivedSubdomainCandidates {
        query_domain,
        candidates: candidates.into_iter().take(result_limit).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_subdomain_candidates;
    use crate::ErrorCode;

    #[test]
    fn filters_and_sorts_archived_subdomain_candidates() {
        let response = br#"["API.Example.com","a.b.c.example.com","example.com","evil-example.com","api.example.com","1.2.0.192.in-addr.arpa",7]"#;
        let result = parse_archived_subdomain_candidates("EXAMPLE.COM.", response, 1000).unwrap();
        assert_eq!(
            result.candidates,
            vec!["a.b.c.example.com", "api.example.com"]
        );
    }

    #[test]
    fn rejects_invalid_domain_and_result_limit() {
        assert_eq!(
            parse_archived_subdomain_candidates("not a domain", b"[]", 10)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_subdomain_candidates("example.com", b"[]", 1001)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_entry_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_subdomain_candidates("example.com", &oversized, 10)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let entries = vec!["a.example.com"; 4097];
        assert_eq!(
            parse_archived_subdomain_candidates(
                "example.com",
                &serde_json::to_vec(&entries).unwrap(),
                10
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
    }
}
