//! Extrae el endpoint declarado por una configuración OIDC archivada.
//!
//! Adaptación offline de la lectura `token_endpoint` de la fuente: no obtiene
//! discovery documents, no analiza valores arbitrarios y no realiza OAuth.
//! El endpoint queda como candidato basado en evidencia, no como hallazgo.

use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_ENDPOINT_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedOidcEndpointHint {
    /// URL normalizada, sin credenciales, query ni fragmento.
    pub endpoint_url: String,
    /// Huella de la respuesta archivada que respalda el candidato.
    pub response_sha256: String,
}

/// Lee `token_endpoint` de un documento OIDC archivado y aplica scope explícito.
///
/// Solo se devuelve el endpoint si su host coincide con la raíz registrable
/// indicada o es un subdominio suyo. La salida no autoriza ninguna solicitud.
pub fn parse_archived_oidc_config(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<Option<ArchivedOidcEndpointHint>> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La configuración OIDC archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz consultada debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz consultada debe ser un dominio registrable ICANN",
        ));
    }

    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La configuración OIDC archivada no contiene JSON válido",
        )
    })?;
    let Some(raw_endpoint) = document
        .as_object()
        .and_then(|object| object.get("token_endpoint"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let Some(endpoint_url) = normalize_scoped_endpoint(raw_endpoint, &query_domain) else {
        return Ok(None);
    };

    let digest = Sha256::digest(response);
    Ok(Some(ArchivedOidcEndpointHint {
        endpoint_url,
        response_sha256: format!("{digest:x}"),
    }))
}

fn normalize_scoped_endpoint(value: &str, query_domain: &str) -> Option<String> {
    if value.len() > MAX_ENDPOINT_BYTES {
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
    let suffix = format!(".{query_domain}");
    if host != query_domain && !host.ends_with(&suffix) {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_oidc_config;
    use crate::ErrorCode;

    #[test]
    fn extracts_only_scoped_token_endpoint_and_sanitizes_secrets() {
        let response = br#"{"issuer":"https://login.example.com","token_endpoint":"https://auth.example.com/tenant/oauth2/token?client_secret=private#fragment","authorization_endpoint":"https://outside.test/auth","email":"analyst@example.com"}"#;
        let hint = parse_archived_oidc_config("example.com", response)
            .unwrap()
            .unwrap();
        assert_eq!(
            hint.endpoint_url,
            "https://auth.example.com/tenant/oauth2/token"
        );
        assert_eq!(hint.response_sha256.len(), 64);
        let rendered = format!("{hint:?}");
        assert!(!rendered.contains("private"));
        assert!(!rendered.contains("analyst@example.com"));
        assert!(!rendered.contains("outside.test"));
    }

    #[test]
    fn excludes_external_lookalikes_credentials_and_non_http_schemes() {
        for endpoint in [
            "https://evil-example.com/token",
            "https://user:secret@auth.example.com/token",
            "javascript:alert(1)",
        ] {
            let response = serde_json::json!({"token_endpoint":endpoint});
            assert!(
                parse_archived_oidc_config("example.com", &serde_json::to_vec(&response).unwrap())
                    .unwrap()
                    .is_none()
            );
        }
        assert_eq!(
            parse_archived_oidc_config(
                "localhost",
                br#"{"token_endpoint":"https://auth.example.com/token"}"#,
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn treats_missing_or_out_of_scope_endpoint_as_no_candidate() {
        assert!(
            parse_archived_oidc_config("example.com", br#"{"issuer":"https://example.com"}"#)
                .unwrap()
                .is_none()
        );
        assert!(
            parse_archived_oidc_config(
                "example.com",
                br#"{"token_endpoint":"https://identity.other.test/token"}"#
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn rejects_invalid_json_and_oversized_documents() {
        assert_eq!(
            parse_archived_oidc_config("example.com", b"not-json")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        let oversized = vec![b'x'; 512 * 1024 + 1];
        assert_eq!(
            parse_archived_oidc_config("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
