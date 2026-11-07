//! Adaptación CHEF del parser ligero `core.event.helpers.EventSeed` de BBOT.
//!
//! Solo clasifica objetivos literales para preparar alcance. No resuelve DNS,
//! expande rangos ni inicia reconocimiento.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_TARGET_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChefTargetKind {
    Asn,
    IpAddress,
    IpRange,
    DnsName,
    Url,
    TcpEndpoint,
}

/// Objetivo CHEF clasificado. `kind` expresa el tipo de entrada, no confirma
/// que el activo exista ni que un servicio esté abierto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChefTargetSeed {
    pub kind: ChefTargetKind,
    pub data: String,
    pub host: Option<String>,
    pub port: Option<u16>,
}

/// Clasifica los tipos host-based de BBOT que CHEF puede representar sin
/// resolver nombres ni consultar servicios.
pub fn parse_chef_target_seed(input: &str) -> ChefResult<ChefTargetSeed> {
    let value = input.trim();
    if value.is_empty()
        || value.len() > MAX_TARGET_BYTES
        || value.chars().any(char::is_control)
        || value.contains('\\')
    {
        return Err(invalid_target());
    }

    if let Some(asn) = parse_asn(value) {
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::Asn,
            data: format!("AS{asn}"),
            host: None,
            port: None,
        });
    }

    if let Ok(address) = value.parse::<IpAddr>() {
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::IpAddress,
            data: address.to_string(),
            host: Some(address.to_string()),
            port: None,
        });
    }

    if value.contains('/') {
        let network = canonical_network(value).ok_or_else(invalid_target)?;
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::IpRange,
            data: network.clone(),
            host: Some(network),
            port: None,
        });
    }

    let lowercase = value.to_ascii_lowercase();
    if lowercase.starts_with("http://") || lowercase.starts_with("https://") {
        let data = crate::url_tools::canonical_http_url(value).ok_or_else(invalid_target)?;
        let url = Url::parse(&data).map_err(|_| invalid_target())?;
        let host = url_host(url.host().ok_or_else(invalid_target)?);
        let port = url.port_or_known_default();
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::Url,
            data,
            host: Some(host),
            port,
        });
    }

    if let Ok(url) = Url::parse(&format!("tcp://{value}"))
        && !crate::url_tools::has_userinfo(&url)
        && url.query().is_none()
        && url.fragment().is_none()
        && url.path() == "/"
        && let Some(port) = url.port().filter(|port| *port > 0)
    {
        let host = url_host(url.host().ok_or_else(invalid_target)?);
        let netloc = format_netloc(&host, port);
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::TcpEndpoint,
            data: netloc,
            host: Some(host),
            port: Some(port),
        });
    }

    if let Ok(Host::Domain(domain)) = Host::parse(value) {
        return Ok(ChefTargetSeed {
            kind: ChefTargetKind::DnsName,
            data: domain.to_owned(),
            host: Some(domain.to_owned()),
            port: None,
        });
    }

    Err(invalid_target())
}

fn parse_asn(value: &str) -> Option<u32> {
    let digits = if value
        .get(..3)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("ASN"))
    {
        &value[3..]
    } else if value
        .get(..2)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("AS"))
    {
        &value[2..]
    } else {
        return None;
    };
    let digits = digits.strip_prefix(':').unwrap_or(digits);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok()
}

pub(crate) fn canonical_network(value: &str) -> Option<String> {
    let (address, prefix) = value.split_once('/')?;
    if address.contains('%') || prefix.is_empty() || !prefix.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let address = address.parse::<IpAddr>().ok()?;
    let prefix = prefix.parse::<u8>().ok()?;
    match address {
        IpAddr::V4(address) if prefix <= 32 => {
            let host_bits = 32 - prefix;
            let mask = if host_bits == 32 {
                0
            } else {
                u32::MAX << host_bits
            };
            let network = Ipv4Addr::from(u32::from(address) & mask);
            Some(format!("{network}/{prefix}"))
        }
        IpAddr::V6(address) if prefix <= 128 => {
            let host_bits = 128 - prefix;
            let mask = if host_bits == 128 {
                0
            } else {
                u128::MAX << host_bits
            };
            let network = Ipv6Addr::from(u128::from(address) & mask);
            Some(format!("{network}/{prefix}"))
        }
        _ => None,
    }
}

pub(crate) fn network_contains(network: &str, address: IpAddr) -> bool {
    let Some((network_address, prefix)) = network.split_once('/') else {
        return false;
    };
    let (Ok(network_address), Ok(prefix)) =
        (network_address.parse::<IpAddr>(), prefix.parse::<u8>())
    else {
        return false;
    };
    match (network_address, address) {
        (IpAddr::V4(network), IpAddr::V4(address)) if prefix <= 32 => {
            let host_bits = 32 - prefix;
            let mask = if host_bits == 32 {
                0
            } else {
                u32::MAX << host_bits
            };
            u32::from(network) & mask == u32::from(address) & mask
        }
        (IpAddr::V6(network), IpAddr::V6(address)) if prefix <= 128 => {
            let host_bits = 128 - prefix;
            let mask = if host_bits == 128 {
                0
            } else {
                u128::MAX << host_bits
            };
            u128::from(network) & mask == u128::from(address) & mask
        }
        _ => false,
    }
}

fn url_host(host: Host<&str>) -> String {
    match host {
        Host::Domain(domain) => domain.to_owned(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => address.to_string(),
    }
}

fn format_netloc(host: &str, port: u16) -> String {
    if host.contains(':') {
        format!("[{host}]:{port}")
    } else {
        format!("{host}:{port}")
    }
}

fn invalid_target() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "El objetivo CHEF no es una IP, CIDR, dominio, URL HTTP(S) o endpoint host:puerto válido",
    )
}
