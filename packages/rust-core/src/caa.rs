//! Interpretación offline de registros CAA ya importados.
//!
//! Las propiedades se conservan como pistas/evidencia de texto: CHEF no
//! consulta DNS, no valida una política de emisión ni contacta destinos IODEF.

use std::collections::BTreeSet;

use serde_json::Value;
use url::{Host, Url};

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefResult, Clock, ModuleDescriptor,
};

const MAX_CAA_RECORD_BYTES: usize = 64 * 1024;
const MAX_CAA_URI_BYTES: usize = 2_048;

pub(crate) struct ChefCaaRecordHints;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CaaUrlHint {
    url: String,
    host: String,
    port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CaaProperty {
    tag: String,
    value: String,
}

impl BbotModule for ChefCaaRecordHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_caa_record_hints",
            watches: &["RAW_DNS_RECORD"],
            produces: &["DNS_NAME_HINT", "EMAIL_ADDRESS", "URL_HINT"],
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
        if event.data_redacted {
            return Ok(Vec::new());
        }
        let Some(data) = event.data.as_object() else {
            return Ok(Vec::new());
        };
        if !data
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|value| value.eq_ignore_ascii_case("CAA"))
        {
            return Ok(Vec::new());
        }
        let Some(owner) = event
            .host
            .as_deref()
            .and_then(crate::host::canonical_domain)
        else {
            return Ok(Vec::new());
        };
        if data
            .get("host")
            .and_then(Value::as_str)
            .is_some_and(|host| crate::host::canonical_host(host) != Some(owner.clone()))
        {
            return Ok(Vec::new());
        }
        let Some(answer) = data.get("answer").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };
        let Some(property) = parse_caa_property(answer) else {
            return Ok(Vec::new());
        };

        match property.tag.as_str() {
            "issue" | "issuewild" => Ok(issue_issuer(&property.value)
                .into_iter()
                .map(|domain| BbotEventDraft {
                    event_type: "DNS_NAME_HINT".to_owned(),
                    data: Value::String(domain.clone()),
                    host: Some(domain),
                    port: None,
                    tags: vec![
                        "chef:derived-caa-issuer-hint".to_owned(),
                        format!("chef:caa-property:{}", property.tag),
                        "chef:rule:caa-record-hints:v1".to_owned(),
                    ],
                })
                .collect()),
            "iodef" => {
                let (emails, urls) = iodef_destinations(&property.value);
                let mut drafts = emails
                    .into_iter()
                    .map(|email| BbotEventDraft {
                        event_type: "EMAIL_ADDRESS".to_owned(),
                        data: Value::String(email),
                        host: None,
                        port: None,
                        tags: vec![
                            "chef:derived-caa-report-target".to_owned(),
                            "chef:caa-property:iodef".to_owned(),
                            "chef:rule:caa-record-hints:v1".to_owned(),
                        ],
                    })
                    .collect::<Vec<_>>();
                drafts.extend(urls.into_iter().map(|hint| BbotEventDraft {
                    event_type: "URL_HINT".to_owned(),
                    data: Value::String(hint.url),
                    host: Some(hint.host),
                    port: hint.port,
                    tags: vec![
                        "chef:derived-caa-report-target".to_owned(),
                        "chef:caa-property:iodef".to_owned(),
                        "chef:rule:caa-record-hints:v1".to_owned(),
                    ],
                }));
                Ok(drafts)
            }
            "contactemail" => Ok(valid_caa_email(&property.value)
                .into_iter()
                .map(|email| BbotEventDraft {
                    event_type: "EMAIL_ADDRESS".to_owned(),
                    data: Value::String(email),
                    host: None,
                    port: None,
                    tags: vec![
                        "chef:derived-caa-contact-email".to_owned(),
                        "chef:caa-property:contactemail".to_owned(),
                        "chef:rule:caa-record-hints:v1".to_owned(),
                    ],
                })
                .collect()),
            _ => Ok(Vec::new()),
        }
    }
}

fn parse_caa_property(record: &str) -> Option<CaaProperty> {
    if record.is_empty()
        || record.len() > MAX_CAA_RECORD_BYTES
        || record.chars().any(char::is_control)
    {
        return None;
    }
    let record = record.trim();
    let flags_end = record.find(char::is_whitespace)?;
    let flags = record[..flags_end].parse::<u8>().ok()?;
    let remainder = record[flags_end..].trim_start();
    let tag_end = remainder.find(char::is_whitespace)?;
    let tag = &remainder[..tag_end];
    let raw_value = remainder[tag_end..].trim();
    if !(1..=15).contains(&tag.len())
        || !tag.bytes().all(|byte| byte.is_ascii_alphanumeric())
        || raw_value.is_empty()
        || raw_value.contains('\\')
    {
        return None;
    }
    let value = if raw_value.starts_with('"') {
        if raw_value.len() < 2 || !raw_value.ends_with('"') {
            return None;
        }
        let value = &raw_value[1..raw_value.len() - 1];
        if value.contains('"') {
            return None;
        }
        value.to_owned()
    } else {
        if raw_value.chars().any(char::is_whitespace) || raw_value.contains('"') {
            return None;
        }
        raw_value.to_owned()
    };
    let _ = flags;
    Some(CaaProperty {
        tag: tag.to_ascii_lowercase(),
        value,
    })
}

fn issue_issuer(value: &str) -> Option<String> {
    let issuer = value.split(';').next()?.trim();
    if issuer.is_empty() {
        return None;
    }
    crate::host::canonical_domain(issuer).filter(|domain| domain.contains('.'))
}

fn iodef_destinations(value: &str) -> (Vec<String>, Vec<CaaUrlHint>) {
    if value.len() > MAX_CAA_URI_BYTES || value.chars().any(char::is_control) {
        return (Vec::new(), Vec::new());
    }
    let mut emails = BTreeSet::new();
    let mut urls = BTreeSet::new();
    for target in value
        .split(',')
        .map(str::trim)
        .filter(|target| !target.is_empty())
    {
        if let Some(email) = parse_mailto(target) {
            emails.insert(email);
        } else if let Some(url) = canonical_https_url(target) {
            urls.insert(url);
        }
    }
    (emails.into_iter().collect(), urls.into_iter().collect())
}

fn parse_mailto(value: &str) -> Option<String> {
    let (scheme, address) = value.split_once(':')?;
    if !scheme.eq_ignore_ascii_case("mailto")
        || address.contains(['?', '#', ',', '\\'])
        || valid_caa_email(address).is_none()
    {
        return None;
    }
    Some(address.to_owned())
}

fn valid_caa_email(value: &str) -> Option<String> {
    let (local, domain) = value.split_once('@')?;
    if !crate::email_export::valid_address(value)
        || !domain.contains('.')
        || crate::host::canonical_domain(domain).is_none()
        || local.is_empty()
    {
        return None;
    }
    Some(value.to_owned())
}

fn canonical_https_url(value: &str) -> Option<CaaUrlHint> {
    if value.len() > MAX_CAA_URI_BYTES || value.contains('\\') {
        return None;
    }
    let canonical = crate::url_tools::canonical_http_url(value)?;
    let mut url = Url::parse(&canonical).ok()?;
    if url.scheme() != "https" || crate::url_tools::has_userinfo(&url) {
        return None;
    }
    let host = match url.host()? {
        Host::Domain(domain) => crate::host::canonical_domain(domain)?,
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => address.to_string(),
    };
    let port = url.port_or_known_default();
    url.set_query(None);
    url.set_fragment(None);
    Some(CaaUrlHint {
        url: url.to_string(),
        host,
        port,
    })
}
