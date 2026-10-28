//! Plan offline de destinos security.txt desde nombres DNS ya observados.
//!
//! Adapta la selección inicial de `modules/securitytxt.py`. Construir el plan
//! no solicita el recurso, no consulta DNS y no convierte un candidato en
//! autorización; una ejecución futura debe pasar por el flujo HTTP autorizado.

use std::collections::{BTreeMap, BTreeSet};

use chef_core::{BbotEvent, ChefTargetKind, canonical_fqdn_or_ip, parse_chef_target_seed};

const MAX_SECURITYTXT_EVENTS: usize = 10_000;
const MAX_SECURITYTXT_REQUESTS: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityTxtPlanError {
    LimitExceeded,
    InvalidSource,
}

impl std::fmt::Display for SecurityTxtPlanError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::LimitExceeded => "El plan security.txt supera los límites CHEF",
            Self::InvalidSource => "El evento fuente security.txt no tiene identidad válida",
        })
    }
}

impl std::error::Error for SecurityTxtPlanError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedSecurityTxtRequest {
    url: String,
    source_event_ids: Vec<String>,
}

impl PlannedSecurityTxtRequest {
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

/// Candidatos HTTPS deterministas y acotados; no contienen cuerpo ni permisos.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SecurityTxtRequestPlan {
    requests: Vec<PlannedSecurityTxtRequest>,
}

impl SecurityTxtRequestPlan {
    /// Selecciona solo eventos `DNS_NAME` con distancia de scope cero.
    /// Cada nombre produce la ruta estándar `/.well-known/security.txt`.
    pub fn from_events(events: &[BbotEvent]) -> Result<Self, SecurityTxtPlanError> {
        if events.len() > MAX_SECURITYTXT_EVENTS {
            return Err(SecurityTxtPlanError::LimitExceeded);
        }

        let mut requests = BTreeMap::<String, BTreeSet<String>>::new();
        for event in events {
            if event.data_redacted || event.event_type != "DNS_NAME" || event.scope_distance != 0 {
                continue;
            }
            let Some(value) = event.data.as_str() else {
                continue;
            };
            let Ok(seed) = parse_chef_target_seed(value) else {
                continue;
            };
            if seed.kind != ChefTargetKind::DnsName {
                continue;
            }
            let Some(host) = canonical_fqdn_or_ip(&seed.data) else {
                continue;
            };
            if host.parse::<std::net::IpAddr>().is_ok() {
                continue;
            }
            if event.id.is_empty() {
                return Err(SecurityTxtPlanError::InvalidSource);
            }

            let url = format!("https://{host}/.well-known/security.txt");
            requests.entry(url).or_default().insert(event.id.clone());
            if requests.len() > MAX_SECURITYTXT_REQUESTS {
                return Err(SecurityTxtPlanError::LimitExceeded);
            }
        }

        Ok(Self {
            requests: requests
                .into_iter()
                .map(|(url, source_event_ids)| PlannedSecurityTxtRequest {
                    url,
                    source_event_ids: source_event_ids.into_iter().collect(),
                })
                .collect(),
        })
    }

    pub fn requests(&self) -> &[PlannedSecurityTxtRequest] {
        &self.requests
    }
}
