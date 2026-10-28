//! Extracción local de URL BIMI desde registros TXT ya importados.
//!
//! Adapta únicamente la parte de `modules/dnsbimi.py` que interpreta una
//! respuesta TXT. No consulta DNS, no descarga logo/certificado y no confirma
//! que los destinos estén disponibles. Cada salida es una pista enlazada al
//! `RAW_DNS_RECORD` que la contiene.

use serde_json::Value;
use url::Url;

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefError, ChefResult, Clock, ErrorCode,
    ModuleDescriptor,
};

const MAX_RDATA_BYTES: usize = 64 * 1024;
const MAX_BIMI_URL_BYTES: usize = 255;
const MAX_HINTS_PER_RECORD: usize = 2;

pub(crate) struct ChefBimiUrlHints;

#[derive(Clone, Copy)]
enum BimiRole {
    Location,
    Authority,
}

impl BimiRole {
    fn tag(self) -> &'static str {
        match self {
            Self::Location => "chef:bimi-location",
            Self::Authority => "chef:bimi-authority",
        }
    }
}

impl BbotModule for ChefBimiUrlHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_dns_bimi_hints",
            watches: &["RAW_DNS_RECORD"],
            produces: &["URL_HINT"],
        }
    }

    fn offline_only(&self) -> bool {
        true
    }

    fn handle(
        &self,
        event: &BbotEvent,
        _config: &ChefConfig,
        _clock: &dyn Clock,
    ) -> ChefResult<Vec<BbotEventDraft>> {
        if event.event_type != "RAW_DNS_RECORD" || event.data_redacted {
            return Ok(Vec::new());
        }
        let Some(data) = event.data.as_object() else {
            return Ok(Vec::new());
        };
        if !data
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|record_type| record_type.eq_ignore_ascii_case("TXT"))
            || !data
                .get("host")
                .and_then(Value::as_str)
                .is_some_and(is_bimi_owner)
        {
            return Ok(Vec::new());
        }
        let Some(answer) = data.get("answer").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };
        if answer.len() > MAX_RDATA_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El dato RDATA BIMI supera el máximo CHEF de bytes",
            ));
        }
        if answer.chars().any(char::is_control) {
            return Ok(Vec::new());
        }

        let Some((location, authority)) = parse_bimi_record(answer) else {
            return Ok(Vec::new());
        };
        let mut drafts = Vec::with_capacity(MAX_HINTS_PER_RECORD);
        for (role, value) in [
            (BimiRole::Location, location),
            (BimiRole::Authority, authority),
        ] {
            let Some(value) = value else {
                continue;
            };
            let Some((safe_url, host, port)) = checked_bimi_url(value, role) else {
                continue;
            };
            drafts.push(BbotEventDraft {
                event_type: "URL_HINT".to_owned(),
                data: Value::String(safe_url),
                host: Some(host),
                port,
                tags: vec![role.tag().to_owned()],
            });
        }
        debug_assert!(drafts.len() <= MAX_HINTS_PER_RECORD);
        Ok(drafts)
    }
}

/// Valida el owner `<selector>._bimi.<root>` sin exigir que el root sea público:
/// los registros importados pueden pertenecer a entornos de laboratorio.
fn is_bimi_owner(value: &str) -> bool {
    let owner = match value.strip_suffix('.') {
        Some(owner) if !owner.ends_with('.') => owner,
        Some(_) => return false,
        None => value,
    };
    if owner.is_empty() || owner.len() > 253 || owner.chars().any(char::is_control) {
        return false;
    }
    let labels = owner.split('.').collect::<Vec<_>>();
    if labels.len() < 4 || !labels[1].eq_ignore_ascii_case("_bimi") {
        return false;
    }
    let selector = labels[0];
    if selector.is_empty()
        || selector.len() > 63
        || !selector.as_bytes()[0].is_ascii_alphanumeric()
        || !selector.as_bytes()[selector.len() - 1].is_ascii_alphanumeric()
        || !selector
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return false;
    }
    let root = labels[2..].join(".");
    !root
        .split('.')
        .any(|label| label.eq_ignore_ascii_case("_wildcard"))
        && crate::host::canonical_domain(&root).is_some()
}

