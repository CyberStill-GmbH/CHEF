//! Adapta respuestas Censys Hosts archivadas al flujo de evidencia CHEF.
//!
//! No hace solicitudes. El momento del evento es el de importación; cuando la
//! fuente no aporta una fecha fiable, los datos se califican como pistas de
//! snapshot y no como observaciones actuales.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BbotEvent, BbotEventGraph, CensysDnsNameSource, CensysIpTransport, ChefError, ChefReconGraph,
    ChefResult, Clock, ErrorCode, parse_censys_ip_host_response, recon::adapt_bbot_to_chef,
};

const MAX_EVENTS: usize = 10_000;
const MAX_GRAPH_BYTES: usize = 8 * 1024 * 1024;
const MODULE: &str = "chef_censys_ip_snapshot";
const SNAPSHOT_TAG: &str = "chef:provider-snapshot-hint";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CensysIpImportResult {
    pub target_ip: String,
    pub response_sha256: String,
    pub event_graph: BbotEventGraph,
    pub recon_graph: ChefReconGraph,
}

/// Convierte bytes archivados en eventos con linaje y una proyección CHEF.
/// La importación no concede permiso para consultar la IP ni los nombres.
pub fn import_censys_ip_snapshot(
    target_ip: &str,
    response: &[u8],
    clock: &impl Clock,
) -> ChefResult<CensysIpImportResult> {
    let parsed = parse_censys_ip_host_response(target_ip, response)?;
    let event_count = 1
        + parsed.dns_name_hints.len()
        + parsed.port_hints.len()
        + parsed.url_hints.len()
        + parsed.technology_hints.len()
        + parsed.protocol_hints.len();
    if event_count > MAX_EVENTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Censys supera el límite de eventos CHEF",
        ));
    }
    let response_sha256 = crate::identity::digest(response);
    let imported_at = clock.now();
    let mut root = new_event(
        "CENSYS_HOST_RESPONSE",
        json!({
            "provider": "censys",
            "targetIp": parsed.target_ip,
            "responseSha256": response_sha256,
            "sourceTimestamp": "unknown",
        }),
        json!({
            "provider": "censys",
            "responseSha256": response_sha256,
            "temporalQualification": "source_time_unknown",
        }),
        None,
        None,
        None,
        imported_at.clone(),
        "/result".to_owned(),
        0,
        0,
    )?;
    root.tags.push("chef:provider-snapshot".to_owned());
    let root_id = root.id.clone();
    let mut events = vec![root];

    for (index, hint) in parsed.dns_name_hints.iter().enumerate() {
        let source = match hint.source {
            CensysDnsNameSource::CertificateSan => "certificate_san",
            CensysDnsNameSource::CertificateCommonName => "certificate_common_name",
            CensysDnsNameSource::ReverseDns => "reverse_dns",
        };
        events.push(child_event(
            "DNS_NAME_HINT",
            Value::String(hint.hostname.clone()),
            json!({ "sourceKind": source }),
            Some(hint.hostname.clone()),
            None,
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/dnsNameHints/{index}"),
        )?);
    }

    for (index, hint) in parsed.port_hints.iter().enumerate() {
        let is_tcp = hint.transport == CensysIpTransport::Tcp;
        let transport = if is_tcp { "tcp" } else { "udp" };
        events.push(child_event(
            if is_tcp {
                "TCP_PORT_HINT"
            } else {
                "UDP_PORT_HINT"
            },
            json!({ "port": hint.port, "transport": transport }),
            json!({ "sourceKind": "service_port" }),
            Some(parsed.target_ip.clone()),
            Some(hint.port),
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/portHints/{index}"),
        )?);
    }

    for (index, hint) in parsed.url_hints.iter().enumerate() {
        events.push(child_event(
            "URL_HINT",
            Value::String(hint.clone()),
            json!({ "sourceKind": "http_origin" }),
            Some(parsed.target_ip.clone()),
            None,
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/urlHints/{index}"),
        )?);
    }

    for (index, hint) in parsed.technology_hints.iter().enumerate() {
        events.push(child_event(
            "TECHNOLOGY_HINT",
            Value::String(hint.clone()),
            json!({ "sourceKind": "software_fingerprint" }),
            Some(parsed.target_ip.clone()),
            None,
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/technologyHints/{index}"),
        )?);
    }

    for (index, hint) in parsed.protocol_hints.iter().enumerate() {
        events.push(child_event(
            "PROTOCOL_HINT",
            json!({ "protocol": hint.protocol, "port": hint.port }),
            json!({ "sourceKind": "service_protocol" }),
            Some(parsed.target_ip.clone()),
            hint.port,
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/protocolHints/{index}"),
        )?);
    }

    let graph = BbotEventGraph {
        events,
        modules_run: 0,
        duplicates_suppressed: 0,
    };
    if serde_json::to_vec(&graph)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Grafo Censys inválido"))?
        .len()
        > MAX_GRAPH_BYTES
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El grafo Censys supera el máximo de bytes CHEF",
        ));
    }
    crate::validate_bbot_event_graph(&graph)?;
    let recon_graph = adapt_bbot_to_chef(&graph)?;

    Ok(CensysIpImportResult {
        target_ip: parsed.target_ip,
        response_sha256,
        event_graph: graph,
        recon_graph,
    })
}

/// Valida que una pista de esta ruta conserva la marca de snapshot CHEF.
pub(crate) fn is_censys_snapshot_hint(event: &BbotEvent) -> bool {
    event.module.as_deref() == Some(MODULE)
        && event.tags.iter().any(|tag| tag == SNAPSHOT_TAG)
        && event.metadata.get("provider").and_then(Value::as_str) == Some("censys")
        && event
            .metadata
            .get("responseSha256")
            .and_then(Value::as_str)
            .is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}

#[allow(clippy::too_many_arguments)]
fn child_event(
    event_type: &str,
    data: Value,
    source: Value,
    host: Option<String>,
    port: Option<u16>,
    response_sha256: &str,
    parent_id: &str,
    timestamp: &str,
    locator: String,
) -> ChefResult<BbotEvent> {
    new_event(
        event_type,
        data,
        json!({
            "provider": "censys",
            "responseSha256": response_sha256,
            "source": source,
            "temporalQualification": "source_time_unknown",
        }),
        host,
        port,
        Some(parent_id.to_owned()),
        timestamp.to_owned(),
        locator,
        1,
        1,
    )
}

#[allow(clippy::too_many_arguments)]
fn new_event(
    event_type: &str,
    data: Value,
    metadata: Value,
    host: Option<String>,
    port: Option<u16>,
    parent_id: Option<String>,
    timestamp: String,
    locator: String,
    depth: usize,
    scope_distance: u16,
) -> ChefResult<BbotEvent> {
    let mut event = BbotEvent {
        id: String::new(),
        source_id: None,
        source_uuid: None,
        event_type: event_type.to_owned(),
        data,
        data_redacted: false,
        metadata,
        metadata_redacted: false,
        host,
        port,
        parent_id,
        parent_source_id: None,
        parent_source_uuid: None,
        timestamp,
        tags: vec![SNAPSHOT_TAG.to_owned()],
        module: Some(MODULE.to_owned()),
        locator,
        depth,
        scope_distance,
    };
    event.id = crate::event_engine::event_id(&event)?;
    Ok(event)
}
