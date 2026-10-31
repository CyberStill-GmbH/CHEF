//! Normaliza respuestas archivadas de Shodan InternetDB como pistas CHEF.
//!
//! No consulta el servicio ni estima vigencia. El campo `vulns` se ignora de
//! forma deliberada: una referencia del proveedor no confirma una vulnerabilidad.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_ENTRIES_PER_FIELD: usize = 2048;
const MAX_HINTS: usize = 1024;
const MAX_TEXT_CHARS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ShodanInternetDbPortHint {
    pub port: u16,
    pub transport: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedShodanInternetDbHints {
    pub query_ip: String,
    pub hostnames: Vec<String>,
    pub cpes: Vec<String>,
    pub ports: Vec<ShodanInternetDbPortHint>,
    pub tags: Vec<String>,
}

/// Normaliza el objeto JSON archivado de InternetDB asociado a una IP.
///
/// Extrae hostnames, CPE, puertos y etiquetas declarados. No devuelve las
/// referencias de CVE de `vulns`, ni las convierte en hallazgos CHEF.
pub fn parse_archived_shodan_internetdb(
    query_ip: &str,
    response: &[u8],
) -> ChefResult<ArchivedShodanInternetDbHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta InternetDB archivada supera el límite de bytes",
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
            "La respuesta InternetDB archivada no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta InternetDB debe ser un objeto JSON",
    ))?;

    if let Some(response_ip) = object.get("ip").and_then(Value::as_str) {
        let response_ip = response_ip.parse::<IpAddr>().map_err(|_| {
            ChefError::new(ErrorCode::InvalidInput, "La IP de InternetDB no es válida")
        })?;
        if response_ip.to_string() != query_ip {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La IP de InternetDB no coincide con la consulta",
            ));
        }
    }

    let mut hostnames = BTreeSet::new();
    for hostname in string_array(object.get("hostnames"), "hostnames")? {
        if let Some(hostname) = canonical_dns_name(hostname) {
            hostnames.insert(hostname);
        }
    }
    let cpes = normalized_strings(object.get("cpes"), "cpes")?;
    let tags = normalized_strings(object.get("tags"), "tags")?;

    let mut ports = BTreeSet::new();
    if let Some(values) = object.get("ports") {
        let values = values.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Los puertos InternetDB deben estar en una lista",
        ))?;
        check_array_limit(values.len())?;
        for value in values {
            let Some(port) = value.as_u64().and_then(|port| u16::try_from(port).ok()) else {
                continue;
            };
            if port > 0 {
                // InternetDB reports port numbers without transport metadata.
                // Retain the source limitation instead of asserting TCP/UDP.
                ports.insert(ShodanInternetDbPortHint {
                    port,
                    transport: "unknown".to_owned(),
                });
            }
        }
    }
    if hostnames.len() > MAX_HINTS
        || cpes.len() > MAX_HINTS
        || tags.len() > MAX_HINTS
        || ports.len() > MAX_HINTS
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta InternetDB supera el máximo CHEF de pistas",
        ));
    }

    Ok(ArchivedShodanInternetDbHints {
        query_ip,
        hostnames: hostnames.into_iter().collect(),
        cpes,
        ports: ports.into_iter().collect(),
        tags,
    })
}

fn string_array<'a>(value: Option<&'a Value>, field: &str) -> ChefResult<Vec<&'a str>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        if field == "hostnames" {
            "Los hostnames InternetDB deben estar en una lista"
        } else {
            "El campo InternetDB debe estar en una lista"
        },
    ))?;
    check_array_limit(values.len())?;
    values
        .iter()
        .map(|value| {
            value.as_str().ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta InternetDB contiene un valor que no es texto",
            ))
        })
        .collect()
}

fn normalized_strings(value: Option<&Value>, field: &str) -> ChefResult<Vec<String>> {
    let mut output = BTreeSet::new();
    for value in string_array(value, field)? {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Un campo de texto InternetDB supera el límite CHEF",
            ));
        }
        output.insert(value.to_owned());
    }
    Ok(output.into_iter().collect())
}

fn check_array_limit(length: usize) -> ChefResult<()> {
    if length > MAX_ENTRIES_PER_FIELD {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Una lista InternetDB supera el máximo CHEF",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_shodan_internetdb;
    use crate::ErrorCode;

    #[test]
    fn extracts_archived_assets_and_ignores_vulnerability_references() {
        let response = br#"{"ip":"192.0.2.30","hostnames":["Mail.Example.com.","mail.example.com","bad host"],"cpes":[" cpe:/a:vendor:mail ","cpe:/a:vendor:mail"],"ports":[25,443,0,70000],"tags":["starttls","starttls"],"vulns":["CVE-2099-0001"]}"#;
        let result = parse_archived_shodan_internetdb("192.0.2.30", response).unwrap();
        assert_eq!(result.hostnames, ["mail.example.com"]);
        assert_eq!(result.cpes, ["cpe:/a:vendor:mail"]);
        assert_eq!(result.ports.len(), 2);
        assert!(result.ports.iter().all(|port| port.transport == "unknown"));
        assert_eq!(result.tags, ["starttls"]);
    }

    #[test]
    fn rejects_ip_mismatch_and_invalid_shapes() {
        assert_eq!(
            parse_archived_shodan_internetdb(
                "192.0.2.30",
                br#"{"ip":"192.0.2.31","hostnames":[],"cpes":[],"ports":[],"tags":[]}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_shodan_internetdb(
                "192.0.2.30",
                br#"{"ip":"192.0.2.30","hostnames":{}}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_byte_and_item_limits() {
        let oversized = vec![b'x'; 512 * 1024 + 1];
        assert_eq!(
            parse_archived_shodan_internetdb("192.0.2.30", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let hosts = vec![serde_json::json!("host.example.com"); 2049];
        let response = serde_json::to_vec(&serde_json::json!({"hostnames": hosts})).unwrap();
        assert_eq!(
            parse_archived_shodan_internetdb("192.0.2.30", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
