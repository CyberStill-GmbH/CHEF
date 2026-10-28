//! Adaptaciones offline CHEF de ramas útiles de `internal.speculate` de BBOT.
//! Las salidas son pistas o miembros derivados con linaje; no se hace DNS ni
//! se visitan endpoints, y el módulo no tiene acceso a red.

use serde_json::Value;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use url::{Host, Url};

use crate::{
    BbotEvent, BbotEventDraft, BbotModule, ChefConfig, ChefResult, Clock, ModuleDescriptor,
};

const MAX_URL_PARENT_HINTS: usize = 32;
const MAX_IP_RANGE_MEMBERS: usize = 256;
const MAX_ORG_STUBS_PER_EVENT: usize = 64;

pub(crate) struct ChefUrlParentHints;
pub(crate) struct ChefUrlAttributeHints;
pub(crate) struct ChefDnsParentHints;
pub(crate) struct ChefCommonPortHints;
pub(crate) struct ChefOrgStubHints;

/// Adaptación offline y acotada del puente IP_RANGE -> IP_ADDRESS de BBOT.
/// No resuelve nombres ni abre conexiones; cada IP conserva el evento CIDR
/// como padre. La derivación no concede autorización para consultar esa IP.
pub(crate) struct ChefIpRangeExpansion;

impl BbotModule for ChefIpRangeExpansion {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_ip_range_expansion",
            watches: &["IP_RANGE"],
            produces: &["IP_ADDRESS"],
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
        let Some(value) = event.data.as_str() else {
            return Ok(Vec::new());
        };
        let Some(addresses) = bounded_network_members(value) else {
            return Ok(Vec::new());
        };
        Ok(addresses
            .into_iter()
            .map(|address| BbotEventDraft {
                event_type: "IP_ADDRESS".to_owned(),
                data: Value::String(address.to_string()),
                host: Some(address.to_string()),
                port: None,
                tags: vec!["chef:derived-ip-range-member".to_owned()],
            })
            .collect())
    }
}

/// Adapta el caso de BBOT que obtiene un campo `url` de otros eventos. CHEF
/// publica solo una pista no verificada; nunca visita el destino ni retiene
/// query o fragmento, que pueden contener datos sensibles.
impl BbotModule for ChefUrlAttributeHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_url_attribute_hints",
            watches: &["*"],
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
        if matches!(
            event.event_type.as_str(),
            "URL_UNVERIFIED" | "URL_HINT" | "HTTP_RESPONSE"
        ) {
            return Ok(Vec::new());
        }
        let value = if event.event_type == "URL" {
            event.data.as_str()
        } else {
            event
                .data
                .as_object()
                .and_then(|data| data.get("url"))
                .and_then(Value::as_str)
        };
        let Some(value) = value else {
            return Ok(Vec::new());
        };
        let Some((url, host, port)) = checked_url_attribute(value) else {
            return Ok(Vec::new());
        };
        Ok(vec![BbotEventDraft {
            event_type: "URL_HINT".to_owned(),
            data: Value::String(url),
            host: Some(host),
            port,
            tags: vec!["chef:derived-url-attribute-hint".to_owned()],
        }])
    }
}

/// Adapta `internal.speculate` de BBOT para derivar dominios padre como
/// candidatos sin resolución DNS. Solo usa la sección ICANN de la lista PSL
/// integrada; dominios de sufijos privados o desconocidos no se expanden.
pub(crate) fn derive_dns_parent_hints(value: &str) -> Vec<String> {
    crate::host::domain_parent_candidates(value, false)
}

impl BbotModule for ChefDnsParentHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_dns_parent_hints",
            watches: &["DNS_NAME", "DNS_NAME_UNRESOLVED"],
            produces: &["DNS_NAME_UNRESOLVED"],
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
        let Some(value) = event.data.as_str() else {
            return Ok(Vec::new());
        };
        Ok(derive_dns_parent_hints(value)
            .into_iter()
            .map(|parent| BbotEventDraft {
                event_type: "DNS_NAME_UNRESOLVED".to_owned(),
                data: Value::String(parent.clone()),
                host: Some(parent),
                port: None,
                tags: vec!["chef:derived-parent-domain-hint".to_owned()],
            })
            .collect())
    }
}

/// Traduce la especulación BBOT de puertos comunes a candidatos CHEF. Las
/// salidas `TCP_PORT_HINT` nunca significan que el puerto esté abierto.
pub(crate) const DEFAULT_HINT_PORTS: [u16; 2] = [80, 443];

