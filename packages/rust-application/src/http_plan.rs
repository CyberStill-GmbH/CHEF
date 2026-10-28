//! Plan offline de solicitudes web a partir de observaciones CHEF existentes.
//!
//! No abre sockets ni resuelve nombres. El plan conserva los eventos fuente,
//! reduce URL a su origen y debe pasar por autorización vigente antes de que
//! una futura composición de infraestructura ejecute cualquier solicitud.

use std::collections::{BTreeMap, BTreeSet};

use chef_core::{BbotEvent, ChefTargetKind, parse_chef_target_seed};
use url::Url;

const MAX_HTTP_PLAN_EVENTS: usize = 10_000;
const MAX_HTTP_REQUESTS: usize = 512;
const MAX_HTTP_REDIRECTS: u8 = 3;
const MAX_HTTP_RESPONSE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpPlanError {
    InvalidSettings,
    InvalidTarget,
    LimitExceeded,
}

impl std::fmt::Display for HttpPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidSettings => "La configuración del plan HTTP CHEF no es válida",
            Self::InvalidTarget => "El objetivo del plan HTTP CHEF no es válido",
            Self::LimitExceeded => "El plan HTTP CHEF supera los límites",
        })
    }
}

impl std::error::Error for HttpPlanError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpPlanSettings {
    max_requests: usize,
    max_redirects: u8,
    max_response_bytes: usize,
}

impl HttpPlanSettings {
    pub fn new(
        max_requests: usize,
        max_redirects: u8,
        max_response_bytes: usize,
    ) -> Result<Self, HttpPlanError> {
        if max_requests == 0
            || max_requests > MAX_HTTP_REQUESTS
            || max_redirects > MAX_HTTP_REDIRECTS
            || max_response_bytes == 0
            || max_response_bytes > MAX_HTTP_RESPONSE_BYTES
        {
            return Err(HttpPlanError::InvalidSettings);
        }
        Ok(Self {
            max_requests,
            max_redirects,
            max_response_bytes,
        })
    }

    pub fn defaults() -> Self {
        Self {
            max_requests: MAX_HTTP_REQUESTS,
            max_redirects: MAX_HTTP_REDIRECTS,
            max_response_bytes: MAX_HTTP_RESPONSE_BYTES,
        }
    }

    pub fn max_requests(self) -> usize {
        self.max_requests
    }

    pub fn max_redirects(self) -> u8 {
        self.max_redirects
    }

    pub fn max_response_bytes(self) -> usize {
        self.max_response_bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedHttpRequest {
    origin: String,
    source_event_ids: Vec<String>,
}

impl PlannedHttpRequest {
    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn method(&self) -> &'static str {
        "GET"
    }

    pub fn source_event_ids(&self) -> &[String] {
        &self.source_event_ids
    }
}

/// Candidatos HTTP deduplicados y acotados; construir este valor no ejecuta red.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestPlan {
    settings: HttpPlanSettings,
    requests: Vec<PlannedHttpRequest>,
}

impl HttpRequestPlan {
    /// Adapta `modules/http.py` para URLs observadas/no verificadas y puertos
    /// TCP abiertos ya importados. Los puertos 80/443 conservan su esquema;
    /// otros puertos producen candidatos HTTP y HTTPS. Se descartan path,
    /// query y fragmento para no trasladar datos de aplicación al plan.
    pub fn from_events(
        events: &[BbotEvent],
        settings: HttpPlanSettings,
    ) -> Result<Self, HttpPlanError> {
        if events.len() > MAX_HTTP_PLAN_EVENTS {
            return Err(HttpPlanError::LimitExceeded);
        }

        let mut requests = BTreeMap::<String, BTreeSet<String>>::new();
        for event in events {
            if event.data_redacted {
                continue;
            }
            let origins = match event.event_type.as_str() {
                "URL" | "URL_UNVERIFIED" => url_event_origin(event),
                "OPEN_TCP_PORT" => tcp_port_origins(event),
                _ => Vec::new(),
            };
            for origin in origins {
                if event.id.is_empty() {
                    return Err(HttpPlanError::InvalidTarget);
                }
                requests.entry(origin).or_default().insert(event.id.clone());
                if requests.len() > settings.max_requests {
                    return Err(HttpPlanError::LimitExceeded);
                }
            }
        }

        Ok(Self {
            settings,
            requests: requests
                .into_iter()
                .map(|(origin, source_event_ids)| PlannedHttpRequest {
                    origin,
                    source_event_ids: source_event_ids.into_iter().collect(),
                })
                .collect(),
        })
    }

    pub fn settings(&self) -> HttpPlanSettings {
        self.settings
    }

    pub fn requests(&self) -> &[PlannedHttpRequest] {
        &self.requests
    }
}

fn url_event_origin(event: &BbotEvent) -> Vec<String> {
    let Some(value) = event.data.as_str() else {
        return Vec::new();
    };
    if value.len() > 8 * 1024 || value.contains('\\') || value.chars().any(char::is_control) {
        return Vec::new();
    }
    let Ok(mut url) = Url::parse(value) else {
        return Vec::new();
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host().is_none()
        || url.port_or_known_default() == Some(0)
    {
        return Vec::new();
    }
    url.set_path("/");
    url.set_query(None);
    url.set_fragment(None);
    vec![url.to_string()]
}

fn tcp_port_origins(event: &BbotEvent) -> Vec<String> {
    let (Some(host), Some(port)) = (event.host.as_deref(), event.port) else {
        return Vec::new();
    };
    if port == 0 {
        return Vec::new();
    }
    let Ok(seed) = parse_chef_target_seed(host) else {
        return Vec::new();
    };
    if !matches!(
        seed.kind,
        ChefTargetKind::DnsName | ChefTargetKind::IpAddress
    ) {
        return Vec::new();
    }
    let Some(host) = seed.host else {
        return Vec::new();
    };
    let display_host = match host.parse::<std::net::Ipv6Addr>() {
        Ok(address) => format!("[{address}]"),
        Err(_) => host,
    };
    let schemes: &[&str] = match port {
        80 => &["http"],
        443 => &["https"],
        _ => &["http", "https"],
    };
    schemes
        .iter()
        .filter_map(|scheme| {
            let mut url = Url::parse(&format!("{scheme}://{display_host}/")).ok()?;
            url.set_port(Some(port)).ok()?;
            Some(url.to_string())
        })
        .collect()
}
