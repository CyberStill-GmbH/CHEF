//! Importador offline de eventos BBOT; no contiene módulos de escaneo ni acceso a red.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use crate::{
    domain::{ChefError, ChefResult, ErrorCode, ServiceInput},
    event_engine::{
        BbotEvent, BbotEventGraph, event_id, sanitize_event_data, valid_event_type, validate_event,
    },
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};

const MAX_LINE_BYTES: usize = 128 * 1024;
const MAX_EVENT_DEPTH: usize = 32;
const DEFAULT_BBOT_SCOPE_DISTANCE: u16 = 10;
const MAX_ARCHIVED_GIT_HINTS: usize = 2048;
const MAX_ARCHIVED_ROBOTS_HINTS: usize = 4096;
const MAX_ARCHIVED_WEB_PARAMETER_HINTS: usize = 4096;
const MAX_ARCHIVED_WEB_MARKER_HINTS: usize = 2048;
const MAX_ARCHIVED_SECURITY_CONTACT_HINTS: usize = 2048;

#[derive(Debug)]
pub(crate) enum ArchivedGitImportHint {
    ConfigExposure {
        parent_event_id: String,
        host: String,
        config_url: String,
        repository_directory_url: String,
    },
    DirectoryEntry {
        parent_event_id: String,
        host: String,
        url: String,
        is_directory: bool,
    },
}

#[derive(Debug)]
pub(crate) struct ArchivedRobotsImportHint {
    pub parent_event_id: String,
    pub host: String,
    pub url: String,
    pub kind: crate::RobotsHintKind,
}

#[derive(Debug)]
pub(crate) struct ArchivedWebParameterHint {
    pub parent_event_id: String,
    pub host: String,
    pub name: String,
}

#[derive(Debug)]
pub(crate) enum ArchivedWebMarkerHint {
    AjaxProPath {
        parent_event_id: String,
        host: String,
        url: String,
    },
    ComponentMarkers {
        parent_event_id: String,
        host: String,
        origin: String,
        kind: &'static str,
        title: &'static str,
        markers: Vec<String>,
    },
}

#[derive(Debug)]
pub(crate) struct ArchivedSecurityContactHint {
    pub parent_event_id: String,
    pub host: String,
    pub url: String,
}

#[derive(Debug)]
enum TransientGitArchiveHint {
    ConfigExposure {
        host: String,
        config_url: String,
        repository_directory_url: String,
    },
    DirectoryEntry {
        host: String,
        url: String,
        is_directory: bool,
    },
}

#[derive(Debug)]
enum TransientArchivedWebMarkerHint {
    AjaxProPath {
        host: String,
        url: String,
    },
    ComponentMarkers {
        host: String,
        origin: String,
        kind: &'static str,
        title: &'static str,
        markers: Vec<String>,
    },
}

#[derive(Debug)]
struct TransientHttpResponse {
    response_event_id: String,
    response_url: String,
    status_code: Option<u16>,
    body: String,
}

#[derive(Debug, Deserialize)]
struct RawEvent {
    #[serde(rename = "type")]
    event_type: String,
    id: Option<String>,
    uuid: Option<String>,
    data: Option<Value>,
    data_json: Option<Value>,
    host: Option<String>,
    port: Option<u16>,
    parent: Option<String>,
    parent_uuid: Option<String>,
    tags: Option<Vec<String>>,
    module: Option<String>,
    timestamp: Value,
    #[serde(default = "default_bbot_scope_distance")]
    scope_distance: u16,
    #[serde(flatten)]
    metadata: Map<String, Value>,
}

fn default_bbot_scope_distance() -> u16 {
    DEFAULT_BBOT_SCOPE_DISTANCE
}

#[derive(Debug)]
pub struct ParsedBbotEvents {
    pub services: Vec<ServiceInput>,
    pub event_graph: BbotEventGraph,
    pub(crate) archived_git_hints: Vec<ArchivedGitImportHint>,
    pub(crate) archived_robots_hints: Vec<ArchivedRobotsImportHint>,
    pub(crate) archived_web_parameter_hints: Vec<ArchivedWebParameterHint>,
    pub(crate) archived_web_marker_hints: Vec<ArchivedWebMarkerHint>,
    pub(crate) archived_security_contact_hints: Vec<ArchivedSecurityContactHint>,
}

