//! Reglas locales de clasificación para tipos de registro DNS BBOT.
//!
//! No resuelve dominios ni crea observaciones; clasifica nombres/RR types ya
//! recibidos como entrada o evidencia importada.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RECORD_TEXT_BYTES: usize = 64 * 1024;
const SUPPORTED_RDATA_TYPES: [&str; 11] = [
    "A", "AAAA", "CNAME", "NS", "PTR", "ANAME", "MX", "SRV", "SOA", "NAPTR", "TXT",
];

/// Tipos que BBOT incluye en su conjunto base de consultas recursivas.
pub const DNS_RECORD_TYPES: [&str; 8] = ["A", "AAAA", "SRV", "MX", "NS", "SOA", "CNAME", "TXT"];

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DnsTarget {
    pub record_type: String,
    pub value: String,
}

/// Indica si el nombre corresponde a un registro de configuración/servicio.
///
/// Adapta `service_record` de BBOT para RR `SRV` y etiquetas comunes de correo,
/// federación, descubrimiento de servicios y políticas DNS.
pub fn is_dns_service_record(host: &str, record_type: Option<&str>) -> bool {
    if record_type.is_some_and(|value| value.eq_ignore_ascii_case("SRV")) {
        return true;
    }

    let host = host.strip_suffix('.').unwrap_or(host);
    let mut labels = host.split('.');
    let Some(first) = labels.next() else {
        return false;
    };
    let Some(second) = labels.next() else {
        return first.eq_ignore_ascii_case("_dmarc") || first.eq_ignore_ascii_case("_mta-sts");
    };

    first.eq_ignore_ascii_case("_dmarc")
        || first.eq_ignore_ascii_case("_mta-sts")
        || ["_udp", "_tcp", "_tls", "_bimi", "_domainkey"]
            .iter()
            .any(|marker| second.eq_ignore_ascii_case(marker))
}

/// Extrae nombres, IP y CIDR literales de texto RDATA ya obtenido.
///
/// La extracción de nombres en TXT se limita a candidatos DNS válidos; SPF
/// `ip4:`/`ip6:` literales se canonicalizan. Macros SPF se ignoran porque su
/// valor depende de evaluación en tiempo de consulta.
pub fn extract_dns_targets(record_type: &str, record_text: &str) -> ChefResult<Vec<DnsTarget>> {
    if record_text.len() > MAX_RECORD_TEXT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El dato RDATA DNS supera el máximo CHEF de bytes",
        ));
    }
    let record_type = record_type.trim().to_ascii_uppercase();
    if !SUPPORTED_RDATA_TYPES.contains(&record_type.as_str()) {
        return Ok(Vec::new());
    }

    let mut targets = BTreeSet::new();
    if matches!(record_type.as_str(), "A" | "AAAA") {
        if let Ok(address) = record_text.trim().parse::<std::net::IpAddr>() {
            let matches_type = matches!(
                (&record_type[..], address),
                ("A", std::net::IpAddr::V4(_)) | ("AAAA", std::net::IpAddr::V6(_))
            );
            if matches_type {
                targets.insert(DnsTarget {
                    record_type,
                    value: address.to_string(),
                });
            }
        }
        return Ok(targets.into_iter().collect());
    }

    if record_type == "TXT" {
        let is_spf = record_text.to_ascii_lowercase().contains("v=spf1");
        if is_spf {
            for token in record_text.split_whitespace() {
                if token.contains('%') {
                    continue;
                }
                let mechanism = token
                    .trim_start_matches(['+', '-', '~', '?'])
                    .split_once(':');
                let Some((kind, value)) = mechanism else {
                    continue;
                };
                if !kind.eq_ignore_ascii_case("ip4") && !kind.eq_ignore_ascii_case("ip6") {
                    continue;
                }
                if let Some(value) = canonical_spf_address(kind, value) {
                    targets.insert(DnsTarget {
                        record_type: record_type.clone(),
                        value,
                    });
                }
            }
        }
    }

    let target_text = match record_type.as_str() {
        "CNAME" | "NS" | "PTR" | "ANAME" => record_text.trim().to_owned(),
        "MX" | "SRV" | "NAPTR" => record_text
            .split_whitespace()
            .last()
            .unwrap_or_default()
            .to_owned(),
        "SOA" => record_text
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_owned(),
        "TXT" if record_text.contains('%') => record_text
            .split_whitespace()
            .filter(|token| !token.contains('%'))
            .collect::<Vec<_>>()
            .join(" "),
        "TXT" => record_text.to_owned(),
        _ => String::new(),
    };
    if record_type != "TXT" {
        if let Some(value) = canonical_dns_candidate(&target_text) {
            targets.insert(DnsTarget { record_type, value });
        }
        return Ok(targets.into_iter().collect());
    }
    for candidate in dns_candidates(&target_text) {
        if let Some(value) = canonical_dns_candidate(candidate) {
            targets.insert(DnsTarget {
                record_type: record_type.clone(),
                value,
            });
        }
    }
    Ok(targets.into_iter().collect())
}

fn canonical_spf_address(kind: &str, value: &str) -> Option<String> {
    let value = value
        .trim_matches(|character: char| matches!(character, '"' | '\'' | ')' | ']' | ',' | ';'));
    if value.contains('/') {
        let network = crate::target_seed::canonical_network(value)?;
        let is_v4 = network.parse::<std::net::Ipv4Addr>().is_ok()
            || network
                .split_once('/')
                .is_some_and(|(address, _)| address.parse::<std::net::Ipv4Addr>().is_ok());
        return (kind.eq_ignore_ascii_case("ip4") == is_v4).then_some(network);
    }
    let address = value.parse::<std::net::IpAddr>().ok()?;
    let is_v4 = matches!(address, std::net::IpAddr::V4(_));
    (kind.eq_ignore_ascii_case("ip4") == is_v4).then(|| address.to_string())
}

fn dns_candidates(text: &str) -> impl Iterator<Item = &str> {
    let mut ranges = Vec::new();
    let mut start = None;
    for (index, character) in text.char_indices() {
        let is_name_character = character.is_alphanumeric() || matches!(character, '.' | '-' | '_');
        if is_name_character {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take()
            && text[begin..index].contains('.')
        {
            ranges.push((begin, index));
        }
    }
    if let Some(begin) = start
        && text[begin..].contains('.')
    {
        ranges.push((begin, text.len()));
    }
    ranges.into_iter().map(|(begin, end)| &text[begin..end])
}

fn canonical_dns_candidate(candidate: &str) -> Option<String> {
    let candidate = candidate.trim_matches('.');
    if candidate.is_empty() || candidate.len() > 253 {
        return None;
    }
    if candidate.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    if let Some(domain) = crate::host::canonical_domain(candidate) {
        return Some(domain);
    }
    let labels = candidate.split('.').collect::<Vec<_>>();
    if labels.len() < 2
        || labels.iter().any(|label| {
            label.is_empty()
                || label.len() > 63
                || !label.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_')
                })
        })
    {
        return None;
    }
    Some(candidate.to_ascii_lowercase())
}
