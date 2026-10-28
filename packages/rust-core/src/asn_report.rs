//! Resumen local de eventos ASN importados.
//!
//! Adapta un resumen tabular ASN a partir de evidencia ya importada, sin repetir
//! consultas al proveedor. Cada fila conserva las observaciones IP/ASN y su
//! procedencia; los metadatos opcionales solo aparecen cuando están serializados
//! en el evento de origen. El módulo no consulta red, no expande prefijos ni
//! autoriza alcance.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use serde_json::Value;

use crate::{BbotEvent, BbotEventGraph, ChefError, ChefResult, ErrorCode};

const MAX_REPORT_ROWS: usize = 1_024;
const MAX_EVIDENCE: usize = 10_000;
const MAX_METADATA_VALUES_PER_FIELD: usize = 512;
const MAX_METADATA_TEXT_BYTES: usize = 1_024;
const MAX_PROVIDER_SUBNETS: usize = 4_096;

/// Resumen por ASN a partir de eventos que ya están guardados localmente.
///
/// Los vectores de metadatos preservan variantes o contradicciones entre
/// observaciones. Una lista vacía indica que esos datos no estaban presentes
/// en los eventos del grafo; no se completan desde el proveedor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AsnReportRow {
    pub asn: u32,
    pub ips: Vec<String>,
    pub names: Vec<String>,
    pub descriptions: Vec<String>,
    pub countries: Vec<String>,
    /// Conteos observados solo cuando `subnets` estaba presente en el evento y
    /// todos sus elementos eran redes canónicas. CHEF no devuelve ni expande
    /// esos prefijos.
    pub preserved_subnet_counts: Vec<usize>,
    pub evidence: Vec<AsnReportEvidence>,
}

/// Referencias directas a los dos eventos que sustentan una observación IP/ASN.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct AsnReportEvidence {
    pub ip: String,
    pub ip_event_id: String,
    pub ip_locator: String,
    pub ip_observed_at: String,
    pub asn_event_id: String,
    pub asn_locator: String,
    pub asn_observed_at: String,
    pub asn_module: Option<String>,
}

#[derive(Default)]
struct RowBuilder {
    ips: BTreeSet<String>,
    names: BTreeSet<String>,
    descriptions: BTreeSet<String>,
    countries: BTreeSet<String>,
    preserved_subnet_counts: BTreeSet<usize>,
    evidence: BTreeSet<AsnReportEvidence>,
}

/// Resume eventos `ASN` vinculados directamente a un evento padre `IP_ADDRESS`.
///
/// No interpreta el número ASN como control de una red. Los IP y metadatos son
/// hechos reportados por los eventos importados, no datos consultados ahora.
/// Eventos huérfanos, con padre de otro tipo, sin IP canónica o producidos a
/// partir de `ipneighbor` se excluyen porque no satisfacen la relación del
/// reporte fuente. `scope_distance` nunca se usa como permiso o filtro.
pub fn summarize_asn_report(graph: &BbotEventGraph) -> ChefResult<Vec<AsnReportRow>> {
    crate::validate_bbot_event_graph(graph)?;

    let events_by_id = graph
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect::<BTreeMap<_, _>>();
    let mut grouped = BTreeMap::<u32, RowBuilder>::new();
    let mut evidence_count = 0usize;

    for asn_event in graph
        .events
        .iter()
        .filter(|event| event.event_type == "ASN")
    {
        let Some(asn) = read_asn(asn_event) else {
            continue;
        };
        let Some(parent) = asn_event
            .parent_id
            .as_deref()
            .and_then(|parent_id| events_by_id.get(parent_id).copied())
            .filter(|parent| parent.event_type == "IP_ADDRESS")
        else {
            continue;
        };
        if parent.module.as_deref() == Some("ipneighbor") {
            continue;
        }
        let Some(ip) = read_ip(parent) else {
            continue;
        };

        if !grouped.contains_key(&asn) && grouped.len() == MAX_REPORT_ROWS {
            return Err(report_limit_error());
        }
        let row = grouped.entry(asn).or_default();
        let evidence = AsnReportEvidence {
            ip: ip.clone(),
            ip_event_id: parent.id.clone(),
            ip_locator: parent.locator.clone(),
            ip_observed_at: parent.timestamp.clone(),
            asn_event_id: asn_event.id.clone(),
            asn_locator: asn_event.locator.clone(),
            asn_observed_at: asn_event.timestamp.clone(),
            asn_module: asn_event.module.clone(),
        };
        if row.evidence.insert(evidence) {
            evidence_count += 1;
            if evidence_count > MAX_EVIDENCE {
                return Err(report_limit_error());
            }
        }
        row.ips.insert(ip);
        add_metadata_values(row, asn_event)?;
    }

    Ok(grouped
        .into_iter()
        .map(|(asn, row)| AsnReportRow {
            asn,
            ips: row.ips.into_iter().collect(),
            names: row.names.into_iter().collect(),
            descriptions: row.descriptions.into_iter().collect(),
            countries: row.countries.into_iter().collect(),
            preserved_subnet_counts: row.preserved_subnet_counts.into_iter().collect(),
            evidence: row.evidence.into_iter().collect(),
        })
        .collect())
}

fn read_asn(event: &BbotEvent) -> Option<u32> {
    let value = match &event.data {
        Value::Object(object) => object.get("asn")?,
        value => value,
    };
    let asn = match value {
        Value::Number(number) => u32::try_from(number.as_u64()?).ok()?,
        Value::String(value) => value
            .strip_prefix("AS")
            .or_else(|| value.strip_prefix("as"))
            .unwrap_or(value)
            .parse::<u32>()
            .ok()?,
        _ => return None,
    };
    (asn > 0).then_some(asn)
}

