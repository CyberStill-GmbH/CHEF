//! Motor tipado de eventos CHEF, inspirado en el flujo recursivo de BBOT.
//!
//! Los módulos se registran en compilación. El motor no ofrece filesystem,
//! sockets, shell ni carga dinámica; las capacidades externas pertenecen a
//! adaptadores de aplicación y necesitan su propio permiso.

use std::{
    collections::{HashSet, VecDeque},
    sync::atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::statistics::{BbotRuntimeStatistics, RuntimeStatisticsBuilder};
use crate::{ChefConfig, ChefError, ChefResult, Clock, ErrorCode, identity};

const MAX_EVENT_TYPE_BYTES: usize = 64;
const MAX_EVENT_DATA_BYTES: usize = 128 * 1024;
const MAX_EVENT_TAGS: usize = 128;
const MAX_TAG_BYTES: usize = 128;
const MAX_MODULE_NAME_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BbotEvent {
    /// ID determinista CHEF; el identificador BBOT original se conserva aparte.
    pub id: String,
    pub source_id: Option<String>,
    pub source_uuid: Option<String>,
    pub event_type: String,
    pub data: Value,
    pub data_redacted: bool,
    /// Metadatos del formato de exportación BBOT fuera de su campo `data`.
    pub metadata: Value,
    pub metadata_redacted: bool,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub parent_id: Option<String>,
    pub parent_source_id: Option<String>,
    pub parent_source_uuid: Option<String>,
    pub timestamp: String,
    pub tags: Vec<String>,
    pub module: Option<String>,
    pub locator: String,
    pub depth: usize,
    pub scope_distance: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BbotEventDraft {
    pub event_type: String,
    pub data: Value,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ModuleDescriptor {
    pub name: &'static str,
    pub watches: &'static [&'static str],
    pub produces: &'static [&'static str],
}

/// Módulo CHEF puro; dependencias externas se inyectan en la capa adaptadora.
pub trait BbotModule: Send + Sync {
    fn descriptor(&self) -> ModuleDescriptor;

    /// Solo los módulos que garantizan no usar red pueden participar en la
    /// expansión offline de evidencia importada.
    fn offline_only(&self) -> bool {
        false
    }

    fn handle(
        &self,
        event: &BbotEvent,
        config: &ChefConfig,
        clock: &dyn Clock,
    ) -> ChefResult<Vec<BbotEventDraft>>;
}

struct OfflineEvidenceGate;

impl EventScopeGate for OfflineEvidenceGate {
    fn authorize(&self, _event: &BbotEvent) -> ChefResult<()> {
        // El caller verifica que todos los módulos registrados son offline-only.
        Ok(())
    }
}

/// El permiso de scope es obligatorio y lo proporciona la capa de aplicación.
pub trait EventScopeGate {
    fn authorize(&self, event: &BbotEvent) -> ChefResult<()>;
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BbotEventGraph {
    pub events: Vec<BbotEvent>,
    pub modules_run: usize,
    pub duplicates_suppressed: usize,
}

pub(crate) fn validate_event_graph(graph: &BbotEventGraph) -> ChefResult<()> {
    if graph.events.is_empty() || graph.events.len() > 10_000 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Cantidad de eventos CHEF fuera de límites",
        ));
    }
    let serialized = serde_json::to_vec(graph)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Grafo de eventos inválido"))?;
    if serialized.len() > 8 * 1024 * 1024 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El grafo de eventos CHEF supera el máximo de bytes",
        ));
    }
    let event_ids = graph
        .events
        .iter()
        .map(|event| event.id.as_str())
        .collect::<HashSet<_>>();
    if event_ids.len() != graph.events.len()
        || graph.events.iter().any(|event| {
            validate_event(event).is_err()
                || event.depth > 32
                || event.scope_distance > 32
                || event.parent_id.is_none() && event.depth != 0
                || event.parent_id.is_some() && event.depth == 0
                || event
                    .parent_id
                    .as_deref()
                    .is_some_and(|parent_id| !event_ids.contains(parent_id))
        })
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Grafo de eventos CHEF inválido o con referencias ausentes",
        ));
    }
    Ok(())
}

pub struct BbotEventEngine {
    modules: Vec<Box<dyn BbotModule>>,
    max_events: usize,
    max_depth: usize,
}

