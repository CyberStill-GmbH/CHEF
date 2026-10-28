//! Importador offline de eventos BBOT; no contiene módulos de escaneo ni acceso a red.

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;

use chrono::{DateTime, SecondsFormat, Utc};
use serde::Deserialize;
use serde_json::{Map, Value};
use url::{Host, Url};

use crate::{
    domain::{ChefError, ChefResult, ErrorCode, ServiceInput},
    event_engine::{
        BbotEvent, BbotEventGraph, event_id, sanitize_event_data, valid_event_type, validate_event,
    },
};

const MAX_LINE_BYTES: usize = 128 * 1024;
const MAX_EVENT_DEPTH: usize = 32;
const DEFAULT_BBOT_SCOPE_DISTANCE: u16 = 10;

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
}

/// Lee el flujo de eventos BBOT conservando tipos, datos, metadatos y linaje.
/// Los datos adicionales quedan dentro de `data` cuando la fuente los incluye
/// allí; los campos reconocidos se normalizan como propiedades del evento.
pub fn parse_event_graph(bytes: &[u8], max_events: usize) -> ChefResult<BbotEventGraph> {
    if max_events == 0 {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El máximo de eventos BBOT debe ser positivo",
        ));
    }
    let input = std::str::from_utf8(bytes)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "BBOT JSONL debe ser UTF-8"))?;
    let mut events = Vec::new();
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
    Ok(BbotEventGraph {
        events,
        modules_run: 0,
        duplicates_suppressed: 0,
    })
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
    let url = Url::parse(&format!("tcp://{value}")).map_err(|_| invalid_port_netloc())?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !url.path().is_empty()
    {
        return Err(invalid_port_netloc());
    }
    let host = match url.host().ok_or_else(invalid_port_netloc)? {
        Host::Domain(domain) => domain.to_owned(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => address.to_string(),
    };
    let port = url
        .port()
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
pub fn parse(bytes: &[u8], max_observations: usize) -> ChefResult<ParsedBbotEvents> {
    let event_graph = parse_event_graph(bytes, max_observations)?;
    let mut services = Vec::new();

    for event in &event_graph.events {
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

    Ok(ParsedBbotEvents {
        services,
        event_graph,
    })
}