impl BbotModule for ChefCommonPortHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_common_port_hints",
            watches: &[
                "DNS_NAME",
                "DNS_NAME_UNRESOLVED",
                "IP_ADDRESS",
                "URL",
                "URL_UNVERIFIED",
            ],
            produces: &["TCP_PORT_HINT"],
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
        if matches!(event.event_type.as_str(), "URL" | "URL_UNVERIFIED") {
            let Some(value) = event.data.as_str() else {
                return Ok(Vec::new());
            };
            let Some((_, host, Some(port))) = checked_url_attribute(value) else {
                return Ok(Vec::new());
            };
            return Ok(vec![BbotEventDraft {
                event_type: "TCP_PORT_HINT".to_owned(),
                data: Value::String(format_netloc(&host, port)),
                host: Some(host),
                port: Some(port),
                tags: vec!["chef:derived-url-port-hint".to_owned()],
            }]);
        }
        let candidate = event
            .host
            .as_deref()
            .or_else(|| event.data.as_str())
            .and_then(crate::host::canonical_host);
        let Some(host) = candidate else {
            return Ok(Vec::new());
        };
        Ok(DEFAULT_HINT_PORTS
            .into_iter()
            .map(|port| BbotEventDraft {
                event_type: "TCP_PORT_HINT".to_owned(),
                data: Value::String(format_netloc(&host, port)),
                host: Some(host.clone()),
                port: Some(port),
                tags: vec!["chef:derived-common-port-hint".to_owned()],
            })
            .collect())
    }
}