/// Implementa la gramática acotada de la expresión regular fuente: versión
/// BIMI1, `l=` opcional y después `a=` opcional, con separadores opcionales.
/// El origen acepta HTTP(S) para `l`, solo HTTPS para `a`; valores vacíos son
/// declinaciones y no generan pistas. La adaptación limita la URL completa a
/// 255 bytes (más estricta que el máximo del grupo URL de la regex fuente).
fn parse_bimi_record(answer: &str) -> Option<(Option<&str>, Option<&str>)> {
    let answer = answer.trim();
    let (version, mut rest) = answer.split_once(';')?;
    if !version.eq_ignore_ascii_case("v=BIMI1") {
        return None;
    }
    rest = consume_optional_space(rest);

    let location = if let Some(after_key) = rest.strip_prefix("l=") {
        let (value, remainder) = read_optional_url_value(after_key, false)?;
        rest = remainder;
        value
    } else {
        None
    };

    if let Some(remainder) = rest.strip_prefix(';') {
        rest = remainder;
    }
    rest = consume_optional_space(rest);

    let authority = if let Some(after_key) = rest.strip_prefix("a=") {
        let (value, remainder) = read_optional_url_value(after_key, true)?;
        rest = remainder;
        value
    } else {
        None
    };

    if let Some(remainder) = rest.strip_prefix(';') {
        rest = remainder;
    }
    rest.is_empty().then_some((location, authority))
}

fn read_optional_url_value(rest: &str, https_only: bool) -> Option<(Option<&str>, &str)> {
    if rest.is_empty() || rest.starts_with(';') || rest.starts_with(' ') || rest.starts_with("a=") {
        return Some((None, rest));
    }
    let end = rest.find([';', ' ']).unwrap_or(rest.len());
    let value = &rest[..end];
    if value.is_empty()
        || value.len() > MAX_BIMI_URL_BYTES
        || value.chars().any(char::is_whitespace)
        || !(if https_only {
            value
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        } else {
            value
                .get(..7)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
                || value
                    .get(..8)
                    .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"))
        })
    {
        return None;
    }
    Some((Some(value), &rest[end..]))
}

fn consume_optional_space(value: &str) -> &str {
    value.strip_prefix(' ').unwrap_or(value)
}

fn checked_bimi_url(value: &str, role: BimiRole) -> Option<(String, String, Option<u16>)> {
    if value.is_empty()
        || value.len() > MAX_BIMI_URL_BYTES
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || (matches!(role, BimiRole::Authority)
            && !value
                .get(..8)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://")))
        || has_suspicious_path(value)
    {
        return None;
    }

    // CHEF canonicaliza URL_HINT con la misma política que el motor: HTTP(S),
    // host requerido, sin userinfo, y elimina query/fragmento.
    let safe_url = crate::url_tools::canonical_http_url(value)?;
    if safe_url.len() > MAX_BIMI_URL_BYTES {
        return None;
    }
    let parsed = Url::parse(&safe_url).ok()?;
    let host = parsed.host()?.to_string();
    let port = parsed.port_or_known_default();
    if port == Some(0) {
        return None;
    }
    Some((safe_url, host, port))
}

/// Rechaza rutas ambiguas que podrían cambiar de recurso al interpretarse:
/// barras repetidas, segmentos `.`/`..`, separadores codificados, controles o
/// escapes porcentuales inválidos. La query y el fragmento se sanea después.
fn has_suspicious_path(value: &str) -> bool {
    let Some(scheme_end) = value.find("://") else {
        return true;
    };
    let after_scheme = &value[scheme_end + 3..];
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let path_and_suffix = &after_scheme[authority_end..];
    let path = path_and_suffix.split(['?', '#']).next().unwrap_or_default();
    if path.is_empty() {
        return false;
    }
    if !path.starts_with('/')
        || path.starts_with("//")
        || path.contains("//")
        || path.contains('\\')
    {
        return true;
    }
    for segment in path.split('/').skip(1) {
        let Some(decoded) = decode_path_segment(segment) else {
            return true;
        };
        if decoded == b"." || decoded == b".." {
            return true;
        }
        if decoded
            .iter()
            .any(|byte| *byte == b'/' || *byte == b'\\' || *byte < 0x20 || *byte == 0x7f)
        {
            return true;
        }
    }
    false
}

fn decode_path_segment(value: &str) -> Option<Vec<u8>> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((hex_value(high)? << 4) | hex_value(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    Some(decoded)
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
