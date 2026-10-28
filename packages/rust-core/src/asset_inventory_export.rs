//! Exportación CHEF de inventario consolidado desde el grafo local.
//! Solo agrega relaciones observadas y mantiene los hallazgos como candidatos.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    str::FromStr,
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::{
    ChefError, ChefReconGraph, ChefResult, ErrorCode, ReconAsset, ReconAssetKind, ReconEvidence,
    ReconRelationship, ReconRelationshipKind, validate_chef_recon_graph,
};

const MAX_EXPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_INVENTORY_ROWS: usize = 20_000;
const HEADERS: [&str; 12] = [
    "id_activo_host",
    "tipo_host",
    "host",
    "ips_observadas_json",
    "puertos_tcp_observados_json",
    "puertos_udp_observados_json",
    "endpoints_observados_json",
    "tecnologias_observadas_json",
    "ids_hallazgos_candidatos_json",
    "observado_desde",
    "observado_hasta",
    "ids_evidencia_json",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefAssetInventoryExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub file_name: String,
    pub content_type: &'static str,
    pub scope_status: &'static str,
    pub row_count: usize,
    pub csv: String,
}

#[derive(Debug, Default)]
struct InventoryRow {
    asset_id: String,
    kind: &'static str,
    host: String,
    ips: BTreeSet<String>,
    tcp_ports: BTreeSet<u16>,
    udp_ports: BTreeSet<u16>,
    endpoints: BTreeSet<String>,
    technologies: BTreeSet<String>,
    candidate_finding_ids: BTreeSet<String>,
    evidence_ids: BTreeSet<String>,
    timestamps: Vec<DateTime<Utc>>,
}

