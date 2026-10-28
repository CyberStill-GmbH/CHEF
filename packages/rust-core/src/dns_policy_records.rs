//! Extracción local y acotada de referencias DNS desde RDATA CAA importado.
//!
//! Los destinos solo identifican nombres citados por una política CAA. No
//! implican propiedad, autorización, estado actual ni una relación de confianza.

use std::collections::BTreeSet;

use url::{Host, Url};

const MAX_CAA_RDATA_BYTES: usize = 64 * 1024;
const MAX_CAA_VALUE_BYTES: usize = 4 * 1024;

/// Un nombre citado en el valor de un registro CAA y el papel de ese valor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct CaaDnsNameHint {
    pub name: String,
    pub role: &'static str,
}

/// Extrae únicamente hosts literales de CAA RDATA con formato de presentación.
///
/// `issue*` produce el nombre de CA; `iodef` y `contactemail` producen solo el
/// dominio del URI/correo. Nunca devuelve URI, ruta, query, correo ni RDATA.
pub(crate) fn extract_caa_dns_name_hints(rdata: &str) -> Vec<CaaDnsNameHint> {
    if rdata.len() > MAX_CAA_RDATA_BYTES || rdata.chars().any(char::is_control) {
        return Vec::new();
    }

    let Some((flags, remainder)) = split_ascii_whitespace(rdata.trim()) else {
        return Vec::new();
    };
    if flags.parse::<u8>().is_err() {
        return Vec::new();
    }

    let Some((tag, value)) = split_ascii_whitespace(remainder.trim()) else {
        return Vec::new();
    };
    let tag = tag.to_ascii_lowercase();
    if tag.is_empty() || tag.len() > 15 || !tag.bytes().all(|byte| byte.is_ascii_alphanumeric()) {
        return Vec::new();
    }

    let Some(value) = unquote_value(value.trim()) else {
        return Vec::new();
    };
    if value.len() > MAX_CAA_VALUE_BYTES || value.chars().any(char::is_control) {
        return Vec::new();
    }

    let extracted = match tag.as_str() {
        "issue" => ca_issuer_host(&value).map(|name| (name, "issuer")),
        "issuewild" => ca_issuer_host(&value).map(|name| (name, "wildcard-issuer")),
        "iodef" => report_host(&value).map(|name| (name, "report")),
        "contactemail" => email_domain(&value).map(|name| (name, "contact")),
        _ => None,
    };

    let Some((name, role)) = extracted else {
        return Vec::new();
    };
    BTreeSet::from([CaaDnsNameHint { name, role }])
        .into_iter()
        .collect()
}

fn split_ascii_whitespace(value: &str) -> Option<(&str, &str)> {
    let separator = value.find(char::is_whitespace)?;
    let (head, tail) = value.split_at(separator);
    Some((head, tail.trim_start()))
}

fn unquote_value(value: &str) -> Option<String> {
    if let Some(quoted) = value.strip_prefix('"') {
        let quoted = quoted.strip_suffix('"')?;
        let mut output = String::with_capacity(quoted.len());
        let mut chars = quoted.chars();
        while let Some(character) = chars.next() {
            if character == '\\' {
                match chars.next()? {
                    '"' => output.push('"'),
                    '\\' => output.push('\\'),
                    _ => return None,
                }
            } else if character == '"' {
                return None;
            } else {
                output.push(character);
            }
        }
        Some(output)
    } else if value.contains('"') {
        None
    } else {
        Some(value.to_owned())
    }
}

fn ca_issuer_host(value: &str) -> Option<String> {
    // An empty issuer (including `;`) explicitly denies issuance.
    let issuer = value.split(';').next()?.trim();
    crate::host::canonical_domain(issuer)
}

fn report_host(value: &str) -> Option<String> {
    if value.len() > MAX_CAA_VALUE_BYTES {
        return None;
    }
    if value
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("mailto:"))
    {
        return email_domain(&value[7..]);
    }

    let authority = value.split_once("://")?.1.split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains(['@', '\\']) {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    match url.host()? {
        Host::Domain(domain) => crate::host::canonical_domain(domain),
        Host::Ipv4(_) | Host::Ipv6(_) => None,
    }
}

fn email_domain(value: &str) -> Option<String> {
    let address = value
        .strip_prefix("mailto:")
        .or_else(|| {
            value
                .get(..7)
                .filter(|prefix| prefix.eq_ignore_ascii_case("mailto:"))
                .map(|_| &value[7..])
        })
        .unwrap_or(value);
    if address.is_empty() || address.len() > MAX_CAA_VALUE_BYTES || address.contains(['?', '#']) {
        return None;
    }
    let (local, domain) = address.rsplit_once('@')?;
    if local.is_empty()
        || domain.is_empty()
        || address.matches('@').count() != 1
        || address.bytes().any(|byte| byte.is_ascii_whitespace())
    {
        return None;
    }
    crate::host::canonical_domain(domain)
}