/// Lee el flujo de eventos BBOT conservando tipos, datos, metadatos y linaje.
/// Los datos adicionales quedan dentro de `data` cuando la fuente los incluye
/// allí; los campos reconocidos se normalizan como propiedades del evento.
pub fn parse_event_graph(bytes: &[u8], max_events: usize) -> ChefResult<BbotEventGraph> {
    Ok(
        parse_event_graph_with_hints(bytes, max_events, false, false, false, false, false)?
            .event_graph,
    )
}

fn parse_event_graph_with_hints(
    bytes: &[u8],
    max_events: usize,
    collect_archived_git_hints: bool,
    collect_archived_robots_hints: bool,
    collect_archived_web_parameter_hints: bool,
    collect_archived_web_marker_hints: bool,
    collect_archived_security_contact_hints: bool,
) -> ChefResult<ParsedBbotEvents> {
    if max_events == 0 {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El máximo de eventos BBOT debe ser positivo",
        ));
    }
    let input = std::str::from_utf8(bytes)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "BBOT JSONL debe ser UTF-8"))?;
    let mut events = Vec::new();
    let mut archived_http_responses = Vec::new();
    let mut archived_web_parameter_hints = Vec::new();
    let mut source_ids = HashMap::<String, usize>::new();
    let mut source_uuids = HashMap::<String, usize>::new();

    for (line_index, line) in input.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Registro BBOT supera el límite por línea",
            ));
        }
        if events.len() >= max_events {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "BBOT JSONL supera el máximo de eventos CHEF",
            ));
        }
        let mut raw: RawEvent = serde_json::from_str(line)
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Registro BBOT JSONL inválido"))?;
        if !valid_event_type(&raw.event_type) {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Tipo de evento BBOT inválido",
            ));
        }
        if raw.scope_distance > 32 {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La distancia de scope BBOT supera el máximo CHEF",
            ));
        }
        let timestamp = parse_timestamp(&raw.timestamp)?;
        let tags = raw.tags.take().unwrap_or_default();
        let source_id = raw.id.take();
        let source_uuid = raw.uuid.take();
        let parent_source_id = raw.parent.take();
        let parent_source_uuid = raw.parent_uuid.take();
        let data = raw
            .data_json
            .take()
            .or_else(|| raw.data.take())
            .ok_or_else(|| {
                ChefError::new(ErrorCode::InvalidInput, "Evento BBOT sin data/data_json")
            })?;
        let transient_web_parameters = if collect_archived_web_parameter_hints
            && raw.event_type == "URL"
        {
            data.as_str()
                .and_then(|value| crate::parse_archived_web_parameters(value).ok())
                .and_then(|parsed| {
                    let url = url::Url::parse(&parsed.url_without_query).ok()?;
                    let host = url.host_str()?.to_owned();
                    if raw.host.as_deref().is_some_and(|declared| {
                        crate::host::canonical_host(declared) != crate::host::canonical_host(&host)
                    }) {
                        return None;
                    }
                    Some((host, parsed.parameter_names))
                })
        } else {
            None
        };
        let transient_http_response = if (collect_archived_git_hints
            || collect_archived_robots_hints
            || collect_archived_web_marker_hints
            || collect_archived_security_contact_hints)
            && raw.event_type == "HTTP_RESPONSE"
        {
            transient_http_response(&data)
        } else {
            None
        };
        let (host, port) =
            complete_open_port_fields(&raw.event_type, &data, raw.host.take(), raw.port.take())?;
        let (data, data_redacted) = sanitize_event_data(&raw.event_type, data);
        let (metadata, metadata_redacted) =
            sanitize_event_data(&raw.event_type, Value::Object(raw.metadata));
        let mut event = BbotEvent {
            id: String::new(),
            source_id,
            source_uuid,
            event_type: raw.event_type,
            data,
            data_redacted,
            metadata,
            metadata_redacted,
            host,
            port,
            parent_id: None,
            parent_source_id,
            parent_source_uuid,
            timestamp,
            tags,
            module: raw.module,
            locator: format!("/events/{}", line_index + 1),
            depth: 0,
            // Pydantic BBOT Event defaults missing distance to 10; absent
            // metadata must not be misread as a root seed (distance 0).
            scope_distance: raw.scope_distance,
        };
        validate_event(&event)?;
        event.id = event_id(&event)?;
        if let Some((host, names)) = transient_web_parameters {
            for name in names {
                archived_web_parameter_hints.push(ArchivedWebParameterHint {
                    parent_event_id: event.id.clone(),
                    host: host.clone(),
                    name,
                });
                if archived_web_parameter_hints.len() > MAX_ARCHIVED_WEB_PARAMETER_HINTS {
                    return Err(ChefError::new(
                        ErrorCode::LimitExceeded,
                        "La importación supera el máximo de nombres de parámetros web",
                    ));
                }
            }
        }
        if let Some(mut response) = transient_http_response {
            response.response_event_id = event.id.clone();
            archived_http_responses.push(response);
            if archived_http_responses.len() > max_events {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La importación supera el máximo de respuestas HTTP archivadas",
                ));
            }
        }
        if let Some(source_id) = &event.source_id {
            if source_id.is_empty() || source_id.len() > 256 || source_ids.contains_key(source_id) {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Identificador de evento BBOT inválido o duplicado",
                ));
            }
            source_ids.insert(source_id.clone(), events.len());
        }
        if let Some(source_uuid) = &event.source_uuid {
            let uuid = normalize_uuid(source_uuid);
            if uuid.is_empty() || uuid.len() > 128 || source_uuids.contains_key(uuid) {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "UUID de evento BBOT inválido o duplicado",
                ));
            }
            source_uuids.insert(uuid.to_owned(), events.len());
        }
        events.push(event);
    }

    for index in 0..events.len() {
        let mut cursor = index;
        let mut ancestors = HashSet::new();
        let mut lineage = Vec::new();
        while let Some(parent_index) = find_parent(&events[cursor], &source_ids, &source_uuids) {
            if !ancestors.insert(cursor) || parent_index == index {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El linaje de eventos BBOT contiene un ciclo",
                ));
            }
            lineage.push(parent_index);
            if lineage.len() > MAX_EVENT_DEPTH {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El linaje de eventos BBOT supera la profundidad CHEF",
                ));
            }
            cursor = parent_index;
        }
        if let Some(parent_index) = lineage.first().copied() {
            events[index].parent_id = Some(events[parent_index].id.clone());
            events[index].depth = lineage.len();
        }
    }
    if events.is_empty() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "BBOT JSONL no contiene eventos",
        ));
    }
    let mut archived_git_hints = Vec::new();
    let mut archived_robots_hints = Vec::new();
    let mut archived_web_marker_hints = Vec::new();
    let mut archived_security_contact_hints = Vec::new();
    if collect_archived_git_hints
        || collect_archived_robots_hints
        || collect_archived_web_marker_hints
        || collect_archived_security_contact_hints
    {
        let events_by_id = events
            .iter()
            .map(|event| (event.id.as_str(), event))
            .collect::<HashMap<_, _>>();
        for response in archived_http_responses {
            let Some(response_event) = events_by_id.get(response.response_event_id.as_str()) else {
                continue;
            };
            let Some(parent_id) = response_event.parent_id.as_deref() else {
                continue;
            };
            let Some(target_event) = events_by_id.get(parent_id) else {
                continue;
            };
            if target_event.event_type != "URL" {
                continue;
            }
            let Some(target_url) = target_event.data.as_str() else {
                continue;
            };
            if collect_archived_git_hints {
                for hint in inspect_archived_git_response(
                    target_url,
                    &response.response_url,
                    response.status_code,
                    response.body.as_bytes(),
                ) {
                    let archived_hint = match hint {
                        TransientGitArchiveHint::ConfigExposure {
                            host,
                            config_url,
                            repository_directory_url,
                        } => ArchivedGitImportHint::ConfigExposure {
                            parent_event_id: response_event.id.clone(),
                            host,
                            config_url,
                            repository_directory_url,
                        },
                        TransientGitArchiveHint::DirectoryEntry {
                            host,
                            url,
                            is_directory,
                        } => ArchivedGitImportHint::DirectoryEntry {
                            parent_event_id: response_event.id.clone(),
                            host,
                            url,
                            is_directory,
                        },
                    };
                    archived_git_hints.push(archived_hint);
                    if archived_git_hints.len() > MAX_ARCHIVED_GIT_HINTS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La importación supera el máximo de pistas Git archivadas",
                        ));
                    }
                }
            }
            if collect_archived_robots_hints
                && response.status_code == Some(200)
                && let Some(origin) = archived_robots_origin(target_url, &response.response_url)
                && let Ok(parsed) = crate::parse_archived_robots_hints(&response.body, &origin)
            {
                let Some(host) = url::Url::parse(&origin)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_owned))
                else {
                    continue;
                };
                for hint in parsed.hints {
                    let Some(url) = hint.url_hint else {
                        continue;
                    };
                    archived_robots_hints.push(ArchivedRobotsImportHint {
                        parent_event_id: response_event.id.clone(),
                        host: host.clone(),
                        url,
                        kind: hint.kind,
                    });
                    if archived_robots_hints.len() > MAX_ARCHIVED_ROBOTS_HINTS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La importación supera el máximo de pistas robots archivadas",
                        ));
                    }
                }
            }
            if collect_archived_web_marker_hints && response.status_code == Some(200) {
                for hint in inspect_archived_web_markers(
                    target_url,
                    &response.response_url,
                    response.body.as_bytes(),
                ) {
                    archived_web_marker_hints.push(match hint {
                        TransientArchivedWebMarkerHint::AjaxProPath { host, url } => {
                            ArchivedWebMarkerHint::AjaxProPath {
                                parent_event_id: response_event.id.clone(),
                                host,
                                url,
                            }
                        }
                        TransientArchivedWebMarkerHint::ComponentMarkers {
                            host,
                            origin,
                            kind,
                            title,
                            markers,
                        } => ArchivedWebMarkerHint::ComponentMarkers {
                            parent_event_id: response_event.id.clone(),
                            host,
                            origin,
                            kind,
                            title,
                            markers,
                        },
                    });
                    if archived_web_marker_hints.len() > MAX_ARCHIVED_WEB_MARKER_HINTS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La importación supera el máximo de pistas de marcadores web archivados",
                        ));
                    }
                }
            }
            if collect_archived_security_contact_hints
                && response.status_code == Some(200)
                && let Some(origin) =
                    archived_security_txt_origin(target_url, &response.response_url)
                && let Ok(parsed) =
                    crate::parse_archived_security_contacts(&response.body, &response.response_url)
            {
                let Some(host) = url::Url::parse(&origin)
                    .ok()
                    .and_then(|url| url.host_str().map(str::to_owned))
                else {
                    continue;
                };
                let origin_url = url::Url::parse(&origin).ok();
                for candidate in parsed.url_hints {
                    let Ok(candidate_url) = url::Url::parse(&candidate) else {
                        continue;
                    };
                    if !origin_url
                        .as_ref()
                        .is_some_and(|base| candidate_url.origin() == base.origin())
                    {
                        continue;
                    }
                    archived_security_contact_hints.push(ArchivedSecurityContactHint {
                        parent_event_id: response_event.id.clone(),
                        host: host.clone(),
                        url: candidate,
                    });
                    if archived_security_contact_hints.len() > MAX_ARCHIVED_SECURITY_CONTACT_HINTS {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "La importación supera el máximo de pistas de contacto security.txt",
                        ));
                    }
                }
            }
        }
    }
    Ok(ParsedBbotEvents {
        services: Vec::new(),
        event_graph: BbotEventGraph {
            events,
            modules_run: 0,
            duplicates_suppressed: 0,
        },
        archived_git_hints,
        archived_robots_hints,
        archived_web_parameter_hints,
        archived_web_marker_hints,
        archived_security_contact_hints,
    })
}

