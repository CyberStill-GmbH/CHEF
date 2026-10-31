//! Convierte un `security.txt` archivado en referencias de contacto CHEF.
//!
//! No solicita el documento ni valida la disponibilidad de sus contactos. Los
//! correos y URL devueltos son pistas históricas bajo el dominio consultado.

use std::collections::BTreeSet;

use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_DOCUMENT_BYTES: usize = 1024 * 1024;
const MAX_FIELDS: usize = 4096;
const MAX_FIELD_VALUE_BYTES: usize = 16 * 1024;
const MAX_CONTACTS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSecurityContactHints {
    pub domain: String,
    pub email_hints: Vec<String>,
    pub url_hints: Vec<String>,
}

/// Extrae correos y URL en alcance desde un `security.txt` archivado.
///
/// Como en la transformación de origen, el documento debe contener una
/// directiva `Contact` o `Expires` antes de procesar otros campos. Se excluye
/// la URL canónica del propio documento y los enlaces fuera del dominio raíz.
pub fn parse_archived_security_contacts(
    content: &str,
    document_url: &str,
) -> ChefResult<ArchivedSecurityContactHints> {
    if content.len() > MAX_DOCUMENT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El security.txt archivado supera el límite de bytes",
        ));
    }
    let document = parse_document_url(document_url)?;
    let domain =
        canonical_dns_name(document.host_str().unwrap_or_default()).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "El security.txt debe pertenecer a un nombre DNS válido",
        ))?;
    let mut email_hints = BTreeSet::new();
    let mut url_hints = BTreeSet::new();
    let mut fields_seen = 0;
    let mut looks_like_security_txt = false;

    for line in content.lines() {
        let Some((raw_name, raw_value)) = line.split_once(':') else {
            continue;
        };
        let name = raw_name.trim();
        if !valid_field_name(name) {
            continue;
        }
        let name = name.to_ascii_lowercase();
        if name == "contact" || name == "expires" {
            looks_like_security_txt = true;
        }
        fields_seen += 1;
        if fields_seen > MAX_FIELDS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El security.txt supera el máximo de campos",
            ));
        }
        let value = raw_value.trim();
        if value.len() > MAX_FIELD_VALUE_BYTES {
            continue;
        }
        extract_emails(value, &domain, &mut email_hints)?;
        extract_urls(value, &domain, &document, &mut url_hints)?;
    }

    if !looks_like_security_txt {
        email_hints.clear();
        url_hints.clear();
    }
    Ok(ArchivedSecurityContactHints {
        domain,
        email_hints: email_hints.into_iter().collect(),
        url_hints: url_hints.into_iter().collect(),
    })
}

fn parse_document_url(value: &str) -> ChefResult<Url> {
    let url = Url::parse(value).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La URL del security.txt archivado no es válida",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La URL del security.txt debe ser HTTP(S), sin credenciales ni query",
        ));
    }
    Ok(url)
}

fn valid_field_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn extract_emails(value: &str, domain: &str, emails: &mut BTreeSet<String>) -> ChefResult<()> {
    let bytes = value.as_bytes();
    for (at, byte) in bytes.iter().enumerate() {
        if *byte != b'@' || at == 0 || at + 1 >= bytes.len() {
            continue;
        }
        let mut start = at;
        while start > 0 && is_local_byte(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = at + 1;
        while end < bytes.len() && is_domain_byte(bytes[end]) {
            end += 1;
        }
        let local = &value[start..at];
        let Some(email_domain) = canonical_dns_name(&value[at + 1..end]) else {
            continue;
        };
        if !valid_local_part(local)
            || (email_domain != domain && !email_domain.ends_with(&format!(".{domain}")))
        {
            continue;
        }
        emails.insert(format!("{}@{email_domain}", local.to_ascii_lowercase()));
        if emails.len() > MAX_CONTACTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El security.txt supera el máximo de correos",
            ));
        }
    }
    Ok(())
}

