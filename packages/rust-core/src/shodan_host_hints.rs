//! Extrae puertos y tecnologías de una respuesta de host Shodan archivada.
//!
//! Todos los valores son contexto histórico del proveedor. La lista `vulns` se
//! ignora por completo; no se crean hallazgos ni observaciones actuales.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_SERVICE_ENTRIES: usize = 1024;
const MAX_TECHNOLOGY_HINTS: usize = 2048;
const MAX_PORT_HINTS: usize = 512;
const MAX_TEXT_CHARS: usize = 512;
const MAX_TAGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShodanTechnologyHint {
    pub host: String,
    pub port: Option<u16>,
    pub technology: String,
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShodanPortHint {
    pub host: String,
    pub port: u16,
    pub transport: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedShodanHostHints {
    pub query_ip: String,
    pub technology_hints: Vec<ShodanTechnologyHint>,
    pub port_hints: Vec<ShodanPortHint>,
}

/// Normaliza los registros de servicio de un host response archivado.
///
/// Solo devuelve `cpe`, `cpe23`, `product`, componentes HTTP y `port`/
/// `transport`. Cualquier dato bajo `vulns` se descarta.
pub fn parse_archived_shodan_host(
    query_ip: &str,
    response: &[u8],
) -> ChefResult<ArchivedShodanHostHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Shodan archivada supera el límite de bytes",
        ));
    }
    let query_ip = query_ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La IP consultada no es válida"))?
        .to_string();
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Shodan archivada no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta Shodan debe ser un objeto JSON",
    ))?;
    validate_response_ip(object, &query_ip)?;
    let entries = object
        .get("data")
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Shodan debe contener data como lista",
        ))?;
    if entries.len() > MAX_SERVICE_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Shodan supera el máximo de servicios",
        ));
    }

    let mut technology_hints = BTreeSet::new();
    let mut port_hints = BTreeSet::new();
    for entry in entries {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        if let Some(response_ip) = entry.get("ip_str").and_then(Value::as_str) {
            let parsed_ip = response_ip.parse::<IpAddr>().map_err(|_| {
                ChefError::new(
                    ErrorCode::InvalidInput,
                    "La respuesta Shodan incluye una IP inválida",
                )
            })?;
            if parsed_ip.to_string() != query_ip {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "La IP del servicio no coincide con la consulta",
                ));
            }
        }
        let port = parse_port(entry.get("port"));
        let tags = bounded_text_array(entry.get("tags"), "tags")?;

        for field in ["cpe", "cpe23"] {
            if let Some(values) = entry.get(field) {
                for technology in bounded_array(values, field, MAX_TECHNOLOGY_HINTS)? {
                    if let Some(technology) = bounded_text(Some(technology))? {
                        technology_hints.insert(ShodanTechnologyHint {
                            host: query_ip.clone(),
                            port,
                            technology,
                            categories: Vec::new(),
                        });
                    }
                }
            }
        }
        if let Some(product) = bounded_text(entry.get("product"))? {
            technology_hints.insert(ShodanTechnologyHint {
                host: query_ip.clone(),
                port,
                technology: product,
                categories: Vec::new(),
            });
        }
        if let Some(http) = entry.get("http").and_then(Value::as_object)
            && let Some(components) = http.get("components").and_then(Value::as_object)
        {
            if components.len() > MAX_TECHNOLOGY_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta Shodan supera el máximo de componentes HTTP",
                ));
            }
            for (name, component) in components {
                let Some(technology) = bounded_text(Some(&Value::String(name.clone())))? else {
                    continue;
                };
                let mut categories = bounded_text_array(
                    component
                        .as_object()
                        .and_then(|object| object.get("categories")),
                    "categories",
                )?;
                categories.push("web-technology".to_owned());
                categories.sort();
                categories.dedup();
                technology_hints.insert(ShodanTechnologyHint {
                    host: query_ip.clone(),
                    port,
                    technology,
                    categories,
                });
            }
        }

        if let (Some(port), Some(transport)) =
            (port, entry.get("transport").and_then(Value::as_str))
            && matches!(transport, "tcp" | "udp")
        {
            port_hints.insert(ShodanPortHint {
                host: query_ip.clone(),
                port,
                transport: transport.to_owned(),
                tags,
            });
        }
        if technology_hints.len() > MAX_TECHNOLOGY_HINTS || port_hints.len() > MAX_PORT_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta Shodan supera el máximo CHEF de pistas",
            ));
        }
    }
    Ok(ArchivedShodanHostHints {
        query_ip,
        technology_hints: technology_hints.into_iter().collect(),
        port_hints: port_hints.into_iter().collect(),
    })
}