fn archived_security_txt_origin(target_url: &str, response_url: &str) -> Option<String> {
    let target = url::Url::parse(target_url).ok()?;
    let response = url::Url::parse(response_url).ok()?;
    if !matches!(target.scheme(), "http" | "https")
        || !target.username().is_empty()
        || target.password().is_some()
        || target.origin() != response.origin()
        || response.path() != "/.well-known/security.txt"
    {
        return None;
    }
    Some(target.origin().ascii_serialization())
}

fn archived_robots_origin(target_url: &str, response_url: &str) -> Option<String> {
    let target = url::Url::parse(target_url).ok()?;
    let response = url::Url::parse(response_url).ok()?;
    if !matches!(target.scheme(), "http" | "https")
        || !target.username().is_empty()
        || target.password().is_some()
        || target.origin() != response.origin()
        || response.path() != "/robots.txt"
    {
        return None;
    }
    Some(target.origin().ascii_serialization())
}

fn inspect_archived_web_markers(
    target_url: &str,
    response_url: &str,
    body: &[u8],
) -> Vec<TransientArchivedWebMarkerHint> {
    let Some(origin) = same_origin_archived_response(target_url, response_url) else {
        return Vec::new();
    };
    let Some(host) = url::Url::parse(&origin)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
    else {
        return Vec::new();
    };
    let mut hints = Vec::new();

    if let Ok(telerik) = crate::parse_archived_telerik_response(body)
        && !telerik.markers.is_empty()
    {
        hints.push(TransientArchivedWebMarkerHint::ComponentMarkers {
            host: host.clone(),
            origin: origin.clone(),
            kind: "telerik_response_markers",
            title: "Marcadores Telerik en respuesta archivada",
            markers: telerik
                .markers
                .into_iter()
                .map(|marker| format!("{:?}:{}", marker.marker, marker.occurrences))
                .collect(),
        });
    }

    if let Ok(body_text) = std::str::from_utf8(body) {
        if let Ok(dotnetnuke) = crate::parse_archived_dotnetnuke_response("", body_text)
            && !dotnetnuke.markers.is_empty()
        {
            hints.push(TransientArchivedWebMarkerHint::ComponentMarkers {
                host: host.clone(),
                origin: origin.clone(),
                kind: "dotnetnuke_response_markers",
                title: "Marcadores DotNetNuke en respuesta archivada",
                markers: dotnetnuke
                    .markers
                    .into_iter()
                    .map(|marker| format!("{marker:?}"))
                    .collect(),
            });
        }

        if let Ok(ajaxpro) = crate::parse_archived_ajaxpro_html(body_text) {
            let Ok(base) = url::Url::parse(&origin) else {
                return hints;
            };
            for path in ajaxpro.script_paths {
                if let Ok(url) = base.join(&path)
                    && url.origin() == base.origin()
                {
                    hints.push(TransientArchivedWebMarkerHint::AjaxProPath {
                        host: host.clone(),
                        url: url.to_string(),
                    });
                }
            }
        }
    }
    hints
}