fn extract_urls(
    value: &str,
    domain: &str,
    document: &Url,
    urls: &mut BTreeSet<String>,
) -> ChefResult<()> {
    let lower = value.to_ascii_lowercase();
    let mut cursor = 0;
    while cursor < value.len() {
        let next_http = lower[cursor..].find("http://").map(|index| cursor + index);
        let next_https = lower[cursor..].find("https://").map(|index| cursor + index);
        let Some(start) = [next_http, next_https].into_iter().flatten().min() else {
            break;
        };
        let mut end = start;
        while end < value.len() {
            let character = value[end..]
                .chars()
                .next()
                .expect("índice dentro del texto");
            if character.is_whitespace() || matches!(character, '<' | '>' | '"' | '\'') {
                break;
            }
            end += character.len_utf8();
        }
        let mut candidate = &value[start..end];
        while candidate.chars().last().is_some_and(|character| {
            matches!(
                character,
                '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}'
            )
        }) {
            candidate = &candidate[..candidate.len() - 1];
        }
        cursor = end.max(start + 1);
        let Ok(mut url) = Url::parse(candidate) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let Some(host) = url.host_str().and_then(canonical_dns_name) else {
            continue;
        };
        if host != domain && !host.ends_with(&format!(".{domain}")) {
            continue;
        }
        url.set_query(None);
        url.set_fragment(None);
        let mut canonical_self = document.clone();
        canonical_self.set_path("/.well-known/security.txt");
        canonical_self.set_query(None);
        canonical_self.set_fragment(None);
        if url == canonical_self {
            continue;
        }
        urls.insert(url.to_string());
        if urls.len() > MAX_CONTACTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El security.txt supera el máximo de URL",
            ));
        }
    }
    Ok(())
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
    use super::parse_archived_security_contacts;
    use crate::ErrorCode;

    #[test]
    fn extracts_in_scope_contacts_from_signed_archived_document() {
        let content = "-----BEGIN PGP SIGNED MESSAGE-----\nContact: mailto:Security@Example.com\nContact: https://vdp.sub.example.com/report\nExpires: 2030-01-01T00:00:00Z\nCanonical: https://example.com/.well-known/security.txt\nPolicy: https://outside.test/policy\n";
        let hints = parse_archived_security_contacts(
            content,
            "https://example.com/.well-known/security.txt",
        )
        .unwrap();

        assert_eq!(hints.email_hints, vec!["security@example.com"]);
        assert_eq!(hints.url_hints, vec!["https://vdp.sub.example.com/report"]);
    }

    #[test]
    fn ignores_non_security_documents_and_foreign_references() {
        let content =
            "Note: Contact someone@example.com or visit https://evil-example.com/policy\n";
        let hints = parse_archived_security_contacts(
            content,
            "https://example.com/.well-known/security.txt",
        )
        .unwrap();
        assert!(hints.email_hints.is_empty());
        assert!(hints.url_hints.is_empty());
    }

    #[test]
    fn strips_query_and_fragment_and_excludes_sanitized_self_link() {
        let content = "Contact: https://vdp.example.com/report?token=private#fragment\nContact: https://example.com/.well-known/security.txt?cache=1#section\n";
        let hints = parse_archived_security_contacts(
            content,
            "https://example.com/.well-known/security.txt",
        )
        .unwrap();

        assert_eq!(hints.url_hints, vec!["https://vdp.example.com/report"]);
        assert!(!hints.url_hints[0].contains('?'));
        assert!(!hints.url_hints[0].contains('#'));
    }

    #[test]
    fn enforces_document_and_field_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_security_contacts(
                &oversized,
                "https://example.com/.well-known/security.txt"
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
        let too_many_fields = (0..4097)
            .map(|_| "Contact: a@example.com\n")
            .collect::<String>();
        assert_eq!(
            parse_archived_security_contacts(
                &too_many_fields,
                "https://example.com/.well-known/security.txt"
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_security_contacts("", "file:///tmp/security.txt")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
