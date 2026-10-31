//! Normaliza candidatos de subdominio desde JSON MySSL archivado.
//!
//! Los nombres son referencias históricas bajo la raíz suministrada. El parser
//! no consulta MySSL, no comprueba DNS y no declara los nombres dentro del scope.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_DATA_ENTRIES: usize = 4096;
const MAX_SUBDOMAIN_HINTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedMysslSubdomainHints {
    pub query_domain: String,
    pub subdomains: Vec<String>,
}

/// Extrae valores `data[].domain` solo si son descendientes DNS estrictos de
/// una raíz registrable ICANN indicada por caller.
pub fn parse_archived_myssl_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedMysslSubdomainHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta MySSL archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz de consulta MySSL no es un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta MySSL debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta MySSL archivada no contiene JSON válido",
        )
    })?;
    let data = document
        .as_object()
        .and_then(|object| object.get("data"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta MySSL debe contener data como lista",
        ))?;
    if data.len() > MAX_DATA_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta MySSL supera el máximo de registros",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut subdomains = BTreeSet::new();
    for entry in data {
        let Some(domain) = entry
            .as_object()
            .and_then(|object| object.get("domain"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if domain.len() > 253 || domain.contains('*') {
            continue;
        }
        let Some(domain) = canonical_dns_name(domain) else {
            continue;
        };
        if domain.ends_with(&suffix) {
            subdomains.insert(domain);
            if subdomains.len() > MAX_SUBDOMAIN_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta MySSL supera el máximo CHEF de subdominios",
                ));
            }
        }
    }
    Ok(ArchivedMysslSubdomainHints {
        query_domain,
        subdomains: subdomains.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_myssl_subdomains;
    use crate::ErrorCode;

    #[test]
    fn extracts_only_strict_descendants_from_archived_json() {
        let response = br#"{"data":[{"domain":"API.Example.com."},{"domain":"api.example.com"},{"domain":"example.com"},{"domain":"*.example.com"},{"domain":"evil-example.com"},{"domain":"other.test"}]}"#;
        let result = parse_archived_myssl_subdomains("EXAMPLE.COM.", response).unwrap();
        assert_eq!(result.query_domain, "example.com");
        assert_eq!(result.subdomains, ["api.example.com"]);
    }

    #[test]
    fn rejects_invalid_roots_and_json_shapes() {
        assert_eq!(
            parse_archived_myssl_subdomains("co.uk", br#"{"data":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_myssl_subdomains("example.com", br#"{"data":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_byte_entry_and_candidate_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_myssl_subdomains("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_entries = vec![serde_json::json!({"domain":"api.example.com"}); 4097];
        let response = serde_json::to_vec(&serde_json::json!({"data": too_many_entries})).unwrap();
        assert_eq!(
            parse_archived_myssl_subdomains("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let domains = (0..513)
            .map(|index| serde_json::json!({"domain": format!("host{index}.example.com")}))
            .collect::<Vec<_>>();
        let response = serde_json::to_vec(&serde_json::json!({"data": domains})).unwrap();
        assert_eq!(
            parse_archived_myssl_subdomains("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
