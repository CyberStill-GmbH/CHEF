//! Pistas locales de políticas TLS-RPT desde registros TXT ya importados.
//!
//! Interpreta solo `_smtp._tls.<dominio>` con `v=TLSRPTvN` y URIs `rua`.
//! No consulta DNS ni HTTP; reduce correos al dominio y URLs al origen.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefResult, Clock, ModuleDescriptor,
};

const MAX_TLSRPT_RDATA_BYTES: usize = 64 * 1024;
const MAX_TLSRPT_URI_BYTES: usize = 2048;
const MAX_TLSRPT_HINTS_PER_RECORD: usize = 32;

pub(crate) struct ChefDnsTlsRptHints;

impl BbotModule for ChefDnsTlsRptHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_dns_tlsrpt_hints",
            watches: &["RAW_DNS_RECORD"],
            produces: &["DNS_NAME_HINT", "URL_HINT"],
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
                .is_some_and(is_tlsrpt_owner)
        {
            return Ok(Vec::new());
        }
        let Some(answer) = data.get("answer").and_then(Value::as_str) else {
            return Ok(Vec::new());
        };
        let Some(uris) = parse_report_uris(answer) else {
            return Ok(Vec::new());
        };

        let mut drafts = Vec::new();
        let mut emitted = BTreeSet::new();
        for uri in uris {
            if let Some(domain) = mailto_domain(&uri) {
                if emitted.insert(("DNS_NAME_HINT", domain.clone())) {
                    drafts.push(BbotEventDraft {
                        event_type: "DNS_NAME_HINT".to_owned(),
                        data: Value::String(domain.clone()),
                        host: Some(domain),
                        port: None,
                        tags: vec![
                            "chef:derived-dns-tlsrpt-hint".to_owned(),
                            "chef:dns-tlsrpt:report-email-domain".to_owned(),
                        ],
                    });
                }
            } else if let Some((origin, host, port)) = report_url_origin(&uri)
                && emitted.insert(("URL_HINT", origin.clone()))
            {
                drafts.push(BbotEventDraft {
                    event_type: "URL_HINT".to_owned(),
                    data: Value::String(origin),
                    host: Some(host),
                    port,
                    tags: vec![
                        "chef:derived-dns-tlsrpt-hint".to_owned(),
                        "chef:dns-tlsrpt:report-url".to_owned(),
                    ],
                });
            }
            if drafts.len() >= MAX_TLSRPT_HINTS_PER_RECORD {
                break;
            }
        }
        Ok(drafts)
    }
}

pub(crate) fn is_generated_dns_tlsrpt_hint(event: &BbotEvent) -> bool {
    event.event_type == "DNS_NAME_HINT"
        && event.module.as_deref() == Some("chef_dns_tlsrpt_hints")
        && event
            .tags
            .iter()
            .any(|tag| tag == "chef:derived-dns-tlsrpt-hint")
}

fn is_tlsrpt_owner(value: &str) -> bool {
    let owner = match value.strip_suffix('.') {
        Some(owner) if !owner.ends_with('.') => owner,
        Some(_) => return false,
        None => value,
    };
    if owner.is_empty() || owner.len() > 253 || owner.chars().any(char::is_control) {
        return false;
    }
    let labels = owner.split('.').collect::<Vec<_>>();
    if labels.len() < 4
        || !labels[0].eq_ignore_ascii_case("_smtp")
        || !labels[1].eq_ignore_ascii_case("_tls")
    {
        return false;
    }
    let root = labels[2..].join(".");
    !root
        .split('.')
        .any(|label| label.eq_ignore_ascii_case("_wildcard"))
        && crate::host::canonical_domain(&root).is_some()
}

fn parse_report_uris(answer: &str) -> Option<Vec<String>> {
    if answer.len() > MAX_TLSRPT_RDATA_BYTES || answer.chars().any(char::is_control) {
        return None;
    }
    let record = answer.trim();
    let record = match record.strip_prefix('"') {
        Some(quoted) => quoted.strip_suffix('"')?,
        None if record.contains('"') => return None,
        None => record,
    };
    let (version, parameters) = record.split_once(';')?;
    let (key, version) = version.trim().split_once('=')?;
    let version = version.trim();
    if !key.trim().eq_ignore_ascii_case("v")
        || !version
            .strip_prefix("TLSRPTv")
            .or_else(|| {
                version
                    .get(..7)
                    .filter(|prefix| prefix.eq_ignore_ascii_case("TLSRPTv"))
                    .map(|_| &version[7..])
            })
            .is_some_and(|digits| {
                !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
            })
    {
        return None;
    }

    let mut uris = BTreeSet::new();
    for parameter in parameters.split(';') {
        let Some((name, value)) = parameter.trim().split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("rua") {
            continue;
        }
        for uri in
            value.split(|character: char| character == ',' || character.is_ascii_whitespace())
        {
            let uri = uri.trim();
            if uri.is_empty()
                || uri.len() > MAX_TLSRPT_URI_BYTES
                || uri.chars().any(char::is_control)
            {
                continue;
            }
            if has_case_insensitive_prefix(uri, "mailto:")
                || has_case_insensitive_prefix(uri, "http://")
                || has_case_insensitive_prefix(uri, "https://")
            {
                uris.insert(uri.to_owned());
            }
        }
    }
    Some(uris.into_iter().take(MAX_TLSRPT_HINTS_PER_RECORD).collect())
}

fn has_case_insensitive_prefix(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn mailto_domain(uri: &str) -> Option<String> {
    if !has_case_insensitive_prefix(uri, "mailto:") {
        return None;
    }
    let address = &uri[7..];
    let address = address.split(['?', '#']).next()?;
    let (local, domain) = address.rsplit_once('@')?;
    if local.is_empty()
        || local.len() > 64
        || local.starts_with('.')
        || local.ends_with('.')
        || local.contains("..")
        || domain.is_empty()
        || address.matches('@').count() != 1
        || address.bytes().any(|byte| byte.is_ascii_whitespace())
        || !local
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&byte))
    {
        return None;
    }
    crate::host::canonical_domain(domain)
}

fn report_url_origin(uri: &str) -> Option<(String, String, Option<u16>)> {
    if !has_case_insensitive_prefix(uri, "http://") && !has_case_insensitive_prefix(uri, "https://")
    {
        return None;
    }
    let origin = crate::url_tools::canonical_http_origin(uri)?;
    let parsed = Url::parse(&origin).ok()?;
    Some((
        origin,
        parsed.host_str()?.to_owned(),
        parsed.port_or_known_default(),
    ))
}