impl BbotEventEngine {
    pub fn new(max_events: usize, max_depth: usize) -> ChefResult<Self> {
        if max_events == 0 || max_depth == 0 {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Los límites del motor de eventos deben ser positivos",
            ));
        }
        Ok(Self {
            modules: Vec::new(),
            max_events,
            max_depth,
        })
    }

    /// Crea el registro con módulos CHEF puros actualmente portados.
    /// Cada módulo sigue desactivado salvo habilitación explícita en config.
    pub fn with_chef_modules(max_events: usize, max_depth: usize) -> ChefResult<Self> {
        let mut engine = Self::new(max_events, max_depth)?;
        engine.register(Box::new(crate::speculate::ChefIpRangeExpansion))?;
        engine.register(Box::new(crate::speculate::ChefUrlAttributeHints))?;
        engine.register(Box::new(crate::speculate::ChefUrlParentHints))?;
        engine.register(Box::new(crate::speculate::ChefDnsParentHints))?;
        engine.register(Box::new(crate::speculate::ChefDnsRdataHints))?;
        engine.register(Box::new(crate::dns_tlsrpt::ChefDnsTlsRptHints))?;
        engine.register(Box::new(crate::speculate::ChefCommonPortHints))?;
        engine.register(Box::new(crate::speculate::ChefOrgStubHints))?;
        Ok(engine)
    }

    pub fn register(&mut self, module: Box<dyn BbotModule>) -> ChefResult<()> {
        let descriptor = module.descriptor();
        if descriptor.name.is_empty()
            || descriptor.name.len() > MAX_MODULE_NAME_BYTES
            || descriptor.watches.is_empty()
            || descriptor.produces.is_empty()
            || descriptor
                .watches
                .iter()
                .any(|event_type| *event_type != "*" && !valid_event_type(event_type))
            || descriptor
                .produces
                .iter()
                .any(|event_type| !valid_event_type(event_type))
            || self
                .modules
                .iter()
                .any(|registered| registered.descriptor().name == descriptor.name)
        {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Definición de módulo CHEF inválida o duplicada",
            ));
        }
        self.modules.push(module);
        self.modules
            .sort_by(|left, right| left.descriptor().name.cmp(right.descriptor().name));
        Ok(())
    }

    /// Expande eventos mediante módulos registrados, validando permiso, linaje
    /// y límites antes de devolver un grafo completo.
    pub fn run(
        &self,
        seeds: Vec<BbotEvent>,
        config: &ChefConfig,
        scope: &impl EventScopeGate,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<BbotEventGraph> {
        self.run_with_statistics(seeds, config, scope, clock, cancelled)
            .map(|(graph, _statistics)| graph)
    }

    /// Ejecuta solo módulos declarados offline-only sobre evidencia importada.
    /// No concede autorización de red; `ChefConfig::validate` mantiene
    /// `active_discovery` desactivado y el registro completo debe ser offline.
    pub fn run_offline(
        &self,
        seeds: Vec<BbotEvent>,
        config: &ChefConfig,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<BbotEventGraph> {
        if self.modules.iter().any(|module| !module.offline_only()) {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "El motor offline contiene un módulo que no garantiza aislamiento de red",
            ));
        }
        self.run(seeds, config, &OfflineEvidenceGate, clock, cancelled)
    }

    /// Ejecuta el motor y devuelve el agregado de eventos consumidos/producidos
    /// por módulo. Las tasas por ventana no se calculan en un lote sin reloj
    /// monotónico de runtime.
    pub fn run_with_statistics(
        &self,
        seeds: Vec<BbotEvent>,
        config: &ChefConfig,
        scope: &impl EventScopeGate,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<(BbotEventGraph, BbotRuntimeStatistics)> {
        if seeds.is_empty() || seeds.len() > self.max_events {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Cantidad inicial de eventos fuera de límites",
            ));
        }
        config.validate()?;

        let mut pending = VecDeque::from(seeds);
        let mut events = Vec::new();
        let mut identities = HashSet::new();
        let mut modules_run = 0;
        let mut duplicates_suppressed = 0;
        let mut runtime_statistics = RuntimeStatisticsBuilder::default();

        while let Some(mut event) = pending.pop_front() {
            if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                return Err(ChefError::new(
                    ErrorCode::Cancelled,
                    "Motor de eventos cancelado",
                ));
            }
            validate_event(&event)?;
            scope.authorize(&event)?;
            let fingerprint = event_fingerprint(&event)?;
            if !identities.insert(fingerprint) {
                duplicates_suppressed += 1;
                continue;
            }
            if events.len() == self.max_events {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El motor de eventos superó el máximo CHEF",
                ));
            }
            if event.id.is_empty() {
                event.id = event_id(&event)?;
            }
            let parent = event.clone();
            events.push(event);

            for module in &self.modules {
                if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                    return Err(ChefError::new(
                        ErrorCode::Cancelled,
                        "Motor de eventos cancelado",
                    ));
                }
                let descriptor = module.descriptor();
                if !config
                    .modules
                    .get(descriptor.name)
                    .copied()
                    .unwrap_or(false)
                    || !(descriptor.watches.contains(&parent.event_type.as_str())
                        || descriptor.watches.contains(&"*"))
                {
                    continue;
                }
                modules_run += 1;
                runtime_statistics.record_consumed(descriptor.name, &parent.event_type);
                for draft in module.handle(&parent, config, clock)? {
                    if !descriptor.produces.contains(&draft.event_type.as_str()) {
                        return Err(ChefError::new(
                            ErrorCode::InvalidInput,
                            "El módulo emitió un tipo fuera de su contrato",
                        ));
                    }
                    runtime_statistics.record_produced(descriptor.name, &draft.event_type);
                    let (data, data_redacted) = sanitize_event_data(&draft.event_type, draft.data);
                    let scope_distance = if same_host(parent.host.as_deref(), draft.host.as_deref())
                    {
                        parent.scope_distance
                    } else {
                        parent.scope_distance.saturating_add(1)
                    };
                    let child = BbotEvent {
                        id: String::new(),
                        source_id: None,
                        source_uuid: None,
                        event_type: draft.event_type,
                        data,
                        data_redacted,
                        metadata: serde_json::json!({}),
                        metadata_redacted: false,
                        host: draft.host,
                        port: draft.port,
                        parent_id: Some(parent.id.clone()),
                        parent_source_id: None,
                        parent_source_uuid: None,
                        timestamp: clock.now(),
                        tags: draft.tags,
                        module: Some(descriptor.name.to_owned()),
                        locator: String::new(),
                        depth: parent.depth.saturating_add(1),
                        scope_distance,
                    };
                    if child.depth > self.max_depth {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "El motor de eventos superó la profundidad máxima CHEF",
                        ));
                    }
                    validate_event(&child)?;
                    scope.authorize(&child)?;
                    if events.len() + pending.len() >= self.max_events {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "El motor de eventos superó el máximo CHEF",
                        ));
                    }
                    pending.push_back(child);
                }
            }
        }

        let graph = BbotEventGraph {
            events,
            modules_run,
            duplicates_suppressed,
        };
        let statistics = runtime_statistics.finish(modules_run);
        Ok((graph, statistics))
    }
}