fn same_origin_archived_response(target_url: &str, response_url: &str) -> Option<String> {
    let target = url::Url::parse(target_url).ok()?;
    let response = url::Url::parse(response_url).ok()?;
    if !matches!(target.scheme(), "http" | "https")
        || !target.username().is_empty()
        || target.password().is_some()
        || target.origin() != response.origin()
    {
        return None;
    }
    Some(target.origin().ascii_serialization())
}

fn transient_http_response(data: &Value) -> Option<TransientHttpResponse> {
    let object = data.as_object()?;
    let response_url = object.get("url").and_then(Value::as_str)?;
    let body = object.get("body").and_then(Value::as_str)?;
    let response_url = crate::url_tools::canonical_http_url(response_url)?;
    let status_code = object
        .get("status_code")
        .and_then(Value::as_u64)
        .and_then(|status| u16::try_from(status).ok());
    Some(TransientHttpResponse {
        response_event_id: String::new(),
        response_url,
        status_code,
        body: body.to_owned(),
    })
}

fn inspect_archived_git_response(
    target_url: &str,
    response_url: &str,
    status_code: Option<u16>,
    body: &[u8],
) -> Vec<TransientGitArchiveHint> {
    let Some(host) = url::Url::parse(response_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned))
    else {
        return Vec::new();
    };
    let mut hints = Vec::new();

    if let Some(status_code) = status_code
        && let Ok(Some(exposure)) =
            crate::classify_archived_git_config(target_url, response_url, status_code, body)
    {
        hints.push(TransientGitArchiveHint::ConfigExposure {
            host: host.clone(),
            config_url: exposure.config_url,
            repository_directory_url: exposure.repository_directory_url,
        });
    }

    if status_code == Some(200)
        && let Ok(Some(listing)) = crate::parse_archived_git_directory_listing(response_url, body)
    {
        hints.extend(listing.entries.into_iter().map(|entry| {
            TransientGitArchiveHint::DirectoryEntry {
                host: host.clone(),
                url: entry.url,
                is_directory: entry.is_directory,
            }
        }));
    }
    hints
}