fn validate_response_ip(object: &serde_json::Map<String, Value>, query_ip: &str) -> ChefResult<()> {
    if let Some(value) = object.get("ip_str")
        && !value.is_null()
    {
        let response_ip = value.as_str().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La IP raíz de la respuesta Shodan no es texto válido",
        ))?;
        let parsed_ip = response_ip.parse::<IpAddr>().map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta Shodan incluye una IP raíz inválida",
            )
        })?;
        if parsed_ip.to_string() != query_ip {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La IP raíz de la respuesta Shodan no coincide con la consulta",
            ));
        }
    }

    if let Some(value) = object.get("ip")
        && !value.is_null()
    {
        let parsed_ip = match value {
            Value::String(value) => value.parse::<IpAddr>().ok(),
            Value::Number(value) => value
                .as_u64()
                .and_then(|value| u32::try_from(value).ok())
                .map(|value| IpAddr::V4(std::net::Ipv4Addr::from(value))),
            _ => None,
        }
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La IP raíz de la respuesta Shodan tiene formato inválido",
        ))?;
        if parsed_ip.to_string() != query_ip {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La IP raíz de la respuesta Shodan no coincide con la consulta",
            ));
        }
    }
    Ok(())
}

fn parse_port(value: Option<&Value>) -> Option<u16> {
    value?
        .as_u64()
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
}

fn bounded_array<'a>(value: &'a Value, field: &str, max: usize) -> ChefResult<&'a [Value]> {
    let values = value.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        if field == "cpe" || field == "cpe23" {
            "Los campos CPE deben ser listas JSON"
        } else {
            "El campo debe ser una lista JSON"
        },
    ))?;
    if values.len() > max {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Una lista Shodan supera el máximo CHEF",
        ));
    }
    Ok(values)
}

fn bounded_text_array(value: Option<&Value>, field: &str) -> ChefResult<Vec<String>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = bounded_array(value, field, MAX_TAGS)?;
    let mut output = BTreeSet::new();
    for value in values {
        if let Some(value) = bounded_text(Some(value))? {
            output.insert(value);
        }
    }
    Ok(output.into_iter().collect())
}

fn bounded_text(value: Option<&Value>) -> ChefResult<Option<String>> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Un valor de tecnología Shodan supera el límite CHEF",
        ));
    }
    Ok(Some(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_shodan_host;
    use crate::ErrorCode;

    #[test]
    fn extracts_archived_ports_and_technologies_but_ignores_vulnerabilities() {
        let response = br#"{"data":[{"ip_str":"192.0.2.10","port":443,"transport":"tcp","tags":["cloud"],"cpe":["cpe:/a:vendor:server"],"cpe23":["cpe:2.3:a:vendor:server:1.0:*:*:*:*:*:*:*"],"product":"Example Server","http":{"components":{"ExampleCMS":{"categories":["CMS"]}}},"vulns":{"CVE-2099-0001":{"cvss":9.8}}}]}"#;
        let result = parse_archived_shodan_host("192.0.2.10", response).unwrap();
        assert_eq!(result.port_hints.len(), 1);
        assert_eq!(result.port_hints[0].port, 443);
        assert_eq!(result.technology_hints.len(), 4);
        assert!(
            result
                .technology_hints
                .iter()
                .any(|technology| technology.technology == "ExampleCMS")
        );
    }

    #[test]
    fn rejects_ip_mismatch_and_invalid_shapes() {
        assert_eq!(
            parse_archived_shodan_host(
                "192.0.2.10",
                br#"{"data":[{"ip_str":"192.0.2.11","port":80,"transport":"tcp"}]}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_shodan_host("192.0.2.10", br#"{"data":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_shodan_host(
                "192.0.2.10",
                br#"{"ip_str":"192.0.2.11","data":[{"port":443,"transport":"tcp"}]}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_shodan_host(
                "192.0.2.10",
                br#"{"ip":3221225995,"data":[{"port":443,"transport":"tcp"}]}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_service_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_shodan_host("192.0.2.10", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let entries = vec![serde_json::json!({"port":443,"transport":"tcp"}); 1025];
        let response = serde_json::to_vec(&serde_json::json!({"data": entries})).unwrap();
        assert_eq!(
            parse_archived_shodan_host("192.0.2.10", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
