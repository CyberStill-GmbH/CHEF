//! Decodifica offline de direcciones `data-cfemail` en HTML archivado.
//!
//! Adapta la transformación del conector de formato de correo del origen sin
//! realizar HTTP ni convertir el resultado en una observación actual. El
//! llamador conserva el HTML original y su linaje como evidencia.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_ENCODED_VALUES: usize = 4096;
const MAX_EMAIL_BYTES: usize = 254;

/// Extrae correos ofuscados de Cloudflare desde HTML archivado.
///
/// Solo devuelve direcciones cuyo dominio sea la raíz indicada o un subdominio
/// suyo. La comparación respeta límites de etiqueta (`evil-example.com` no
/// coincide con `example.com`). Las direcciones son candidatas históricas.
pub fn parse_archived_cfemail_html(html: &str, queried_domain: &str) -> ChefResult<Vec<String>> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML archivado supera el límite de tamaño para correos",
        ));
    }
    let domain = canonical_dns_name(queried_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El dominio consultado no es un nombre DNS válido",
    ))?;

    let mut emails = BTreeSet::new();
    let mut cursor = 0;
    let mut encoded_count = 0;
    while let Some(relative) = html[cursor..].find("data-cfemail=") {
        let attribute_start = cursor + relative + "data-cfemail=".len();
        let Some(quote) = html.as_bytes().get(attribute_start).copied() else {
            break;
        };
        if quote != b'\'' && quote != b'"' {
            cursor = attribute_start;
            continue;
        }
        let value_start = attribute_start + 1;
        let Some(end_relative) = html.as_bytes()[value_start..]
            .iter()
            .position(|byte| *byte == quote)
        else {
            break;
        };
        let value_end = value_start + end_relative;
        let encoded = &html[value_start..value_end];
        cursor = value_end + 1;
        encoded_count += 1;
        if encoded_count > MAX_ENCODED_VALUES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML archivado supera el máximo de valores cfemail",
            ));
        }

        let Some(email) = decode_cfemail(encoded) else {
            continue;
        };
        if !valid_email_for_domain(&email, &domain) {
            continue;
        }
        emails.insert(email);
    }
    Ok(emails.into_iter().collect())
}

fn decode_cfemail(encoded: &str) -> Option<String> {
    if encoded.len() < 4
        || !encoded.len().is_multiple_of(2)
        || !encoded.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    let key = u8::from_str_radix(&encoded[..2], 16).ok()?;
    let mut decoded = Vec::with_capacity((encoded.len() - 2) / 2);
    for pair in encoded.as_bytes()[2..].chunks(2) {
        let pair = std::str::from_utf8(pair).ok()?;
        decoded.push(u8::from_str_radix(pair, 16).ok()? ^ key);
    }
    String::from_utf8(decoded)
        .ok()
        .map(|value| value.to_lowercase())
}

fn valid_email_for_domain(email: &str, queried_domain: &str) -> bool {
    if email.is_empty()
        || email.len() > MAX_EMAIL_BYTES
        || email
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return false;
    }
    let Some((local, domain)) = email.rsplit_once('@') else {
        return false;
    };
    if local.is_empty() || local.len() > 64 || domain.is_empty() || email.matches('@').count() != 1
    {
        return false;
    }
    let Some(domain) = canonical_dns_name(domain) else {
        return false;
    };
    domain == queried_domain || domain.ends_with(&format!(".{queried_domain}"))
}

#[cfg(test)]
mod tests {
    use super::{decode_cfemail, parse_archived_cfemail_html};
    use crate::ErrorCode;

    fn encode(value: &str, key: u8) -> String {
        let mut encoded = format!("{key:02x}");
        for byte in value.bytes() {
            encoded.push_str(&format!("{:02x}", byte ^ key));
        }
        encoded
    }

    #[test]
    fn decodes_and_filters_archived_cloudflare_addresses() {
        let encoded = encode("Security@Sub.Example.com", 0x4a);
        let html = format!(r#"<a data-cfemail="{encoded}">contact</a>"#);
        assert_eq!(
            parse_archived_cfemail_html(&html, "EXAMPLE.COM.").unwrap(),
            vec!["security@sub.example.com"]
        );
    }

    #[test]
    fn rejects_bad_encoding_and_domain_suffix_collision() {
        let accepted = encode("a@evil-example.com", 0x10);
        let html = format!(
            r#"<span data-cfemail="00xz">x</span><span data-cfemail="{accepted}">y</span>"#
        );
        assert!(
            parse_archived_cfemail_html(&html, "example.com")
                .unwrap()
                .is_empty()
        );
        assert!(decode_cfemail("0").is_none());
    }

    #[test]
    fn enforces_html_limit_and_domain_validation() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_cfemail_html(&oversized, "example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_cfemail_html("", "https://example.com")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