/// Agrupa la evidencia CHEF por host sin reproducir asociaciones heurísticas,
/// score de riesgo ni clasificación interna/externa del módulo de origen.
pub fn export_asset_inventory_csv(
    run_id: &str,
    graph: &ChefReconGraph,
) -> ChefResult<ChefAssetInventoryExport> {
    validate_run_id(run_id)?;
    validate_chef_recon_graph(graph)?;

    let assets = graph
        .assets
        .iter()
        .map(|asset| (asset.id.as_str(), asset))
        .collect::<BTreeMap<_, _>>();
    let evidence = graph
        .evidence
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let mut rows = BTreeMap::<String, InventoryRow>::new();

    for host in graph.assets.iter().filter(|asset| {
        matches!(
            asset.kind,
            ReconAssetKind::Domain | ReconAssetKind::IpAddress
        ) && asset.qualification.is_none()
    }) {
        let kind = match host.kind {
            ReconAssetKind::Domain => "domain",
            ReconAssetKind::IpAddress => "ip_address",
            _ => continue,
        };
        let mut row = InventoryRow {
            asset_id: host.id.clone(),
            kind,
            host: host.key.clone(),
            ..InventoryRow::default()
        };
        if host.kind == ReconAssetKind::IpAddress
            && let Ok(address) = IpAddr::from_str(&host.key)
        {
            row.ips.insert(address.to_string());
        }
        for item in direct_host_evidence(host, &evidence) {
            add_evidence(&mut row, item)?;
        }
        if !row.evidence_ids.is_empty() {
            rows.insert(host.id.clone(), row);
        }
    }

    for relation in &graph.relationships {
        let Some(host) = assets.get(relation.from_asset_id.as_str()) else {
            continue;
        };
        let Some(row) = rows.get_mut(&host.id) else {
            continue;
        };
        let Some(target) = assets.get(relation.to_asset_id.as_str()) else {
            continue;
        };
        match relation.kind {
            ReconRelationshipKind::ResolvesTo
                if relation.assertion == "observed"
                    && host.kind == ReconAssetKind::Domain
                    && target.kind == ReconAssetKind::IpAddress
                    && target.qualification.is_none() =>
            {
                let matched = related_evidence(relation, host, target, &evidence, |item| {
                    item.event_type == "DNS_NAME"
                });
                if !matched.is_empty() {
                    row.ips.insert(target.key.clone());
                    for item in matched {
                        add_evidence(row, item)?;
                    }
                }
            }
            ReconRelationshipKind::ExposesService
                if relation.assertion == "observed"
                    && target.kind == ReconAssetKind::Service
                    && target.qualification.as_deref() == Some("observed_open") =>
            {
                let Some((service_host, protocol, port)) = parse_service_key(&target.key) else {
                    continue;
                };
                if host.kind == ReconAssetKind::IpAddress && host.key != service_host {
                    continue;
                }
                let matched = related_evidence(relation, host, target, &evidence, |item| {
                    matches!(
                        (item.event_type.as_str(), protocol),
                        ("OPEN_TCP_PORT", "tcp") | ("OPEN_UDP_PORT", "udp")
                    )
                });
                if !matched.is_empty() {
                    match protocol {
                        "tcp" => {
                            row.tcp_ports.insert(port);
                        }
                        "udp" => {
                            row.udp_ports.insert(port);
                        }
                        _ => unreachable!("el protocolo se valida al analizar la clave"),
                    }
                    for item in matched {
                        add_evidence(row, item)?;
                    }
                }
            }
            ReconRelationshipKind::ServesEndpoint
                if relation.assertion == "observed"
                    && target.kind == ReconAssetKind::WebEndpoint
                    && target.qualification.as_deref() == Some("observed") =>
            {
                let matched = related_evidence(relation, host, target, &evidence, |item| {
                    matches!(item.event_type.as_str(), "URL" | "HTTP_RESPONSE")
                });
                if !matched.is_empty() {
                    row.endpoints.insert(target.key.clone());
                    for item in matched {
                        add_evidence(row, item)?;
                    }
                }
            }
            ReconRelationshipKind::UsesTechnology
                if relation.assertion == "observed"
                    && target.kind == ReconAssetKind::Technology
                    && target.qualification.is_none() =>
            {
                let matched = related_evidence(relation, host, target, &evidence, |item| {
                    item.event_type == "TECHNOLOGY"
                });
                if !matched.is_empty() {
                    row.technologies.insert(target.key.clone());
                    for item in matched {
                        add_evidence(row, item)?;
                    }
                }
            }
            ReconRelationshipKind::ReportsCandidate
                if relation.assertion == "candidate"
                    && target.kind == ReconAssetKind::CandidateFinding
                    && target.qualification.as_deref() == Some("candidate") =>
            {
                let matched = related_evidence(relation, host, target, &evidence, |item| {
                    item.event_type == "FINDING"
                });
                if !matched.is_empty() {
                    row.candidate_finding_ids.insert(target.key.clone());
                    for item in matched {
                        add_evidence(row, item)?;
                    }
                }
            }
            _ => {}
        }
    }

    if rows.is_empty() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "No hay hosts con evidencia directa para el inventario CHEF",
        ));
    }
    if rows.len() > MAX_INVENTORY_ROWS {
        return Err(row_limit_error());
    }

    let mut ordered_rows = rows.values().collect::<Vec<_>>();
    ordered_rows.sort_by(|left, right| {
        (left.host.as_str(), left.kind, left.asset_id.as_str()).cmp(&(
            right.host.as_str(),
            right.kind,
            right.asset_id.as_str(),
        ))
    });
    let mut csv = String::new();
    write_row(&mut csv, HEADERS.iter().map(|header| (*header).to_owned()));
    for row in &ordered_rows {
        let first_seen = row.timestamps.iter().min().ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "El inventario CHEF no conserva fecha para un host",
            )
        })?;
        let last_seen = row.timestamps.iter().max().ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "El inventario CHEF no conserva fecha para un host",
            )
        })?;
        let values = vec![
            row.asset_id.clone(),
            row.kind.to_owned(),
            row.host.clone(),
            json_cell(&row.ips),
            json_cell(&row.tcp_ports),
            json_cell(&row.udp_ports),
            json_cell(&row.endpoints),
            json_cell(&row.technologies),
            json_cell(&row.candidate_finding_ids),
            first_seen.to_rfc3339_opts(SecondsFormat::Millis, true),
            last_seen.to_rfc3339_opts(SecondsFormat::Millis, true),
            json_cell(&row.evidence_ids),
        ];
        write_row(&mut csv, values);
        if csv.len() > MAX_EXPORT_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La exportación de inventario CHEF supera el máximo de bytes",
            ));
        }
    }

    Ok(ChefAssetInventoryExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        file_name: format!("chef-inventario-{}.csv", run_id.replace(':', "-")),
        content_type: "text/csv; charset=utf-8",
        scope_status: "not_asserted",
        row_count: ordered_rows.len(),
        csv,
    })
}

