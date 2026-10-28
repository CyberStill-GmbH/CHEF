//! Normalización acotada de respuestas archivadas de Censys Hosts.
//!
//! Convierte servicios, URLs, nombres, tecnologías y protocolos en candidatos
//! CHEF. Los puertos de Censys son históricos y nunca se publican como puertos
//! abiertos actuales. El parser no realiza solicitudes ni persiste el cuerpo.

use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;

use serde_json::Value;
use url::{Host, Url};

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SERVICES: usize = 2048;
const MAX_DNS_NAMES: usize = 100;
const MAX_DERIVED_DNS_NAMES: usize = 16_384;
const MAX_TECHNOLOGIES: usize = 4096;
const MAX_URLS: usize = 2048;
const MAX_TEXT_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CensysIpTransport {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CensysDnsNameSource {
    CertificateSan,
    CertificateCommonName,
    ReverseDns,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CensysIpPortHint {
    pub port: u16,
    pub transport: CensysIpTransport,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CensysIpDnsNameHint {
    pub hostname: String,
    pub source: CensysDnsNameSource,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CensysIpProtocolHint {
    pub protocol: String,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensysIpHostResponse {
    pub target_ip: String,
    pub port_hints: Vec<CensysIpPortHint>,
    pub dns_name_hints: Vec<CensysIpDnsNameHint>,
    /// Solo conserva el origen HTTP(S); elimina path, query y fragmento.
    pub url_hints: Vec<String>,
    pub technology_hints: Vec<String>,
    pub protocol_hints: Vec<CensysIpProtocolHint>,
}

/// Normaliza una respuesta `/v2/hosts/{ip}` archivada, sin volver a consultarla.
///
/// Se preservan las categorías que el módulo BBOT extrae, pero se califican
/// como pistas por proceder de un snapshot de proveedor. Las URLs se reducen
/// al origen y solo se aceptan cuando apuntan a la IP consultada.
pub fn parse_censys_ip_host_response(
    target_ip: &str,
    response: &[u8],
) -> ChefResult<CensysIpHostResponse> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Censys Hosts supera el límite de bytes CHEF",
        ));
    }
    let target_ip = target_ip.parse::<IpAddr>().map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El objetivo Censys debe ser una IP",
        )
    })?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Censys Hosts no contiene JSON válido",
        )
    })?;
    let mut output = empty_response(target_ip);
    let Some(result) = document.get("result") else {
        return Ok(output);
    };
    if result.is_null() {
        return Ok(output);
    }
    let result = result.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "result de Censys Hosts debe ser un objeto",
    ))?;
    if let Some(response_ip) = result.get("ip") {
        let response_ip = response_ip.as_str().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La IP de result Censys no es texto",
        ))?;
        let response_ip = response_ip.parse::<IpAddr>().map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La IP de result Censys no es válida",
            )
        })?;
        if response_ip != target_ip {
            return Err(ChefError::new(
                ErrorCode::OutOfScope,
                "La respuesta Censys pertenece a otra IP",
            ));
        }
    }

    let mut ports = BTreeSet::new();
    let mut protocols = BTreeSet::new();
    let mut urls = BTreeSet::new();
    let mut technologies = BTreeSet::new();
    let mut dns_names = BTreeMap::new();
    let mut dns_name_count = 0;
    let mut technology_count = 0;
    let mut url_count = 0;

    if let Some(services) = result.get("services") {
        let services = services.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "services de Censys Hosts debe ser una lista",
        ))?;
        if services.len() > MAX_SERVICES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Censys Hosts supera el límite de servicios",
            ));
        }

        for service in services {
            let service = service.as_object().ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "Censys Hosts contiene un servicio con formato inválido",
            ))?;
            let port = match service.get("port") {
                None | Some(Value::Null) => None,
                Some(value) => Some(
                    value
                        .as_u64()
                        .filter(|port| (1..=u16::MAX as u64).contains(port))
                        .map(|port| port as u16)
                        .ok_or(ChefError::new(
                            ErrorCode::InvalidInput,
                            "Censys Hosts contiene un puerto inválido",
                        ))?,
                ),
            };
            let transport = match service.get("transport_protocol") {
                None => "TCP",
                Some(Value::String(transport)) => transport,
                Some(_) => {
                    return Err(ChefError::new(
                        ErrorCode::InvalidInput,
                        "Censys Hosts contiene un transporte que no es texto",
                    ));
                }
            }
            .trim()
            .to_ascii_uppercase();
            if transport.is_empty()
                || transport.len() > 16
                || !transport.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Censys Hosts contiene un transporte inválido",
                ));
            }
            if let Some(port) = port {
                ports.insert(CensysIpPortHint {
                    port,
                    transport: if matches!(transport.as_str(), "UDP" | "QUIC") {
                        CensysIpTransport::Udp
                    } else {
                        CensysIpTransport::Tcp
                    },
                });
            }

            let mut service_name = service
                .get("extended_service_name")
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .or_else(|| service.get("service_name").and_then(Value::as_str))
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            if service_name == "UNKNOWN" && !matches!(transport.as_str(), "TCP" | "UDP") {
                service_name.clone_from(&transport);
            }
            if !service_name.is_empty()
                && !matches!(service_name.as_str(), "HTTP" | "HTTPS" | "UNKNOWN")
            {
                protocols.insert(CensysIpProtocolHint {
                    protocol: service_name,
                    port,
                });
            }

            if let Some(uri) = service
                .get("http")
                .and_then(|http| http.get("request"))
                .and_then(|request| request.get("uri"))
            {
                let uri = uri.as_str().ok_or(ChefError::new(
                    ErrorCode::InvalidInput,
                    "La URI HTTP de Censys no es texto",
                ))?;
                if let Some(origin) = target_http_origin(uri, target_ip) {
                    url_count += 1;
                    if url_count > MAX_URLS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "Censys Hosts supera el límite de URLs",
                        ));
                    }
                    urls.insert(origin);
                }
            }

            if let Some(leaf) = service
                .get("tls")
                .and_then(|tls| tls.get("certificates"))
                .and_then(|certificates| certificates.get("leaf_data"))
            {
                collect_dns_names(
                    leaf.get("names"),
                    CensysDnsNameSource::CertificateSan,
                    &mut dns_names,
                    &mut dns_name_count,
                )?;
                if let Some(subject) = leaf.get("subject") {
                    collect_dns_names(
                        subject.get("common_name"),
                        CensysDnsNameSource::CertificateCommonName,
                        &mut dns_names,
                        &mut dns_name_count,
                    )?;
                }
            }

            if let Some(software) = service.get("software") {
                let software = software.as_array().ok_or(ChefError::new(
                    ErrorCode::InvalidInput,
                    "software de Censys Hosts debe ser una lista",
                ))?;
                for item in software {
                    let item = item.as_object().ok_or(ChefError::new(
                        ErrorCode::InvalidInput,
                        "Censys Hosts contiene un software con formato inválido",
                    ))?;
                    let product = item
                        .get("uniform_resource_identifier")
                        .and_then(Value::as_str)
                        .or_else(|| item.get("product").and_then(Value::as_str))
                        .unwrap_or("")
                        .trim();
                    if !product.is_empty()
                        && product.len() <= MAX_TEXT_BYTES
                        && !product.chars().any(char::is_control)
                    {
                        technology_count += 1;
                        if technology_count > MAX_TECHNOLOGIES {
                            return Err(ChefError::new(
                                ErrorCode::LimitExceeded,
                                "Censys Hosts supera el límite de tecnologías",
                            ));
                        }
                        technologies.insert(product.to_ascii_lowercase());
                    }
                }
            }
        }
    }

    if let Some(dns) = result.get("dns")
        && let Some(names) = dns.get("names")
    {
        let names = names.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "dns.names de Censys Hosts debe ser una lista",
        ))?;
        for name in names.iter().take(MAX_DNS_NAMES) {
            collect_one_dns_name(
                name,
                CensysDnsNameSource::ReverseDns,
                &mut dns_names,
                &mut dns_name_count,
            )?;
        }
    }

    output.port_hints = ports.into_iter().collect();
    output.dns_name_hints = dns_names
        .into_iter()
        .map(|(hostname, source)| CensysIpDnsNameHint { hostname, source })
        .collect();
    output.url_hints = urls.into_iter().collect();
    output.technology_hints = technologies.into_iter().collect();
    output.protocol_hints = protocols.into_iter().collect();
    Ok(output)
}

