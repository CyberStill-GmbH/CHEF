//! Convierte un resultado urlscan archivado a evidencia CHEF con linaje.
//!
//! La importación registra nombres y orígenes como pistas históricas. No
//! consulta urlscan, no comprueba DNS y no transforma candidatos en activos
//! observados.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    BbotEvent, BbotEventGraph, ChefError, ChefReconGraph, ChefResult, Clock, ErrorCode,
    parse_urlscan_subdomains, recon::adapt_bbot_to_chef,
};

const MAX_EVENTS: usize = 10_000;
const MAX_GRAPH_BYTES: usize = 8 * 1024 * 1024;
const MODULE: &str = "chef_urlscan_snapshot";
const SNAPSHOT_TAG: &str = "chef:provider-snapshot-hint";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UrlScanImportResult {
    pub query_domain: String,
    pub response_sha256: String,
    pub event_graph: BbotEventGraph,
    pub recon_graph: ChefReconGraph,
}

pub(crate) fn is_urlscan_snapshot_hint(event: &BbotEvent) -> bool {
    event.module.as_deref() == Some(MODULE)
        && event.tags.iter().any(|tag| tag == SNAPSHOT_TAG)
        && event.metadata.get("provider").and_then(Value::as_str) == Some("urlscan")
        && event
            .metadata
            .get("responseSha256")
            .and_then(Value::as_str)
            .is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
}

/// Convierte bytes archivados urlscan en eventos CHEF deterministas y una
/// proyección con evidencia. La importación no concede permiso de red.
pub fn import_urlscan_snapshot(
    query_domain: &str,
    response: &[u8],
    clock: &impl Clock,
) -> ChefResult<UrlScanImportResult> {
    let parsed = parse_urlscan_subdomains(query_domain, response)?;
    let event_count = 1 + parsed.candidates.len() + parsed.url_hints.len();
    if event_count > MAX_EVENTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta urlscan supera el límite de eventos CHEF",
        ));
    }

    let response_sha256 = crate::identity::digest(response);
    let imported_at = clock.now();
    let mut root = new_event(
        "URLSCAN_SEARCH_RESPONSE",
        json!({
            "provider": "urlscan",
            "queryDomain": parsed.query_domain,
            "responseSha256": response_sha256,
            "sourceTimestamp": "unknown",
        }),
        json!({
            "provider": "urlscan",
            "responseSha256": response_sha256,
            "temporalQualification": "source_time_unknown",
        }),
        None,
        None,
        imported_at.clone(),
        "/results".to_owned(),
        0,
        0,
    )?;
    root.tags.push(SNAPSHOT_TAG.to_owned());
    let root_id = root.id.clone();
    let mut events = vec![root];

    for (index, candidate) in parsed.candidates.iter().enumerate() {
        events.push(child_event(
            "DNS_NAME_HINT",
            Value::String(candidate.clone()),
            json!({ "sourceKind": "urlscan_search_result" }),
            Some(candidate.clone()),
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/candidates/{index}"),
        )?);
    }

    for (index, url_hint) in parsed.url_hints.iter().enumerate() {
        let host = url::Url::parse(url_hint)
            .ok()
            .and_then(|url| url.host_str().map(ToOwned::to_owned));
        events.push(child_event(
            "URL_HINT",
            Value::String(url_hint.clone()),
            json!({ "sourceKind": "urlscan_http_origin" }),
            host,
            &response_sha256,
            &root_id,
            &imported_at,
            format!("/normalized/urlHints/{index}"),
        )?);
    }

    let event_graph = BbotEventGraph {
        events,
        modules_run: 0,
        duplicates_suppressed: 0,
    };
    if serde_json::to_vec(&event_graph)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Grafo urlscan inválido"))?
        .len()
        > MAX_GRAPH_BYTES
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El grafo urlscan supera el máximo de bytes CHEF",
        ));
    }
    crate::validate_bbot_event_graph(&event_graph)?;
    let recon_graph = adapt_bbot_to_chef(&event_graph)?;

    Ok(UrlScanImportResult {
        query_domain: parsed.query_domain,
        response_sha256,
        event_graph,
        recon_graph,
    })
}

#[allow(clippy::too_many_arguments)]
fn child_event(
    event_type: &str,
    data: Value,
    source: Value,
    host: Option<String>,
    response_sha256: &str,
    parent_id: &str,
    timestamp: &str,
    locator: String,
) -> ChefResult<BbotEvent> {
    new_event(
        event_type,
        data,
        json!({
            "provider": "urlscan",
            "responseSha256": response_sha256,
            "source": source,
            "temporalQualification": "source_time_unknown",
        }),
        host,
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
        port: None,
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
