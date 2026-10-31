//! Extrae descendientes de una respuesta C99 archivada.
//!
//! Este adaptador solo interpreta el JSON `subdomains[].subdomain`; delega la
//! validación de raíz, nombres y scope de candidato en el parser CHEF común.
//! No consulta C99 ni acredita resolución o estado actual.

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, provider_subdomains::ProviderSubdomainPage};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ENTRIES: usize = 4096;
const MAX_NAME_BYTES: usize = 253;

/// Extrae candidatos desde JSON archivado de C99 y reutiliza el normalizador
/// común CHEF para descartar nombres inválidos o fuera de la raíz declarada.
pub fn parse_archived_c99_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ProviderSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta C99 archivada supera el límite de bytes",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta C99 archivada no contiene JSON válido",
        )
    })?;
    let entries = document
        .as_object()
        .and_then(|object| object.get("subdomains"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta C99 debe contener subdomains como lista",
        ))?;
    if entries.len() > MAX_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta C99 supera el máximo de entradas",
        ));
    }

    let mut hostnames = String::new();
    for entry in entries {
        let Some(hostname) = entry
            .as_object()
            .and_then(|object| object.get("subdomain"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        if hostname.is_empty()
            || hostname.len() > MAX_NAME_BYTES
            || hostname.chars().any(char::is_control)
        {
            continue;
        }
        if hostnames.len() + hostname.len() + 1 > MAX_RESPONSE_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Los nombres C99 superan el máximo de texto CHEF",
            ));
        }
        hostnames.push_str(hostname);
        hostnames.push('\n');
    }
    crate::provider_subdomains::parse_provider_subdomains(query_domain, hostnames.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_c99_subdomains;
    use crate::ErrorCode;

    #[test]
    fn parses_c99_json_and_uses_common_scoped_hostname_rules() {
        let response = br#"{"subdomains":[{"subdomain":"WWW.Example.com"},{"subdomain":"api.example.com"},{"subdomain":"example.com"},{"subdomain":"evil-example.com"},{"subdomain":"*.example.com"}]}"#;
        let result = parse_archived_c99_subdomains("example.com", response).unwrap();
        assert_eq!(result.query_domain, "example.com");
        assert_eq!(result.candidates, ["api.example.com", "www.example.com"]);
    }

    #[test]
    fn rejects_malformed_json_shape_and_invalid_query_domain() {
        assert_eq!(
            parse_archived_c99_subdomains("example.com", br#"{"subdomains":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_c99_subdomains("co.uk", br#"{"subdomains":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_entry_and_common_candidate_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_c99_subdomains("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_entries = vec![serde_json::json!({"subdomain":"api.example.com"}); 4097];
        let response =
            serde_json::to_vec(&serde_json::json!({"subdomains": too_many_entries})).unwrap();
        assert_eq!(
            parse_archived_c99_subdomains("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let candidates = (0..513)
            .map(|index| serde_json::json!({"subdomain": format!("host{index}.example.com")}))
            .collect::<Vec<_>>();
        let response = serde_json::to_vec(&serde_json::json!({"subdomains": candidates})).unwrap();
        assert_eq!(
            parse_archived_c99_subdomains("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