/// Deriva nombres de organización candidatos a partir de dominios raíz,
/// perfiles sociales y nombres de tenant Azure. No afirma propiedad ni hace
/// consultas externas; el evento hijo conserva la procedencia del origen.
impl BbotModule for ChefOrgStubHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_org_stub_hints",
            watches: &["DNS_NAME", "SOCIAL", "AZURE_TENANT"],
            produces: &["ORG_STUB"],
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
        let mut stubs = std::collections::BTreeSet::new();
        match event.event_type.as_str() {
            "DNS_NAME" if event.scope_distance == 0 => {
                if let Some(domain) = event.data.as_str().and_then(crate::host::canonical_domain)
                    && let Some(registrable) = psl::domain(domain.as_bytes())
                    && registrable.suffix().typ() == Some(psl::Type::Icann)
                    && let Some(label) = String::from_utf8_lossy(registrable.as_bytes())
                        .split('.')
                        .next()
                        .map(str::to_owned)
                {
                    let (decoded_label, decode_result) = idna::domain_to_unicode(&label);
                    let org_stub = if decode_result.is_ok() {
                        decoded_label
                    } else {
                        label
                    };
                    add_org_stub(&mut stubs, &org_stub);
                    add_org_stub(&mut stubs, &deunicode::deunicode(&org_stub));
                }
            }
            "SOCIAL" => {
                if let Some(stub) = event
                    .data
                    .as_object()
                    .and_then(|data| data.get("stub"))
                    .and_then(Value::as_str)
                {
                    add_org_stub(&mut stubs, stub);
                }
            }
            "AZURE_TENANT" => {
                if let Some(names) = event
                    .data
                    .as_object()
                    .and_then(|data| data.get("tenant-names"))
                    .and_then(Value::as_array)
                {
                    for name in names.iter().filter_map(Value::as_str) {
                        add_org_stub(&mut stubs, name);
                        if stubs.len() >= MAX_ORG_STUBS_PER_EVENT {
                            break;
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(stubs
            .into_iter()
            .take(MAX_ORG_STUBS_PER_EVENT)
            .map(|stub| BbotEventDraft {
                event_type: "ORG_STUB".to_owned(),
                data: Value::String(stub),
                host: event.host.clone(),
                port: None,
                tags: vec!["chef:derived-org-stub-hint".to_owned()],
            })
            .collect())
    }
}

fn add_org_stub(stubs: &mut std::collections::BTreeSet<String>, value: &str) {
    let value = value.trim().to_lowercase();
    if !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control) {
        stubs.insert(value);
    }
}

fn format_netloc(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn checked_url_attribute(value: &str) -> Option<(String, String, Option<u16>)> {
    if value.is_empty()
        || value.len() > 8_192
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let scheme_end = value.find("://")?;
    let authority = value[scheme_end + 3..].split(['/', '?', '#']).next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = match url.host()? {
        Host::Domain(domain) => domain.to_owned(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => address.to_string(),
    };
    let port = url.port_or_known_default();
    url.set_query(None);
    url.set_fragment(None);
    Some((url.to_string(), host, port))
}

fn bounded_network_members(value: &str) -> Option<Vec<IpAddr>> {
    let (address, prefix) = value.split_once('/')?;
    if prefix.is_empty() || !prefix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let address = address.parse::<IpAddr>().ok()?;
    let prefix = prefix.parse::<u8>().ok()?;
    match address {
        IpAddr::V4(address) if prefix <= 32 => {
            let host_bits = 32 - prefix;
            let count = 1_usize.checked_shl(u32::from(host_bits))?;
            if count > MAX_IP_RANGE_MEMBERS {
                return None;
            }
            let mask = if host_bits == 32 {
                0
            } else {
                u32::MAX << host_bits
            };
            let network = u32::from(address) & mask;
            Some(
                (0..count)
                    .map(|offset| IpAddr::V4(Ipv4Addr::from(network + offset as u32)))
                    .collect(),
            )
        }
        IpAddr::V6(address) if prefix <= 128 => {
            let host_bits = 128 - prefix;
            let count = 1_usize.checked_shl(u32::from(host_bits))?;
            if count > MAX_IP_RANGE_MEMBERS {
                return None;
            }
            let mask = if host_bits == 128 {
                0
            } else {
                u128::MAX << host_bits
            };
            let network = u128::from(address) & mask;
            Some(
                (0..count)
                    .map(|offset| IpAddr::V6(Ipv6Addr::from(network + offset as u128)))
                    .collect(),
            )
        }
        _ => None,
    }
}

impl BbotModule for ChefUrlParentHints {
    fn descriptor(&self) -> ModuleDescriptor {
        ModuleDescriptor {
            name: "chef_url_parents",
            watches: &["URL"],
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
        let Some(value) = event.data.as_str() else {
            return Ok(Vec::new());
        };
        let Some(host) = event.host.as_deref() else {
            return Ok(Vec::new());
        };
        let Some((origin, path)) = checked_url_origin_and_path(value, host, event.port) else {
            return Ok(Vec::new());
        };

        let mut drafts = Vec::new();
        let mut parent_path = path;
        while drafts.len() < MAX_URL_PARENT_HINTS
            && let Some(parent) = parent_directory(&parent_path)
        {
            let url = format!("{origin}{parent}");
            drafts.push(BbotEventDraft {
                event_type: "URL_HINT".to_owned(),
                data: Value::String(url),
                host: Some(host.to_owned()),
                port: event.port,
                tags: vec!["chef:derived-url-hint".to_owned()],
            });
            parent_path = parent;
        }
        Ok(drafts)
    }
}

/// Devuelve solo URLs HTTP(S) cuya autoridad corresponde al host del evento.
/// Rechaza userinfo, separadores ambiguos y hosts que no coincidan.
fn checked_url_origin_and_path(
    value: &str,
    event_host: &str,
    event_port: Option<u16>,
) -> Option<(String, String)> {
    let scheme_end = value.find("://")?;
    let scheme = &value[..scheme_end];
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let after_scheme = &value[scheme_end + 3..];
    let authority_end = after_scheme
        .find(['/', '?', '#'])
        .unwrap_or(after_scheme.len());
    let authority = &after_scheme[..authority_end];
    if authority.is_empty()
        || authority.contains('@')
        || authority.contains('\\')
        || authority.chars().any(char::is_control)
        || authority.chars().any(char::is_whitespace)
    {
        return None;
    }

    let (url_host, explicit_port) = parse_authority(authority)?;
    if crate::host::canonical_host(url_host) != crate::host::canonical_host(event_host) {
        return None;
    }
    if explicit_port.is_some() && explicit_port != event_port {
        return None;
    }

    let path = &after_scheme[authority_end..];
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let path = if path.is_empty() { "/" } else { path };
    if !path.starts_with('/') || path.contains('\\') || path.bytes().any(|byte| byte < 0x20) {
        return None;
    }

    Some((
        format!("{}://{authority}", scheme.to_ascii_lowercase()),
        path.to_owned(),
    ))
}

fn parse_authority(authority: &str) -> Option<(&str, Option<u16>)> {
    if let Some(bracketed) = authority.strip_prefix('[') {
        let end = bracketed.find(']')?;
        let host = &bracketed[..end];
        let suffix = &bracketed[end + 1..];
        let port = if suffix.is_empty() {
            None
        } else {
            Some(suffix.strip_prefix(':')?.parse::<u16>().ok()?)
        };
        return Some((host, port));
    }

    if let Some((host, port)) = authority.rsplit_once(':')
        && !host.contains(':')
        && port.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Some((host, Some(port.parse::<u16>().ok()?)));
    }
    if authority.contains(':') {
        return None;
    }
    Some((authority, None))
}

fn parent_directory(path: &str) -> Option<String> {
    if path == "/" || path.is_empty() {
        return None;
    }
    let without_trailing_slashes = path.trim_end_matches('/');
    let separator = without_trailing_slashes.rfind('/')?;
    let parent = if separator == 0 {
        "/".to_owned()
    } else {
        format!("{}/", &without_trailing_slashes[..separator])
    };
    (parent != path).then_some(parent)
}
