//! Extrae direcciones candidatas de respuestas PGP archivadas.
//!
//! Adapta la transformación del módulo pasivo de búsqueda PGP sobre contenido
//! ya guardado. No consulta servidores de claves ni confirma vigencia o control.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_AT_SIGNS: usize = 16_384;
const MAX_EMAILS: usize = 512;

/// Extrae correos de una respuesta PGP archivada bajo el dominio consultado.
///
/// Los resultados son pistas históricas. Solo acepta el dominio raíz o un
/// subdominio con límite de etiqueta; ordena y deduplica las direcciones.
pub fn parse_archived_pgp_emails(content: &str, query_domain: &str) -> ChefResult<Vec<String>> {
    if content.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta PGP archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El dominio consultado para PGP no es un nombre DNS válido",
    ))?;

    let bytes = content.as_bytes();
    let mut at_signs_seen = 0;
    let mut emails = BTreeSet::new();
    for (at, byte) in bytes.iter().enumerate() {
        if *byte != b'@' {
            continue;
        }
        at_signs_seen += 1;
        if at_signs_seen > MAX_AT_SIGNS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta PGP archivada contiene demasiados candidatos",
            ));
        }
        if at == 0 || at + 1 >= bytes.len() || bytes[at - 1] == b'@' || bytes[at + 1] == b'@' {
            continue;
        }

        let mut local_start = at;
        while local_start > 0 && is_local_byte(bytes[local_start - 1]) {
            local_start -= 1;
        }
        let mut domain_end = at + 1;
        while domain_end < bytes.len() && is_domain_byte(bytes[domain_end]) {
            domain_end += 1;
        }
        let local = &content[local_start..at];
        let raw_domain = &content[at + 1..domain_end];
        if !valid_local_part(local) {
            continue;
        }
        let Some(domain) = canonical_dns_name(raw_domain) else {
            continue;
        };
        if domain != query_domain && !domain.ends_with(&format!(".{query_domain}")) {
            continue;
        }

        emails.insert(format!("{}@{domain}", local.to_ascii_lowercase()));
        if emails.len() > MAX_EMAILS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta PGP archivada supera el máximo de correos",
            ));
        }
    }
    Ok(emails.into_iter().collect())
}

fn is_local_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'%' | b'+' | b'-')
}

fn is_domain_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-')
}

fn valid_local_part(local: &str) -> bool {
    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local.bytes().all(is_local_byte)
}

#[cfg(test)]
mod tests {
    use super::parse_archived_pgp_emails;
    use crate::ErrorCode;

    #[test]
    fn extracts_deduplicated_archived_addresses_for_query_domain() {
        let content = "<a>Admin@Sub.Example.com</a> ops@example.com admin@sub.example.com";
        assert_eq!(
            parse_archived_pgp_emails(content, "EXAMPLE.COM.").unwrap(),
            vec!["admin@sub.example.com", "ops@example.com"]
        );
    }

    #[test]
    fn filters_invalid_addresses_and_suffix_collisions() {
        let content = "bad..name@example.com x@evil-example.com y@other.net valid@example.com";
        assert_eq!(
            parse_archived_pgp_emails(content, "example.com").unwrap(),
            vec!["valid@example.com"]
        );
    }

    #[test]
    fn enforces_response_candidate_and_domain_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_pgp_emails(&oversized, "example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_at_signs = "@".repeat(16_385);
        assert_eq!(
            parse_archived_pgp_emails(&too_many_at_signs, "example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_pgp_emails("", "https://example.com")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