fn read_ip(event: &BbotEvent) -> Option<String> {
    let ip = event
        .host
        .as_deref()
        .and_then(|value| value.parse::<std::net::IpAddr>().ok())
        .or_else(|| event.data.as_str()?.parse::<std::net::IpAddr>().ok())?;
    is_conservatively_global_ip(ip).then(|| ip.to_string())
}

/// Filtro conservador para aproximar `ipaddress.ip_address(...).is_global`
/// usado por el reporte fuente antes de consultar el proveedor ASN.
///
/// IPv4 rechaza rangos especiales conocidos (incluye `192.0.0.0/24` completo,
/// aunque allí haya excepciones globales). IPv6 solo acepta `2000::/3` y
/// excluye los bloques especiales conocidos listados aquí. Esta tabla está
/// embebida y no es una copia completa/versionada del registro IANA; puede
/// excluir direcciones globales excepcionales o no reconocer nuevas reservas.
/// Por ello la IP se conserva únicamente como referencia a evidencia ya
/// importada, nunca como autorización para enviar consultas.
fn is_conservatively_global_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(address) => {
            let blocked = [
                ([0, 0, 0, 0], 8),
                ([10, 0, 0, 0], 8),
                ([100, 64, 0, 0], 10),
                ([127, 0, 0, 0], 8),
                ([169, 254, 0, 0], 16),
                ([172, 16, 0, 0], 12),
                ([192, 0, 0, 0], 24),
                ([192, 0, 2, 0], 24),
                ([192, 88, 99, 0], 24),
                ([192, 168, 0, 0], 16),
                ([198, 18, 0, 0], 15),
                ([198, 51, 100, 0], 24),
                ([203, 0, 113, 0], 24),
                ([224, 0, 0, 0], 4),
                ([240, 0, 0, 0], 4),
            ];
            !blocked
                .iter()
                .any(|(network, prefix)| ipv4_in_cidr(address, *network, *prefix))
        }
        std::net::IpAddr::V6(address) => {
            ipv6_in_cidr(address, [0x2000, 0, 0, 0, 0, 0, 0, 0], 3)
                && !ipv6_in_cidr(address, [0x2001, 0, 0, 0, 0, 0, 0, 0], 23)
                && !ipv6_in_cidr(address, [0x2001, 0x0db8, 0, 0, 0, 0, 0, 0], 32)
                && !ipv6_in_cidr(address, [0x2002, 0, 0, 0, 0, 0, 0, 0], 16)
                && !ipv6_in_cidr(address, [0x3fff, 0, 0, 0, 0, 0, 0, 0], 20)
        }
    }
}

fn ipv4_in_cidr(address: std::net::Ipv4Addr, network: [u8; 4], prefix: u8) -> bool {
    let address = u32::from(address);
    let network = u32::from(std::net::Ipv4Addr::from(network));
    let mask = u32::MAX << (32 - prefix);
    address & mask == network & mask
}

fn ipv6_in_cidr(address: std::net::Ipv6Addr, network: [u16; 8], prefix: u8) -> bool {
    let address = u128::from(address);
    let network = u128::from(std::net::Ipv6Addr::from(network));
    let mask = u128::MAX << (128 - prefix);
    address & mask == network & mask
}

fn add_metadata_values(row: &mut RowBuilder, event: &BbotEvent) -> ChefResult<()> {
    for key in ["asn_name", "name"] {
        insert_text(&mut row.names, field(event, key));
    }
    for key in ["org", "description"] {
        insert_text(&mut row.descriptions, field(event, key));
    }
    insert_text(&mut row.countries, field(event, "country"));

    if let Some(subnets) = field(event, "subnets").and_then(Value::as_array) {
        if subnets.len() > MAX_PROVIDER_SUBNETS {
            return Err(report_limit_error());
        }
        let canonical = subnets
            .iter()
            .map(|value| {
                let value = value.as_str()?;
                crate::target_seed::canonical_network(value)
            })
            .collect::<Option<Vec<_>>>();
        if canonical.is_some() {
            // El reporte de origen usaba `len(subnets)`: no deduplicar el
            // conteo preserva esa semántica aunque los prefijos se repitan.
            row.preserved_subnet_counts.insert(subnets.len());
        }
    }

    if row.names.len() > MAX_METADATA_VALUES_PER_FIELD
        || row.descriptions.len() > MAX_METADATA_VALUES_PER_FIELD
        || row.countries.len() > MAX_METADATA_VALUES_PER_FIELD
        || row.preserved_subnet_counts.len() > MAX_METADATA_VALUES_PER_FIELD
    {
        return Err(report_limit_error());
    }
    Ok(())
}

fn field<'a>(event: &'a BbotEvent, key: &str) -> Option<&'a Value> {
    event
        .data
        .as_object()
        .and_then(|object| object.get(key))
        .or_else(|| {
            event
                .metadata
                .as_object()
                .and_then(|object| object.get(key))
        })
}

fn insert_text(values: &mut BTreeSet<String>, value: Option<&Value>) {
    let Some(value) = value.and_then(Value::as_str).map(str::trim) else {
        return;
    };
    if !value.is_empty()
        && value.len() <= MAX_METADATA_TEXT_BYTES
        && !value.chars().any(char::is_control)
    {
        values.insert(value.to_owned());
    }
}

fn report_limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "El resumen local ASN supera los límites CHEF",
    )
}
