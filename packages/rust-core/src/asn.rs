//! Reglas puras para adaptar la respuesta de enriquecimiento ASN de BBOT.
//!
//! El adaptador de aplicación valida límites HTTP y aporta el JSON recibido.
//! Este módulo no consulta proveedores, espera, resuelve objetivos ni amplía scope.

use std::time::Duration;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_SUBNETS: usize = 4096;
const MAX_TEXT_BYTES: usize = 1024;

/// Datos normalizados de un proveedor ASN. Las subredes son pistas del proveedor,
/// no autorización de escaneo ni prueba de propiedad de una IP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsnRecord {
    pub asn: u32,
    pub subnets: Vec<String>,
    pub name: String,
    pub description: String,
    pub country: String,
}

impl AsnRecord {
    /// Forma compatible con `ASNHelper.UNKNOWN_ASN` de BBOT.
    pub fn unknown() -> Self {
        Self {
            asn: 0,
            subnets: Vec::new(),
            name: "Unknown".to_owned(),
            description: "Unknown ASN".to_owned(),
            country: "Unknown".to_owned(),
        }
    }
}

/// Normaliza respuestas de `asndb`, aceptando sus nombres de campo actuales
/// y las alternativas históricas utilizadas por BBOT.
pub fn normalize_asn_response(response: Option<&Value>) -> ChefResult<AsnRecord> {
    let Some(response) = response.filter(|value| !value.is_null()) else {
        return Ok(AsnRecord::unknown());
    };
    let Some(object) = response.as_object() else {
        return Err(invalid_response());
    };

    let asn = match object.get("asn") {
        None | Some(Value::Null) => 0,
        Some(Value::Number(value)) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(invalid_response)?,
        Some(Value::String(value)) => value.parse::<u32>().map_err(|_| invalid_response())?,
        _ => return Err(invalid_response()),
    };
    if asn == 0 {
        return Ok(AsnRecord::unknown());
    }

    let subnets = match object.get("subnets") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(values)) if values.len() <= MAX_SUBNETS => values
            .iter()
            .map(|value| {
                let value = value.as_str().ok_or_else(invalid_response)?;
                crate::target_seed::canonical_network(value).ok_or_else(invalid_response)
            })
            .collect::<ChefResult<Vec<_>>>()?,
        _ => return Err(invalid_response()),
    };

    let name = provider_text(object.get("asn_name"))
        .or_else(|| provider_text(object.get("name")))
        .unwrap_or_default();
    let description = provider_text(object.get("org"))
        .or_else(|| provider_text(object.get("description")))
        .unwrap_or_default();
    let country = provider_text(object.get("country")).unwrap_or_default();

    Ok(AsnRecord {
        asn,
        subnets,
        name,
        description,
        country,
    })
}

fn provider_text(value: Option<&Value>) -> Option<String> {
    let value = value?.as_str()?.trim();
    (!value.is_empty() && value.len() <= MAX_TEXT_BYTES && !value.chars().any(char::is_control))
        .then(|| value.to_owned())
}

/// Estado del cortacircuito de enriquecimiento tolerante de BBOT.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsnEnrichmentCircuitBreaker {
    consecutive_failures: u8,
    disabled: bool,
}

impl AsnEnrichmentCircuitBreaker {
    pub const FAILURE_THRESHOLD: u8 = 5;

    pub fn allows_lookup(self) -> bool {
        !self.disabled
    }

    /// Registra fallo y devuelve `true` solo al abrirse el circuito.
    pub fn record_failure(&mut self) -> bool {
        if self.disabled {
            return false;
        }
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= Self::FAILURE_THRESHOLD {
            self.disabled = true;
            return true;
        }
        false
    }

    /// Un enriquecimiento correcto reinicia los fallos consecutivos.
    pub fn record_success(&mut self) {
        if !self.disabled {
            self.consecutive_failures = 0;
        }
    }
}

/// Política BBOT para una semilla ASN: máximo tres intentos, con tres segundos
/// entre intentos fallidos. No inicia esos intentos ni activa la semilla.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AsnTargetRetryPolicy;

impl AsnTargetRetryPolicy {
    pub const MAX_ATTEMPTS: u8 = 3;
    pub const RETRY_DELAY: Duration = Duration::from_secs(3);

    /// Devuelve la espera después del intento fallido indicado (base 1).
    pub fn delay_after_failure(self, failed_attempt: u8) -> Option<Duration> {
        (failed_attempt > 0 && failed_attempt < Self::MAX_ATTEMPTS).then_some(Self::RETRY_DELAY)
    }
}

fn invalid_response() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta del proveedor ASN no cumple el formato CHEF",
    )
}
