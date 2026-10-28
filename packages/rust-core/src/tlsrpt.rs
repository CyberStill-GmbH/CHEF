//! Extracción offline de destinos TLS-RPT desde registros TXT importados.
//!
//! Los correos y endpoints solo quedan como evidencia local con linaje. CHEF
//! no consulta DNS, envía informes ni realiza solicitudes HTTP.

use std::collections::BTreeSet;

use serde_json::Value;
use url::{Host, Url};

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefResult, Clock, ModuleDescriptor,
};

const MAX_TLSRPT_RECORD_BYTES: usize = 64 * 1024;
const MAX_TLSRPT_TAGS: usize = 32;
const MAX_REPORT_DESTINATIONS: usize = 32;
const MAX_REPORT_URI_BYTES: usize = 2_048;

pub(crate) struct ChefTlsRptTargetHints;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ReportUrlHint {
    url: String,
    host: String,
    port: Option<u16>,
}

impl BbotModule for ChefTlsRptTargetHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_tlsrpt_target_hints",
            watches: &["RAW_DNS_RECORD"],
            produces: &["EMAIL_ADDRESS", "URL_HINT"],
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
        if event.data_redacted || !is_tlsrpt_owner_name(event.host.as_deref().unwrap_or_default()) {
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

        let (emails, urls) = parse_tlsrpt_destinations(answer);
        let mut drafts = emails
            .into_iter()
            .map(|email| BbotEventDraft {
                event_type: "EMAIL_ADDRESS".to_owned(),
                data: Value::String(email),
                host: None,
                port: None,
                tags: vec![
                    "chef:derived-tlsrpt-report-target".to_owned(),
                    "chef:tlsrpt-target:mailto".to_owned(),
                    "chef:rule:tlsrpt-targets:v1".to_owned(),
                ],
            })
            .collect::<Vec<_>>();
        drafts.extend(urls.into_iter().map(|hint| BbotEventDraft {
            event_type: "URL_HINT".to_owned(),
            data: Value::String(hint.url),
            host: Some(hint.host),
            port: hint.port,
            tags: vec![
                "chef:derived-tlsrpt-report-target".to_owned(),
                "chef:tlsrpt-target:https".to_owned(),
                "chef:rule:tlsrpt-targets:v1".to_owned(),
            ],
        }));
        Ok(drafts)
    }
}

fn is_tlsrpt_owner_name(value: &str) -> bool {
    canonical_tlsrpt_owner_name(value).is_some()
}

fn canonical_tlsrpt_owner_name(value: &str) -> Option<String> {
    let value = value.trim_end_matches('.');
    if value.is_empty() || value.len() > 253 || value.chars().any(char::is_control) {
        return None;
    }
    let labels = value
        .to_ascii_lowercase()
        .split('.')
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if labels.len() < 4 || labels[0] != "_smtp" || labels[1] != "_tls" {
        return None;
    }
    let domain = crate::host::canonical_domain(&labels[2..].join("."))?;
    Some(format!("_smtp._tls.{domain}"))
}

fn same_canonical_host(left: &str, right: &str) -> bool {
    canonical_tlsrpt_owner_name(left) == canonical_tlsrpt_owner_name(right)
}

fn parse_tlsrpt_destinations(record: &str) -> (Vec<String>, Vec<ReportUrlHint>) {
    if record.is_empty()
        || record.len() > MAX_TLSRPT_RECORD_BYTES
        || record.chars().any(char::is_control)
    {
        return (Vec::new(), Vec::new());
    }
    let tags = record.split(';').collect::<Vec<_>>();
    if tags.len() > MAX_TLSRPT_TAGS {
        return (Vec::new(), Vec::new());
    }
    let Some((version_name, version_value)) = tags.first().and_then(|tag| tag.split_once('='))
    else {
        return (Vec::new(), Vec::new());
    };
    if version_name.trim() != "v" || version_value.trim() != "TLSRPTv1" {
        return (Vec::new(), Vec::new());
    }

    let mut rua = None;
    for tag in tags
        .iter()
        .skip(1)
        .map(|tag| tag.trim())
        .filter(|tag| !tag.is_empty())
    {
        let Some((name, value)) = tag.split_once('=') else {
            continue;
        };
        if name.trim() == "rua" {
            if rua.is_some() {
                return (Vec::new(), Vec::new());
            }
            rua = Some(value.trim());
        }
    }
    let Some(rua) = rua.filter(|value| !value.is_empty()) else {
        return (Vec::new(), Vec::new());
    };
    let destinations = rua
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    if destinations.len() > MAX_REPORT_DESTINATIONS {
        return (Vec::new(), Vec::new());
    }

    let mut emails = BTreeSet::new();
    let mut urls = BTreeSet::new();
    for destination in destinations {
        if let Some(address) = parse_mailto(destination) {
            emails.insert(address);
        } else if let Some(endpoint) = canonical_https_endpoint(destination) {
            urls.insert(endpoint);
        }
    }
    (emails.into_iter().collect(), urls.into_iter().collect())
}

fn parse_mailto(value: &str) -> Option<String> {
    let (scheme, address) = value.split_once(':')?;
    let (_, domain) = address.split_once('@')?;
    if !scheme.eq_ignore_ascii_case("mailto")
        || address.is_empty()
        || address.len() > MAX_REPORT_URI_BYTES
        || address.contains(['?', '#', ',', '\\'])
        || address.chars().any(char::is_control)
        || !domain.contains('.')
        || crate::host::canonical_domain(domain).is_none()
        || !crate::email_export::valid_address(address)
    {
        return None;
    }
    Some(address.to_owned())
}

fn canonical_https_endpoint(value: &str) -> Option<ReportUrlHint> {
    if value.len() > MAX_REPORT_URI_BYTES || value.contains('\\') {
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
    Some(ReportUrlHint {
        url: url.to_string(),
        host,
        port,
    })
}
