//! Extracción offline de URI anunciadas por registros BIMI TXT importados.
//!
//! CHEF solo crea candidatos URL con evidencia del registro padre. No consulta
//! DNS, no descarga imágenes o certificados y no valida marcas ni certificados.

use std::collections::BTreeSet;

use serde_json::Value;
use url::{Host, Url};

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefResult, Clock, ModuleDescriptor,
};

const MAX_BIMI_RECORD_BYTES: usize = 64 * 1024;
const MAX_BIMI_TAGS: usize = 32;
const MAX_BIMI_URI_BYTES: usize = 255;

pub(crate) struct ChefBimiUrlHints;

impl BbotModule for ChefBimiUrlHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_bimi_url_hints",
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
        if event.data_redacted || !is_bimi_owner_name(event.host.as_deref().unwrap_or_default()) {
            return Ok(Vec::new());
        }
        let Some(data) = event.data.as_object() else {
            return Ok(Vec::new());
        };
        if !data
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case("TXT"))
        {
            return Ok(Vec::new());
        }
        if data
            .get("host")
            .and_then(Value::as_str)
            .is_some_and(|host| {
                !same_canonical_host(host, event.host.as_deref().unwrap_or_default())
            })
        {
            return Ok(Vec::new());
        }
        let Some(answer) = data.get("answer").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };

        Ok(parse_bimi_uris(answer)
            .into_iter()
            .map(|(uri, kind, host, port)| BbotEventDraft {
                event_type: "URL_HINT".to_owned(),
                data: Value::String(uri),
                host: Some(host),
                port,
                tags: vec![
                    "chef:derived-bimi-url-hint".to_owned(),
                    format!("chef:bimi-resource:{kind}"),
                    "chef:rule:bimi-url-hints:v1".to_owned(),
                ],
            })
            .collect())
    }
}

fn is_bimi_owner_name(value: &str) -> bool {
    canonical_bimi_owner_name(value).is_some()
}

fn canonical_bimi_owner_name(value: &str) -> Option<String> {
    let value = value.trim_end_matches('.');
    if value.is_empty() || value.len() > 253 || value.chars().any(char::is_control) {
        return None;
    }
    let host = value.to_ascii_lowercase();
    let labels = host.split('.').collect::<Vec<_>>();
    let namespace_index = labels.iter().position(|label| *label == "_bimi")?;
    if namespace_index == 0
        || labels.len().saturating_sub(namespace_index + 1) < 2
        || !labels[..namespace_index].iter().all(|selector| {
            (1..=63).contains(&selector.len())
                && selector
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !selector.starts_with('-')
                && !selector.ends_with('-')
        })
    {
        return None;
    }
    let domain = crate::host::canonical_domain(&labels[namespace_index + 1..].join("."))?;
    Some(format!(
        "{}._bimi.{domain}",
        labels[..namespace_index].join(".")
    ))
}

fn same_canonical_host(left: &str, right: &str) -> bool {
    canonical_bimi_owner_name(left) == canonical_bimi_owner_name(right)
}

fn parse_bimi_uris(record: &str) -> Vec<(String, &'static str, String, Option<u16>)> {
    if record.is_empty()
        || record.len() > MAX_BIMI_RECORD_BYTES
        || record.chars().any(char::is_control)
    {
        return Vec::new();
    }
    let tags = record.split(';').collect::<Vec<_>>();
    if tags.len() > MAX_BIMI_TAGS {
        return Vec::new();
    }
    let Some((version_name, version_value)) = tags.first().and_then(|tag| tag.split_once('='))
    else {
        return Vec::new();
    };
    if version_name.trim() != "v" || version_value.trim() != "BIMI1" {
        return Vec::new();
    }

    let mut location = None;
    let mut authority = None;
    for tag in tags
        .iter()
        .skip(1)
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
    {
        let Some((name, value)) = tag.split_once('=') else {
            continue;
        };
        let slot = if name.trim() == "l" {
            &mut location
        } else if name.trim() == "a" {
            &mut authority
        } else {
            continue;
        };
        if slot.is_some() {
            return Vec::new();
        }
        *slot = Some(value.trim());
    }

    if location.is_none() {
        return Vec::new();
    }

    let mut uris = BTreeSet::new();
    if let Some(value) = location.filter(|value| !value.is_empty())
        && let Some(parsed) = canonical_bimi_uri(value, false)
    {
        uris.insert((parsed.0, "location", parsed.1, parsed.2));
    }
    if let Some(value) = authority.filter(|value| !value.is_empty())
        && let Some(parsed) = canonical_bimi_uri(value, true)
    {
        uris.insert((parsed.0, "authority", parsed.1, parsed.2));
    }
    uris.into_iter().collect()
}

fn canonical_bimi_uri(
    value: &str,
    require_fully_qualified_domain: bool,
) -> Option<(String, String, Option<u16>)> {
    if value.is_empty()
        || value.len() > MAX_BIMI_URI_BYTES
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || crate::url_tools::has_userinfo(&url)
    {
        return None;
    }
    let host = match url.host()? {
        Host::Domain(domain) => {
            if require_fully_qualified_domain && !domain.contains('.') {
                return None;
            }
            crate::host::canonical_domain(domain)?
        }
        Host::Ipv4(address) if !require_fully_qualified_domain => address.to_string(),
        Host::Ipv6(address) if !require_fully_qualified_domain => address.to_string(),
        Host::Ipv4(_) | Host::Ipv6(_) => return None,
    };
    let port = url.port_or_known_default();
    url.set_query(None);
    url.set_fragment(None);
    Some((url.to_string(), host, port))
}