/// BBOT suele serializar `host` y `port` por separado; completa campos ausentes
/// desde el netloc del propio evento, sin DNS ni solicitudes. Si ambos lugares
/// aportan un valor, deben describir el mismo servicio para evitar evidencia
/// contradictoria.
fn complete_open_port_fields(
    event_type: &str,
    data: &Value,
    host: Option<String>,
    port: Option<u16>,
) -> ChefResult<(Option<String>, Option<u16>)> {
    if !matches!(event_type, "OPEN_TCP_PORT" | "OPEN_UDP_PORT") {
        return Ok((host, port));
    }

    let value = data.as_str().ok_or_else(|| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "Evento BBOT de puerto abierto sin netloc textual",
        )
    })?;
    let (data_host, data_port) = parse_port_netloc(value)?;
    if host.as_deref().is_some_and(|explicit_host| {
        crate::host::canonical_host(explicit_host) != crate::host::canonical_host(&data_host)
    }) || port.is_some_and(|explicit_port| explicit_port != data_port)
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El netloc del evento BBOT contradice su host o puerto explícito",
        ));
    }
    Ok((host.or(Some(data_host)), port.or(Some(data_port))))
}

fn parse_port_netloc(value: &str) -> ChefResult<(String, u16)> {
    if value.is_empty()
        || value.len() > 8_192
        || value.contains('\\')
        || value
            .chars()
            .any(|character| matches!(character, '/' | '?' | '#'))
    {
        return Err(invalid_port_netloc());
    }
    let (host, port) = crate::host::split_host_port(value).ok_or_else(invalid_port_netloc)?;
    let port = port
        .filter(|port| *port > 0)
        .ok_or_else(invalid_port_netloc)?;
    Ok((host, port))
}