fn empty_response(target_ip: IpAddr) -> CensysIpHostResponse {
    CensysIpHostResponse {
        target_ip: target_ip.to_string(),
        port_hints: Vec::new(),
        dns_name_hints: Vec::new(),
        url_hints: Vec::new(),
        technology_hints: Vec::new(),
        protocol_hints: Vec::new(),
    }
}

fn collect_dns_names(
    values: Option<&Value>,
    source: CensysDnsNameSource,
    names: &mut BTreeMap<String, CensysDnsNameSource>,
    count: &mut usize,
) -> ChefResult<()> {
    let Some(values) = values else {
        return Ok(());
    };
    let values = values.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "Censys Hosts contiene nombres DNS con formato inválido",
    ))?;
    for value in values {
        collect_one_dns_name(value, source.clone(), names, count)?;
    }
    Ok(())
}

fn collect_one_dns_name(
    value: &Value,
    source: CensysDnsNameSource,
    names: &mut BTreeMap<String, CensysDnsNameSource>,
    count: &mut usize,
) -> ChefResult<()> {
    *count += 1;
    if *count > MAX_DERIVED_DNS_NAMES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Censys Hosts supera el límite de nombres DNS",
        ));
    }
    let value = value.as_str().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "Censys Hosts contiene un nombre DNS que no es texto",
    ))?;
    let value = value.trim();
    let value = value.strip_prefix("*.").unwrap_or(value);
    let Some(hostname) = crate::host::canonical_dns_name(value) else {
        return Ok(());
    };
    names.entry(hostname).or_insert(source);
    Ok(())
}

fn target_http_origin(uri: &str, target_ip: IpAddr) -> Option<String> {
    let origin = crate::url_tools::canonical_http_origin(uri)?;
    let parsed = Url::parse(&origin).ok()?;
    let address = match parsed.host()? {
        Host::Ipv4(address) => IpAddr::V4(address),
        Host::Ipv6(address) => IpAddr::V6(address),
        Host::Domain(_) => return None,
    };
    (address == target_ip).then_some(origin)
}
