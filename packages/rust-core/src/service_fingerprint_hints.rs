//! Adapta a pistas el JSONL archivado de un reconocedor de servicios.
//!
//! Este módulo interpreta resultados ya guardados; no inicia procesos, sondea
//! puertos ni convierte los resultados en observaciones actuales.

use std::{collections::HashSet, net::IpAddr};

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, escape_terminal_controls, host::canonical_host};

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_RECORDS: usize = 1024;
const MAX_BANNER_BYTES: usize = 4096;
const MAX_BANNER_CHARS: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ServiceFingerprintHint {
    pub host: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub banner: Option<String>,
    /// URL candidata para HTTP(S), sin afirmar disponibilidad del servicio.
    pub url_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedServiceFingerprintHints {
    pub hints: Vec<ServiceFingerprintHint>,
}

/// Interpreta líneas JSON archivadas del reconocedor de servicios.
///
/// Se conservan solo registros con host y protocolo válidos. Los banners se
/// limitan y escapan para mostrarlos como evidencia no confiable.
pub fn parse_archived_service_fingerprints(
    input: &[u8],
) -> ChefResult<ArchivedServiceFingerprintHints> {
    if input.len() > MAX_INPUT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La salida de reconocimiento archivada supera el límite de bytes",
        ));
    }

    let mut hints = Vec::new();
    let mut hints_seen = HashSet::new();
    let mut records_seen = 0;
    for line in input.split(|byte| *byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        records_seen += 1;
        if records_seen > MAX_RECORDS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La salida de reconocimiento supera el máximo de registros",
            ));
        }
        let record: Value = serde_json::from_slice(line).map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La salida archivada contiene una línea JSON inválida",
            )
        })?;
        let Some(record) = record.as_object() else {
            continue;
        };

        let host_value = record
            .get("host")
            .and_then(Value::as_str)
            .or_else(|| record.get("ip").and_then(Value::as_str))
            .unwrap_or_default();
        let Some(host) = canonical_host(host_value) else {
            continue;
        };
        let Some(protocol) = record
            .get("protocol")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| valid_protocol(value))
        else {
            continue;
        };
        let port = parse_port(record.get("port"));
        let banner = record
            .get("metadata")
            .and_then(Value::as_object)
            .and_then(|metadata| metadata.get("banner"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| escape_terminal_controls(&truncate_banner(value)));
        let protocol = protocol.to_ascii_uppercase();
        let url_hint = make_url_hint(&host, port, &protocol);
        let hint = ServiceFingerprintHint {
            host,
            port,
            protocol,
            banner,
            url_hint,
        };
        if hints_seen.insert(hint.clone()) {
            hints.push(hint);
        }
    }

    Ok(ArchivedServiceFingerprintHints { hints })
}

fn truncate_banner(value: &str) -> String {
    let mut bounded = String::new();
    for (char_count, character) in value.chars().enumerate() {
        if char_count == MAX_BANNER_CHARS || bounded.len() + character.len_utf8() > MAX_BANNER_BYTES
        {
            break;
        }
        bounded.push(character);
    }
    bounded
}

fn parse_port(value: Option<&Value>) -> Option<u16> {
    match value? {
        Value::Number(number) => number
            .as_u64()
            .and_then(|value| u16::try_from(value).ok())
            .filter(|value| *value > 0),
        Value::String(value) => value.parse::<u16>().ok().filter(|value| *value > 0),
        _ => None,
    }
}

fn valid_protocol(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn make_url_hint(host: &str, port: Option<u16>, protocol: &str) -> Option<String> {
    let scheme = match protocol {
        "HTTP" => "http",
        "HTTPS" => "https",
        _ => return None,
    };
    let host_for_url = match host.parse::<IpAddr>() {
        Ok(IpAddr::V6(_)) => format!("[{host}]"),
        _ => host.to_owned(),
    };
    let include_port =
        port.filter(|port| !matches!((scheme, *port), ("http", 80) | ("https", 443)));
    let authority = include_port.map_or(host_for_url.clone(), |port| {
        format!("{host_for_url}:{port}")
    });
    Url::parse(&format!("{scheme}://{authority}/"))
        .ok()
        .map(|url| url.to_string().trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_service_fingerprints;
    use crate::ErrorCode;

    #[test]
    fn parses_archived_protocol_records_and_http_hints() {
        let input = br#"{"host":"example.com","port":443,"protocol":"https","metadata":{"banner":"server\u001b[31m"}}
{"ip":"192.0.2.10","port":"22","protocol":"ssh"}
"#;
        let result = parse_archived_service_fingerprints(input).unwrap();

        assert_eq!(result.hints.len(), 2);
        assert_eq!(result.hints[0].protocol, "HTTPS");
        assert_eq!(
            result.hints[0].url_hint.as_deref(),
            Some("https://example.com")
        );
        assert_eq!(result.hints[0].banner.as_deref(), Some("server\\x1b[31m"));
        assert_eq!(result.hints[1].url_hint, None);
    }

    #[test]
    fn rejects_malformed_hosts_protocols_and_ports() {
        let input = br#"{"host":"user@example.com","port":80,"protocol":"HTTP"}
{"host":"example.com","port":70000,"protocol":"HTTP"}
{"host":"example.com","port":443,"protocol":"HTTPS"}
"#;
        let result = parse_archived_service_fingerprints(input).unwrap();
        assert_eq!(result.hints.len(), 2);
        assert_eq!(result.hints[0].port, None);
        assert_eq!(result.hints[1].port, Some(443));
    }

    #[test]
    fn deduplicates_identical_archived_hints_but_preserves_distinct_banners() {
        let input =
            br#"{"host":"example.com","port":443,"protocol":"https","metadata":{"banner":"server"}}
{"host":"example.com","port":443,"protocol":"HTTPS","metadata":{"banner":"server"}}
{"host":"example.com","port":443,"protocol":"https","metadata":{"banner":"different"}}
"#;
        let result = parse_archived_service_fingerprints(input).unwrap();
        assert_eq!(result.hints.len(), 2);
        assert_eq!(result.hints[0].banner.as_deref(), Some("server"));
        assert_eq!(result.hints[1].banner.as_deref(), Some("different"));
    }

    #[test]
    fn enforces_input_record_and_json_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_service_fingerprints(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many = (0..1025)
            .map(|_| r#"{"host":"example.com","protocol":"ssh"}"#)
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            parse_archived_service_fingerprints(too_many.as_bytes())
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_service_fingerprints(b"{not-json}")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
