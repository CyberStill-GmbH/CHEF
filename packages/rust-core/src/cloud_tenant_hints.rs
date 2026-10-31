//! Normaliza dominios de tenant Azure desde una respuesta archivada.
//!
//! Los nombres y el identificador se conservan como contexto de proveedor. No
//! se consultan endpoints ni se derivan hallazgos de configuración o riesgo.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 256 * 1024;
const MAX_OBJECT_FIELDS: usize = 64;
const MAX_DOMAIN_VALUES: usize = 4096;
const MAX_DOMAIN_HINTS: usize = 512;
const MAX_TEXT_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedCloudTenantHints {
    pub query_domain: String,
    pub tenant_id_hint: Option<String>,
    pub tenant_name_hint: Option<String>,
    /// Dominios del resultado dentro de la raíz consultada, excluida la raíz.
    pub in_scope_domain_hints: Vec<String>,
    /// Dominio Microsoft hospedado, guardado como relación contextual externa.
    pub onmicrosoft_domain_hint: Option<String>,
}

/// Extrae contexto de una respuesta `tenant_data` archivada del módulo fuente.
///
/// Solo los dominios dentro de la raíz declarada se presentan como candidatos
/// CHEF. Un dominio `onmicrosoft.com` se devuelve separado y no amplía el scope.
pub fn parse_archived_cloud_tenant(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedCloudTenantHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta tenant archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta de tenant debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta de tenant debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta tenant archivada no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta tenant archivada debe ser un objeto JSON",
    ))?;
    if object.len() > MAX_OBJECT_FIELDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta tenant supera el máximo de campos",
        ));
    }

    let tenant_id_hint = object
        .get("tenant_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| valid_guid(value))
        .map(str::to_ascii_lowercase);
    let tenant_name_hint = bounded_text(object.get("tenant_name"))?;
    let mut in_scope_domain_hints = BTreeSet::new();
    let mut onmicrosoft_domain_hint = None;
    if let Some(values) = object.get("email_domains") {
        let values = values.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "email_domains debe ser una lista JSON",
        ))?;
        if values.len() > MAX_DOMAIN_VALUES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta tenant supera el máximo de dominios",
            ));
        }
        let suffix = format!(".{query_domain}");
        for value in values.iter().filter_map(Value::as_str) {
            let Some(domain) = canonical_dns_name(value) else {
                continue;
            };
            if domain.ends_with(".onmicrosoft.com") {
                if onmicrosoft_domain_hint.is_none() {
                    onmicrosoft_domain_hint = Some(domain);
                }
                continue;
            }
            if domain != query_domain && domain.ends_with(&suffix) {
                in_scope_domain_hints.insert(domain);
                if in_scope_domain_hints.len() > MAX_DOMAIN_HINTS {
                    return Err(ChefError::new(
                        ErrorCode::LimitExceeded,
                        "La respuesta tenant supera el máximo de dominios CHEF",
                    ));
                }
            }
        }
    }

    let tenant_name_hint = tenant_name_hint.or_else(|| {
        onmicrosoft_domain_hint
            .as_deref()
            .and_then(|domain| domain.strip_suffix(".onmicrosoft.com"))
            .and_then(|domain| domain.split('.').next())
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
    });
    Ok(ArchivedCloudTenantHints {
        query_domain,
        tenant_id_hint,
        tenant_name_hint,
        in_scope_domain_hints: in_scope_domain_hints.into_iter().collect(),
        onmicrosoft_domain_hint,
    })
}

fn bounded_text(value: Option<&Value>) -> ChefResult<Option<String>> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El nombre de tenant archivado supera el límite CHEF",
        ));
    }
    Ok(Some(value.to_owned()))
}

fn valid_guid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_cloud_tenant;
    use crate::ErrorCode;

    #[test]
    fn separates_scoped_domains_from_external_tenant_context() {
        let response = br#"{"tenant_id":"12345678-1234-abcd-9876-1234567890ab","tenant_name":"Example Org","email_domains":["example.com","api.example.com","portal.sub.example.com","example.onmicrosoft.com","evil-example.com"]}"#;
        let result = parse_archived_cloud_tenant("EXAMPLE.COM.", response).unwrap();
        assert_eq!(result.tenant_name_hint.as_deref(), Some("Example Org"));
        assert_eq!(
            result.in_scope_domain_hints,
            vec!["api.example.com", "portal.sub.example.com"]
        );
        assert_eq!(
            result.onmicrosoft_domain_hint.as_deref(),
            Some("example.onmicrosoft.com")
        );
        assert_eq!(
            result.tenant_id_hint.as_deref(),
            Some("12345678-1234-abcd-9876-1234567890ab")
        );
    }

    #[test]
    fn filters_invalid_tenant_id_and_rejects_bad_domain_array() {
        let result = parse_archived_cloud_tenant(
            "example.com",
            br#"{"tenant_id":"not-a-guid","email_domains":["bad_domain","other.test"]}"#,
        )
        .unwrap();
        assert_eq!(result.tenant_id_hint, None);
        assert!(result.in_scope_domain_hints.is_empty());
        assert_eq!(
            parse_archived_cloud_tenant("example.com", br#"{"email_domains":"bad"}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_domain_count_limits() {
        let oversized = vec![b'x'; 256 * 1024 + 1];
        assert_eq!(
            parse_archived_cloud_tenant("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let domains = vec!["a.example.com"; 4097];
        let response = serde_json::to_vec(&serde_json::json!({"email_domains": domains})).unwrap();
        assert_eq!(
            parse_archived_cloud_tenant("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