pub(crate) fn event_id(event: &BbotEvent) -> ChefResult<String> {
    let data = serde_json::to_string(&event.data)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Datos de evento inválidos"))?;
    let port = event
        .port
        .map(|value| value.to_string())
        .unwrap_or_default();
    let locator = if event.source_id.is_none() {
        event.locator.as_str()
    } else {
        ""
    };
    Ok(identity::id(
        "bbot-event",
        &[
            &event.event_type,
            &data,
            event.source_id.as_deref().unwrap_or_default(),
            event.source_uuid.as_deref().unwrap_or_default(),
            locator,
            event.host.as_deref().unwrap_or_default(),
            &port,
            event
                .parent_id
                .as_deref()
                .or(event.parent_source_id.as_deref())
                .or(event.parent_source_uuid.as_deref())
                .unwrap_or_default(),
        ],
    ))
}

pub(crate) fn sanitize_event_data(event_type: &str, data: Value) -> (Value, bool) {
    if matches!(event_type, "PASSWORD" | "HASHED_PASSWORD") {
        return (serde_json::json!({ "redacted": true }), true);
    }
    if event_type == "HTTP_RESPONSE" {
        return sanitize_http_response(data);
    }
    if matches!(event_type, "URL" | "URL_UNVERIFIED" | "URL_HINT")
        && let Value::String(value) = data
    {
        let safe = sanitize_url(&value);
        return (Value::String(safe.clone()), safe != value);
    }
    sanitize_value(data, None)
}

/// Conserva solo los localizadores y metadatos mínimos que CHEF necesita para
/// representar una respuesta observada. El cuerpo y las cabeceras pueden
/// contener datos de cliente, tokens o información personal.
fn sanitize_http_response(data: Value) -> (Value, bool) {
    let Value::Object(mut input) = data else {
        return (serde_json::json!({ "redacted": true }), true);
    };
    let mut output = serde_json::Map::new();
    if let Some(Value::String(url)) = input.remove("url") {
        let safe = sanitize_url(&url);
        if let Some(origin) = crate::url_tools::canonical_http_origin(&safe) {
            output.insert("url".to_owned(), Value::String(origin));
        }
    }
    if let Some(Value::Number(status)) = input.remove("status_code")
        && status
            .as_u64()
            .is_some_and(|status| (100..=599).contains(&status))
    {
        output.insert("status_code".to_owned(), Value::Number(status));
    }
    if let Some(Value::String(method)) = input.remove("method")
        && !method.is_empty()
        && method.len() <= 16
        && method
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte == b'-')
    {
        output.insert("method".to_owned(), Value::String(method));
    }
    // Se descartan todas las claves restantes, incluso campos futuros que
    // introduzca BBOT. El evento siempre queda marcado como parcialmente
    // redactado aunque no hubiera cuerpo o cabeceras.
    (Value::Object(output), true)
}

