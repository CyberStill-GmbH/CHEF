//! Extrae nombres de `cert_info` TLS archivado como candidatos CHEF.
//!
//! El módulo recibe metadatos ya estructurados; no abre conexiones TLS, valida
//! cadenas de confianza ni afirma que el certificado siga vigente.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_CERT_INFO_BYTES: usize = 512 * 1024;
const MAX_OBJECT_FIELDS: usize = 64;
const MAX_SAN_NAMES: usize = 4096;
const MAX_EMAIL_VALUES: usize = 1024;
const MAX_HINTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedTlsCertificateHints {
    pub endpoint_host: String,
    pub dns_name_hints: Vec<String>,
    pub email_hints: Vec<String>,
}

/// Normaliza un objeto `cert_info` archivado bajo raíces de scope explícitas.
///
/// El nombre común y SAN con prefijo `*.` se reducen a dominios candidatos. Se
/// excluye el host del endpoint y toda referencia que no esté bajo las raíces.
pub fn parse_archived_tls_certificate_hints(
    cert_info: &[u8],
    endpoint_host: &str,
    scope_roots: &[String],
) -> ChefResult<ArchivedTlsCertificateHints> {
    if cert_info.len() > MAX_CERT_INFO_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Los metadatos del certificado archivado superan el límite de bytes",
        ));
    }
    if scope_roots.is_empty() || scope_roots.len() > 32 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las raíces de scope deben contener entre 1 y 32 entradas",
        ));
    }
    let endpoint_host = canonical_dns_name(endpoint_host).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El host del endpoint TLS no es un nombre DNS válido",
    ))?;
    let mut roots = BTreeSet::new();
    for root in scope_roots {
        let Some(root) = canonical_dns_name(root) else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Una raíz de scope no es un nombre DNS válido",
            ));
        };
        roots.insert(root);
    }

    let document: Value = serde_json::from_slice(cert_info).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "Los metadatos del certificado no contienen JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "Los metadatos del certificado deben ser un objeto JSON",
    ))?;
    if object.len() > MAX_OBJECT_FIELDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Los metadatos del certificado superan el máximo de campos",
        ));
    }

    let mut dns_candidates = Vec::new();
    if let Some(common_name) = object.get("common_name").and_then(Value::as_str)
        && let Some(name) = canonical_certificate_name(common_name)
    {
        dns_candidates.push(name);
    }
    if let Some(sans) = object.get("sans") {
        let sans = sans.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Los SAN del certificado deben ser una lista",
        ))?;
        if sans.len() > MAX_SAN_NAMES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Los SAN del certificado superan el máximo CHEF",
            ));
        }
        for san in sans.iter().filter_map(Value::as_str) {
            if let Some(name) = canonical_certificate_name(san) {
                dns_candidates.push(name);
            }
        }
    }

    let mut dns_name_hints = BTreeSet::new();
    for name in dns_candidates {
        if name != endpoint_host && is_in_scope(&name, &roots) {
            dns_name_hints.insert(name);
            check_hint_limit(dns_name_hints.len())?;
        }
    }

    let mut email_hints = BTreeSet::new();
    if let Some(emails) = object.get("emails") {
        let emails = emails.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Los correos del certificado deben ser una lista",
        ))?;
        if emails.len() > MAX_EMAIL_VALUES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Los correos del certificado superan el máximo CHEF",
            ));
        }
        for email in emails.iter().filter_map(Value::as_str) {
            if let Some(email) = canonical_certificate_email(email)
                && let Some((_, domain)) = email.rsplit_once('@')
                && is_in_scope(domain, &roots)
            {
                email_hints.insert(email);
                check_hint_limit(email_hints.len())?;
            }
        }
    }

    Ok(ArchivedTlsCertificateHints {
        endpoint_host,
        dns_name_hints: dns_name_hints.into_iter().collect(),
        email_hints: email_hints.into_iter().collect(),
    })
}

fn canonical_certificate_name(value: &str) -> Option<String> {
    canonical_dns_name(value.trim().strip_prefix("*.").unwrap_or(value.trim()))
}

fn canonical_certificate_email(value: &str) -> Option<String> {
    let value = value.trim();
    let (local, domain) = value.split_once('@')?;
    if local.is_empty()
        || local.len() > 64
        || value.matches('@').count() != 1
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || !local.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
        })
    {
        return None;
    }
    let domain = canonical_dns_name(domain)?;
    Some(format!("{}@{domain}", local.to_ascii_lowercase()))
}

fn is_in_scope(name: &str, roots: &BTreeSet<String>) -> bool {
    roots
        .iter()
        .any(|root| name == root || name.ends_with(&format!(".{root}")))
}

fn check_hint_limit(count: usize) -> ChefResult<()> {
    if count > MAX_HINTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Los metadatos del certificado superan el máximo de pistas",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_tls_certificate_hints;
    use crate::ErrorCode;

    #[test]
    fn extracts_scoped_names_and_emails_from_archived_metadata() {
        let cert_info = br#"{"common_name":"*.example.com","sans":["api.example.com","www.example.com","outside.test"],"emails":["Security@sub.example.com","x@evil-example.com"]}"#;
        let result = parse_archived_tls_certificate_hints(
            cert_info,
            "api.example.com",
            &["example.com".to_owned()],
        )
        .unwrap();
        assert_eq!(
            result.dns_name_hints,
            vec!["example.com", "www.example.com"]
        );
        assert_eq!(result.email_hints, vec!["security@sub.example.com"]);
    }

    #[test]
    fn rejects_malformed_scope_and_wrong_json_shapes() {
        assert_eq!(
            parse_archived_tls_certificate_hints(
                b"[]",
                "api.example.com",
                &["example.com".to_owned()]
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_tls_certificate_hints(
                br#"{"sans":"example.com"}"#,
                "api.example.com",
                &["https://example.com".to_owned()]
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_metadata_and_name_count_limits() {
        let oversized = vec![b'x'; 512 * 1024 + 1];
        assert_eq!(
            parse_archived_tls_certificate_hints(
                &oversized,
                "api.example.com",
                &["example.com".to_owned()]
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
        let sans = vec!["a.example.com"; 4097];
        let cert_info = serde_json::to_vec(&serde_json::json!({"sans": sans})).unwrap();
        assert_eq!(
            parse_archived_tls_certificate_hints(
                &cert_info,
                "api.example.com",
                &["example.com".to_owned()]
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
    }
}
