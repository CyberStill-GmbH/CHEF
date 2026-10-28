//! Exportación XML CHEF con vocabulario inspirado en Nmap.
//! Solo serializa puertos abiertos directos asociados a una IP por relación observada.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    str::FromStr,
};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;

use crate::{
    ChefError, ChefReconGraph, ChefResult, ErrorCode, ReconAssetKind, ReconRelationshipKind,
    validate_chef_recon_graph,
};

const MAX_XML_BYTES: usize = 2 * 1024 * 1024;
const MAX_HOSTS: usize = 10_000;
const MAX_PORTS: usize = 40_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefNmapEvidenceExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub file_name: String,
    pub content_type: &'static str,
    pub scope_status: &'static str,
    pub observation_start: String,
    pub observation_end: String,
    pub host_count: usize,
    pub port_count: usize,
    pub xml: String,
}

/// Exporta únicamente OPEN_* directos con relación IP→servicio observada.
/// No incorpora banners, hostnames, método de escaneo ni scope inferido.
pub fn export_nmap_evidence_xml(
    run_id: &str,
    graph: &ChefReconGraph,
) -> ChefResult<ChefNmapEvidenceExport> {
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
    let mut hosts = BTreeMap::<IpAddr, BTreeSet<(String, u16)>>::new();
    let mut timestamps = Vec::<DateTime<Utc>>::new();

    for relation in &graph.relationships {
        if relation.kind != ReconRelationshipKind::ExposesService
            || relation.assertion != "observed"
        {
            continue;
        }
        let (Some(host), Some(service)) = (
            assets.get(relation.from_asset_id.as_str()),
            assets.get(relation.to_asset_id.as_str()),
        ) else {
            continue;
        };
        if host.kind != ReconAssetKind::IpAddress
            || host.qualification.is_some()
            || service.kind != ReconAssetKind::Service
            || service.qualification.as_deref() != Some("observed_open")
        {
            continue;
        }
        let Ok(host_ip) = IpAddr::from_str(&host.key) else {
            continue;
        };
        let Some((service_host, protocol, port)) = parse_service_key(&service.key) else {
            continue;
        };
        if host_ip != service_host {
            continue;
        }

        for evidence_id in &relation.evidence_ids {
            if !host.evidence_ids.contains(evidence_id)
                || !service.evidence_ids.contains(evidence_id)
            {
                continue;
            }
            let Some(item) = evidence.get(evidence_id.as_str()) else {
                continue;
            };
            if !event_matches_protocol(&item.event_type, &protocol) {
                continue;
            }
            let observed_at = DateTime::parse_from_rfc3339(&item.observed_at)
                .map_err(|_| {
                    ChefError::new(
                        ErrorCode::InvalidInput,
                        "La fecha de evidencia Nmap CHEF no es válida",
                    )
                })?
                .with_timezone(&Utc);
            timestamps.push(observed_at);
            hosts
                .entry(host_ip)
                .or_default()
                .insert((protocol.clone(), port));
        }
    }

    if hosts.is_empty() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "No hay puertos abiertos directos asociados a una IP para exportar",
        ));
    }
    let port_count = hosts.values().map(BTreeSet::len).sum::<usize>();
    if hosts.len() > MAX_HOSTS || port_count > MAX_PORTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La exportación Nmap CHEF supera el máximo de hosts o puertos",
        ));
    }
    let observation_start = timestamps
        .iter()
        .min()
        .expect("cada host exportado tiene evidencia temporal")
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let observation_end = timestamps
        .iter()
        .max()
        .expect("cada host exportado tiene evidencia temporal")
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    let xml = render_nmap_xml(&hosts, &observation_start);
    if xml.len() > MAX_XML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La exportación XML Nmap CHEF supera el máximo de bytes",
        ));
    }

    Ok(ChefNmapEvidenceExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        file_name: format!("chef-evidencia-nmap-{}.xml", run_id.replace(':', "-")),
        content_type: "application/xml; charset=utf-8",
        scope_status: "not_asserted",
        observation_start,
        observation_end,
        host_count: hosts.len(),
        port_count,
        xml,
    })
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

fn parse_service_key(key: &str) -> Option<(IpAddr, String, u16)> {
    let mut parts = key.split('|');
    let host = IpAddr::from_str(parts.next()?).ok()?;
    let protocol = parts.next()?;
    if protocol != "tcp" && protocol != "udp" {
        return None;
    }
    let port = parts.next()?.parse::<u16>().ok().filter(|port| *port > 0)?;
    if parts.next().is_some() {
        return None;
    }
    Some((host, protocol.to_owned(), port))
}

fn event_matches_protocol(event_type: &str, protocol: &str) -> bool {
    matches!(
        (event_type, protocol),
        ("OPEN_TCP_PORT", "tcp") | ("OPEN_UDP_PORT", "udp")
    )
}

fn render_nmap_xml(
    hosts: &BTreeMap<IpAddr, BTreeSet<(String, u16)>>,
    observation_start: &str,
) -> String {
    let start = DateTime::parse_from_rfc3339(observation_start)
        .expect("se construyó desde evidencia RFC3339")
        .timestamp();
    let start_string = DateTime::parse_from_rfc3339(observation_start)
        .expect("se construyó desde evidencia RFC3339")
        .format("%a %b %d %H:%M:%S %Y UTC");
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE nmaprun>\n<nmaprun scanner=\"chef\" args=\"CHEF local evidence export\" start=\"{start}\" startstr=\"{start_string}\" version=\"CHEF 1.0.0\" xmloutputversion=\"1.05\">\n"
    );
    for (ip, ports) in hosts {
        let address_type = if ip.is_ipv4() { "ipv4" } else { "ipv6" };
        xml.push_str("  <host>\n    <status state=\"up\" reason=\"unknown\" reason_ttl=\"0\"/>\n");
        xml.push_str(&format!(
            "    <address addr=\"{ip}\" addrtype=\"{address_type}\"/>\n    <ports>\n"
        ));
        for (protocol, port) in ports {
            xml.push_str(&format!(
                "      <port protocol=\"{protocol}\" portid=\"{port}\"><state state=\"open\" reason=\"unknown\" reason_ttl=\"0\"/></port>\n"
            ));
        }
        xml.push_str("    </ports>\n  </host>\n");
    }
    xml.push_str("</nmaprun>\n");
    xml
}