fn sanitize_value(value: Value, parent_key: Option<&str>) -> (Value, bool) {
    if parent_key.is_some_and(is_sensitive_key) {
        return (Value::String("[redacted]".to_owned()), true);
    }
    match value {
        Value::Object(object) => {
            let mut changed = false;
            let sanitized = object
                .into_iter()
                .map(|(key, value)| {
                    let (value, redacted) = sanitize_value(value, Some(&key));
                    changed |= redacted;
                    (key, value)
                })
                .collect();
            (Value::Object(sanitized), changed)
        }
        Value::Array(values) => {
            let mut changed = false;
            let sanitized = values
                .into_iter()
                .map(|value| {
                    let (value, redacted) = sanitize_value(value, parent_key);
                    changed |= redacted;
                    value
                })
                .collect();
            (Value::Array(sanitized), changed)
        }
        Value::String(value) if parent_key.is_some_and(is_url_key) => {
            let safe = sanitize_url(&value);
            (Value::String(safe.clone()), safe != value)
        }
        value => (value, false),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = normalize_key(key);
    [
        "password",
        "passwd",
        "secret",
        "token",
        "authorization",
        "cookie",
        "credential",
        "apikey",
        "accesskey",
        "privatekey",
        "rawheader",
        "body",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn is_url_key(key: &str) -> bool {
    normalize_key(key).ends_with("url")
}

/// Remove query/fragment and URL userinfo while retaining a useful locator.
/// This intentionally avoids URL parsing: event data is untrusted and the
/// conservative string operation must never decode or reinterpret it.
fn sanitize_url(value: &str) -> String {
    let without_query = value.split(['?', '#']).next().unwrap_or_default();
    let Some(scheme_end) = without_query.find("://") else {
        return without_query.to_owned();
    };
    let authority_start = scheme_end + 3;
    let authority_end = without_query[authority_start..]
        .find('/')
        .map(|index| authority_start + index)
        .unwrap_or(without_query.len());
    let authority = &without_query[authority_start..authority_end];
    let Some(at_index) = authority.rfind('@') else {
        return without_query.to_owned();
    };
    format!(
        "{}{}{}",
        &without_query[..authority_start],
        &authority[at_index + 1..],
        &without_query[authority_end..]
    )
}

fn normalize_key(key: &str) -> String {
    let normalized = key
        .bytes()
        .filter(|byte| byte.is_ascii_alphanumeric())
        .map(|byte| byte.to_ascii_lowercase())
        .collect::<Vec<_>>();
    String::from_utf8_lossy(&normalized).into_owned()
}

fn same_host(left: Option<&str>, right: Option<&str>) -> bool {
    left.zip(right).is_some_and(|(left, right)| {
        left.trim_end_matches('.')
            .eq_ignore_ascii_case(right.trim_end_matches('.'))
    })
}

fn event_fingerprint(event: &BbotEvent) -> ChefResult<String> {
    event_id(event)
}

pub(crate) fn valid_event_type(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_EVENT_TYPE_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

pub(crate) fn validate_event(event: &BbotEvent) -> ChefResult<()> {
    let data_size = serde_json::to_vec(&event.data)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Datos de evento inválidos"))?
        .len();
    let metadata_size = serde_json::to_vec(&event.metadata)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Metadatos de evento inválidos"))?
        .len();
    if !valid_event_type(&event.event_type)
        || data_size > MAX_EVENT_DATA_BYTES
        || metadata_size > MAX_EVENT_DATA_BYTES
        || event.tags.len() > MAX_EVENT_TAGS
        || event
            .source_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 256 || id.chars().any(char::is_control))
        || event.source_uuid.as_ref().is_some_and(|uuid| {
            uuid.is_empty() || uuid.len() > 128 || uuid.chars().any(char::is_control)
        })
        || event
            .parent_source_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 256 || id.chars().any(char::is_control))
        || event.parent_source_uuid.as_ref().is_some_and(|uuid| {
            uuid.is_empty() || uuid.len() > 128 || uuid.chars().any(char::is_control)
        })
        || event
            .tags
            .iter()
            .any(|tag| tag.len() > MAX_TAG_BYTES || tag.chars().any(char::is_control))
        || event
            .module
            .as_ref()
            .is_some_and(|module| module.is_empty() || module.len() > MAX_MODULE_NAME_BYTES)
        || event.host.as_ref().is_some_and(|host| host.len() > 253)
        || event.locator.len() > 256
        || event.locator.chars().any(char::is_control)
        || event.port == Some(0)
        || event.timestamp.is_empty()
        || event.timestamp.len() > 64
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Evento BBOT excede límites o tiene campos inválidos",
        ));
    }
    Ok(())
}
