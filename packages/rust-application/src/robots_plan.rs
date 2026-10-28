//! Plan offline de destinos `robots.txt` desde URL ya observadas.
//!
//! Adapta la selección inicial de `modules/robots.py`. No descarga ni analiza
//! el archivo; una ejecución futura debe pasar por el caso HTTP autorizado.

use std::collections::{BTreeMap, BTreeSet};

use chef_core::BbotEvent;
use url::Url;

const MAX_ROBOTS_EVENTS: usize = 10_000;
const MAX_ROBOTS_REQUESTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RobotsPlanError {
    LimitExceeded,
    InvalidSource,
}

impl std::fmt::Display for RobotsPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::LimitExceeded => "El plan robots.txt supera los límites CHEF",
            Self::InvalidSource => "El evento fuente robots.txt no tiene identidad válida",
        })
    }
}

impl std::error::Error for RobotsPlanError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRobotsTxtRequest {
    url: String,
    source_event_ids: Vec<String>,
}

impl PlannedRobotsTxtRequest {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn method(&self) -> &'static str {
        "GET"
    }

    pub fn source_event_ids(&self) -> &[String] {
        &self.source_event_ids
    }
}

/// Candidatos GET deterministas y acotados; crear el plan no realiza HTTP.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RobotsTxtRequestPlan {
    requests: Vec<PlannedRobotsTxtRequest>,
}

impl RobotsTxtRequestPlan {
    /// Usa solo eventos `URL` no redactados y con distancia de scope cero.
    /// Deduplica por origen conservando esquema y puerto no predeterminado.
    pub fn from_events(events: &[BbotEvent]) -> Result<Self, RobotsPlanError> {
        if events.len() > MAX_ROBOTS_EVENTS {
            return Err(RobotsPlanError::LimitExceeded);
        }

        let mut requests = BTreeMap::<String, BTreeSet<String>>::new();
        for event in events {
            if event.data_redacted || event.event_type != "URL" || event.scope_distance != 0 {
                continue;
            }
            let Some(value) = event.data.as_str() else {
                continue;
            };
            let Some(url) = robots_url(value) else {
                continue;
            };
            if event.id.is_empty() {
                return Err(RobotsPlanError::InvalidSource);
            }

            requests.entry(url).or_default().insert(event.id.clone());
            if requests.len() > MAX_ROBOTS_REQUESTS {
                return Err(RobotsPlanError::LimitExceeded);
            }
        }

        Ok(Self {
            requests: requests
                .into_iter()
                .map(|(url, source_event_ids)| PlannedRobotsTxtRequest {
                    url,
                    source_event_ids: source_event_ids.into_iter().collect(),
                })
                .collect(),
        })
    }

    pub fn requests(&self) -> &[PlannedRobotsTxtRequest] {
        &self.requests
    }
}

fn robots_url(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > 8 * 1024 || value.contains('\\') {
        return None;
    }
    if value.chars().any(char::is_control) {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host().is_none()
        || url.port_or_known_default() == Some(0)
    {
        return None;
    }
    url.set_path("/robots.txt");
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}
