//! Configuración CHEF derivada de las opciones BBOT que aplican al escritorio.
//! No acepta secretos y nunca habilita red activa desde un archivo TOML.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BbotEvent, ChefError, ChefResult, ErrorCode, EventScopeGate};

const MAX_CONFIG_BYTES: usize = 64 * 1024;

fn schema_version() -> u16 {
    1
}

fn max_workers() -> usize {
    4
}

fn max_events() -> usize {
    10_000
}

fn max_depth() -> usize {
    32
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct ChefConfig {
    #[serde(default = "schema_version")]
    pub schema_version: u16,
    pub runtime: RuntimeSettings,
    pub scope: ScopeSettings,
    pub modules: BTreeMap<String, bool>,
    pub sources: BTreeMap<String, SourceSettings>,
    pub defectdojo: DefectDojoSettings,
}

impl Default for ChefConfig {
    fn default() -> Self {
        Self {
            schema_version: schema_version(),
            runtime: RuntimeSettings::default(),
            scope: ScopeSettings::default(),
            modules: BTreeMap::new(),
            sources: BTreeMap::new(),
            defectdojo: DefectDojoSettings::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct RuntimeSettings {
    pub active_discovery: bool,
    #[serde(default = "max_workers")]
    pub max_workers: usize,
    #[serde(default = "max_events")]
    pub max_events: usize,
    #[serde(default = "max_depth")]
    pub max_depth: usize,
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self {
            active_discovery: false,
            max_workers: max_workers(),
            max_events: max_events(),
            max_depth: max_depth(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct ScopeSettings {
    pub allowed_hosts: Vec<String>,
    pub excluded_hosts: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<String>,
    pub max_scope_distance: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct SourceSettings {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "snake_case")]
pub struct DefectDojoSettings {
    pub enabled: bool,
    pub base_url: Option<String>,
    pub engagement_id: Option<u64>,
}

impl ChefConfig {
    /// Parse CHEF's bounded, secret-free TOML configuration.
    pub fn parse_toml(bytes: &[u8]) -> ChefResult<Self> {
        if bytes.len() > MAX_CONFIG_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El archivo de configuración CHEF supera el máximo de bytes",
            ));
        }
        crate::config_merge::parse_layered_toml(&[bytes])
    }

    pub fn validate(&self) -> ChefResult<()> {
        if self.schema_version != schema_version()
            || self.runtime.max_workers == 0
            || self.runtime.max_workers > 64
            || self.runtime.max_events == 0
            || self.runtime.max_events > max_events()
            || self.runtime.max_depth == 0
            || self.runtime.max_depth > max_depth()
            || self.modules.len() > 256
            || self.sources.len() > 128
            || self.scope.allowed_hosts.len() > 1000
            || self.scope.excluded_hosts.len() > 1000
            || self.scope.allowed_ports.len() > 256
            || self.scope.allowed_protocols.len() > 2
            || self.runtime.active_discovery
        {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Configuración CHEF fuera de límites o con una capacidad bloqueada",
            ));
        }
        for name in self.modules.keys() {
            if !valid_name(name) {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Nombre de módulo CHEF inválido",
                ));
            }
        }
        for name in self.sources.keys() {
            if !valid_name(name) {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Nombre de fuente CHEF inválido",
                ));
            }
        }
        if self
            .scope
            .allowed_hosts
            .iter()
            .chain(self.scope.excluded_hosts.iter())
            .any(|host| !valid_host(host))
        {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Scope CHEF debe contener IP o dominios literales válidos",
            ));
        }
        if self.scope.allowed_ports.contains(&0)
            || self
                .scope
                .allowed_ports
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != self.scope.allowed_ports.len()
            || self
                .scope
                .allowed_protocols
                .iter()
                .any(|protocol| protocol != "tcp" && protocol != "udp")
            || self.scope.max_scope_distance > 32
        {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Límites de puertos, protocolos o distancia de scope inválidos",
            ));
        }
        if self.defectdojo.enabled {
            let Some(url) = self.defectdojo.base_url.as_deref() else {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "DefectDojo requiere URL HTTPS configurada",
                ));
            };
            if !url.starts_with("https://")
                || url.contains('@')
                || url.contains('?')
                || url.contains('#')
                || url.strip_prefix("https://").is_none_or(|host| {
                    host.is_empty() || host.starts_with('/') || host.contains(char::is_whitespace)
                })
                || self.defectdojo.engagement_id.is_none_or(|id| id == 0)
            {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "La URL de DefectDojo debe ser HTTPS y no contener credenciales",
                ));
            }
        }
        Ok(())
    }
}

impl EventScopeGate for ScopeSettings {
    fn authorize(&self, event: &BbotEvent) -> ChefResult<()> {
        let Some(host) = event.host.as_deref() else {
            return Err(ChefError::new(
                ErrorCode::OutOfScope,
                "El evento BBOT no identifica un host dentro del scope",
            ));
        };
        if event.scope_distance > self.max_scope_distance
            || !self
                .allowed_hosts
                .iter()
                .any(|allowed| host_matches(host, allowed))
            || self
                .excluded_hosts
                .iter()
                .any(|excluded| host_matches(host, excluded))
        {
            return Err(ChefError::new(
                ErrorCode::OutOfScope,
                "El host BBOT está fuera del scope CHEF explícito",
            ));
        }
        if let Some(port) = event.port {
            if !self.allowed_ports.contains(&port) {
                return Err(ChefError::new(
                    ErrorCode::OutOfScope,
                    "El puerto BBOT no está permitido por el scope CHEF",
                ));
            }
            let required_protocol = match event.event_type.as_str() {
                "OPEN_TCP_PORT" => Some("tcp"),
                "OPEN_UDP_PORT" => Some("udp"),
                _ => None,
            };
            if required_protocol.is_some_and(|protocol| {
                !self
                    .allowed_protocols
                    .iter()
                    .any(|allowed| allowed == protocol)
            }) {
                return Err(ChefError::new(
                    ErrorCode::OutOfScope,
                    "El protocolo BBOT no está permitido por el scope CHEF",
                ));
            }
        }
        Ok(())
    }
}

fn host_matches(host: &str, allowed: &str) -> bool {
    crate::host::hosts_match(host, allowed)
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
}

fn valid_host(value: &str) -> bool {
    crate::host::canonical_host(value).is_some()
}