fn invalid_port_netloc() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "Netloc de puerto abierto BBOT inválido",
    )
}

fn parse_timestamp(value: &Value) -> ChefResult<String> {
    let date = if let Some(seconds) = value.as_f64() {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Fecha BBOT inválida",
            ));
        }
        let millis = (seconds * 1000.0).round() as i64;
        DateTime::<Utc>::from_timestamp_millis(millis)
            .ok_or_else(|| ChefError::new(ErrorCode::InvalidInput, "Fecha BBOT fuera de rango"))?
    } else if let Some(text) = value.as_str() {
        DateTime::parse_from_rfc3339(text)
            .map(|date| date.with_timezone(&Utc))
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Fecha BBOT inválida"))?
    } else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Fecha BBOT inválida",
        ));
    };
    Ok(date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn normalize_uuid(value: &str) -> &str {
    value.rsplit_once(':').map_or(value, |(_, uuid)| uuid)
}

fn find_parent(
    child: &BbotEvent,
    source_ids: &HashMap<String, usize>,
    source_uuids: &HashMap<String, usize>,
) -> Option<usize> {
    child
        .parent_source_uuid
        .as_deref()
        .and_then(|uuid| source_uuids.get(normalize_uuid(uuid)).copied())
        .or_else(|| {
            child
                .parent_source_id
                .as_deref()
                .and_then(|id| source_ids.get(id).copied())
        })
}

/// Lee cualquier tipo de evento BBOT; el snapshot CHEF actual solo modela
/// puertos, por eso los otros tipos permanecen en el grafo de eventos tipado.
pub fn parse(
    bytes: &[u8],
    max_observations: usize,
    collect_archived_git_hints: bool,
    collect_archived_robots_hints: bool,
    collect_archived_web_parameter_hints: bool,
    collect_archived_web_marker_hints: bool,
    collect_archived_security_contact_hints: bool,
) -> ChefResult<ParsedBbotEvents> {
    let mut parsed = parse_event_graph_with_hints(
        bytes,
        max_observations,
        collect_archived_git_hints,
        collect_archived_robots_hints,
        collect_archived_web_parameter_hints,
        collect_archived_web_marker_hints,
        collect_archived_security_contact_hints,
    )?;
    let mut services = Vec::new();

    for event in &parsed.event_graph.events {
        let protocol = match event.event_type.as_str() {
            "OPEN_TCP_PORT" => "tcp",
            "OPEN_UDP_PORT" => "udp",
            _ => continue,
        };
        if services.len() >= max_observations {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "BBOT JSONL supera el máximo de observaciones CHEF",
            ));
        }
        let host = event
            .host
            .as_deref()
            .ok_or_else(|| ChefError::new(ErrorCode::InvalidInput, "Evento BBOT sin host"))?;
        let address = host
            .parse::<IpAddr>()
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Host BBOT debe ser IP literal"))?
            .to_string();
        let port = event
            .port
            .filter(|port| *port > 0)
            .ok_or_else(|| ChefError::new(ErrorCode::InvalidInput, "Puerto BBOT inválido"))?;
        services.push(ServiceInput {
            address,
            protocol: protocol.to_owned(),
            port,
            observed_at: event.timestamp.clone(),
            locator: event.locator.clone(),
        });
    }

    parsed.services = services;
    Ok(parsed)
}