fn direct_host_evidence<'a>(
    host: &ReconAsset,
    evidence: &BTreeMap<&str, &'a ReconEvidence>,
) -> Vec<&'a ReconEvidence> {
    host.evidence_ids
        .iter()
        .filter_map(|id| evidence.get(id.as_str()).copied())
        .filter(|item| {
            matches!(
                item.event_type.as_str(),
                "DNS_NAME"
                    | "IP_ADDRESS"
                    | "OPEN_TCP_PORT"
                    | "OPEN_UDP_PORT"
                    | "URL"
                    | "HTTP_RESPONSE"
                    | "TECHNOLOGY"
                    | "CERTIFICATE"
                    | "FINDING"
            )
        })
        .collect()
}

fn related_evidence<'a>(
    relation: &ReconRelationship,
    host: &ReconAsset,
    target: &ReconAsset,
    evidence: &BTreeMap<&str, &'a ReconEvidence>,
    matches_event: impl Fn(&ReconEvidence) -> bool,
) -> Vec<&'a ReconEvidence> {
    relation
        .evidence_ids
        .iter()
        .filter(|id| host.evidence_ids.contains(id) && target.evidence_ids.contains(id))
        .filter_map(|id| evidence.get(id.as_str()).copied())
        .filter(|item| matches_event(item))
        .collect()
}

fn add_evidence(row: &mut InventoryRow, evidence: &ReconEvidence) -> ChefResult<()> {
    let timestamp = DateTime::parse_from_rfc3339(&evidence.observed_at)
        .map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La fecha de evidencia del inventario CHEF no es válida",
            )
        })?
        .with_timezone(&Utc);
    row.evidence_ids.insert(evidence.id.clone());
    row.timestamps.push(timestamp);
    Ok(())
}

fn parse_service_key(key: &str) -> Option<(String, &'static str, u16)> {
    let mut parts = key.split('|');
    let host = crate::host::canonical_host(parts.next()?)?;
    let protocol = match parts.next()? {
        "tcp" => "tcp",
        "udp" => "udp",
        _ => return None,
    };
    let port = parts.next()?.parse::<u16>().ok().filter(|port| *port > 0)?;
    if parts.next().is_some() {
        return None;
    }
    Some((host, protocol, port))
}

fn json_cell<T: Serialize>(values: &BTreeSet<T>) -> String {
    serde_json::to_string(values).expect("los valores del inventario se serializan")
}

fn validate_run_id(run_id: &str) -> ChefResult<()> {
    if !run_id.starts_with("run:")
        || run_id.len() == "run:".len()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El identificador de importación CHEF no es válido",
        ));
    }
    Ok(())
}

fn row_limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "La exportación de inventario CHEF supera el máximo de filas",
    )
}

fn write_row(output: &mut String, values: impl IntoIterator<Item = String>) {
    let mut first = true;
    for value in values {
        if !first {
            output.push(',');
        }
        first = false;
        output.push('"');
        let value = neutralize_formula(&value);
        for character in value.chars() {
            if character == '"' {
                output.push_str("\"\"");
            } else {
                output.push(character);
            }
        }
        output.push('"');
    }
    output.push_str("\r\n");
}

fn neutralize_formula(value: &str) -> String {
    let first_non_whitespace = value
        .chars()
        .find(|character| !character.is_whitespace() && *character != '\u{feff}');
    if first_non_whitespace.is_some_and(|character| matches!(character, '=' | '+' | '-' | '@')) {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}
