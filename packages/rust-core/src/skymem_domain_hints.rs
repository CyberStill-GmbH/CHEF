//! Extrae dominios, no direcciones, desde HTML archivado de resultados SkyMem.
//!
//! Las partes locales de emails se descartan y nunca forman parte del resultado.
//! Los dominios son pistas históricas: no prueban buzones, entrega o afiliación.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_EMAIL_MARKERS: usize = 16_384;
const MAX_DOMAIN_HINTS: usize = 512;
const MAX_LOCAL_PART_BYTES: usize = 64;
const MAX_DOMAIN_BYTES: usize = 253;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSkymemDomainHints {
    pub query_domain: String,
    pub domains: Vec<String>,
}

/// Busca tokens con forma de email en HTML ya archivado, conservando solo su
/// dominio si está en la raíz explícita o debajo de ella.
pub fn parse_archived_skymem_domains(
    query_domain: &str,
    html: &str,
) -> ChefResult<ArchivedSkymemDomainHints> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML SkyMem archivado supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz de consulta SkyMem no es un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta SkyMem debe ser una raíz registrable ICANN",
        ));
    }

    let bytes = html.as_bytes();
    let suffix = format!(".{query_domain}");
    let mut domains = BTreeSet::new();
    let mut markers_seen = 0usize;
    for (at, byte) in bytes.iter().enumerate() {
        if *byte != b'@' {
            continue;
        }
        markers_seen += 1;
        if markers_seen > MAX_EMAIL_MARKERS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML supera el máximo de marcadores email CHEF",
            ));
        }
        if !has_bounded_local_part(bytes, at) {
            continue;
        }
        let mut end = at + 1;
        while bytes
            .get(end)
            .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        {
            end += 1;
        }
        if end <= at + 1 || end - (at + 1) > MAX_DOMAIN_BYTES {
            continue;
        }
        let Ok(raw_domain) = std::str::from_utf8(&bytes[at + 1..end]) else {
            continue;
        };
        let Some(domain) = canonical_dns_name(raw_domain) else {
            continue;
        };
        if domain == query_domain || domain.ends_with(&suffix) {
            domains.insert(domain);
            if domains.len() > MAX_DOMAIN_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El HTML supera el máximo CHEF de dominios SkyMem",
                ));
            }
        }
    }
    Ok(ArchivedSkymemDomainHints {
        query_domain,
        domains: domains.into_iter().collect(),
    })
}

fn has_bounded_local_part(bytes: &[u8], at: usize) -> bool {
    let mut start = at;
    while start > 0 && is_local_part_byte(bytes[start - 1]) {
        start -= 1;
        if at - start > MAX_LOCAL_PART_BYTES {
            return false;
        }
    }
    let local = &bytes[start..at];
    !local.is_empty()
        && local.first() != Some(&b'.')
        && local.last() != Some(&b'.')
        && !local.windows(2).any(|pair| pair == b"..")
}

fn is_local_part_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'.' | b'_'
                | b'%'
                | b'+'
                | b'-'
                | b'!'
                | b'#'
                | b'$'
                | b'&'
                | b'\''
                | b'*'
                | b'/'
                | b'='
                | b'?'
                | b'^'
                | b'`'
                | b'{'
                | b'|'
                | b'}'
                | b'~'
        )
}

#[cfg(test)]
mod tests {
    use super::parse_archived_skymem_domains;
    use crate::ErrorCode;

    #[test]
    fn retains_only_scoped_email_domains_not_local_parts() {
        let html = r#"<div>alice@example.com bob@mail.example.com external@other.test not-an-email @example.com</div>"#;
        let result = parse_archived_skymem_domains("example.com", html).unwrap();
        assert_eq!(result.domains, ["example.com", "mail.example.com"]);
        assert!(result.domains.iter().all(|domain| !domain.contains('@')));
        assert!(
            result
                .domains
                .iter()
                .all(|domain| !domain.contains("alice"))
        );
    }

    #[test]
    fn rejects_invalid_root_and_ignores_lookalike_domains() {
        assert_eq!(
            parse_archived_skymem_domains("co.uk", "alice@example.com")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        let result = parse_archived_skymem_domains(
            "example.com",
            "alice@evil-example.com x@sub.example.com.attacker.test good@sub.example.com bad..local@example.com",
        )
        .unwrap();
        assert_eq!(result.domains, ["sub.example.com"]);
    }

    #[test]
    fn enforces_html_marker_and_domain_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_skymem_domains("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_markers = "@".repeat(16_385);
        assert_eq!(
            parse_archived_skymem_domains("example.com", &too_many_markers)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let domains = (0..513)
            .map(|index| format!("user@host{index}.example.com"))
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            parse_archived_skymem_domains("example.com", &domains)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
