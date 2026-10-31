//! Normaliza respuestas archivadas de geolocalización IP como contexto histórico.
//!
//! No consulta proveedores ni afirma la ubicación actual de una dirección.

use std::net::IpAddr;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_OBJECT_FIELDS: usize = 64;
const MAX_TEXT_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq)]
pub struct ArchivedIpGeolocationHint {
    pub query_ip: String,
    pub country_name: Option<String>,
    pub region_name: Option<String>,
    pub city_name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Normaliza una respuesta JSON archivada asociada a una IP concreta.
///
/// Una respuesta con error de proveedor o sin datos geográficos produce `None`.
/// Si la respuesta incluye otra IP, se rechaza para conservar el linaje.
pub fn parse_archived_ip_geolocation(
    query_ip: &str,
    response: &[u8],
) -> ChefResult<Option<ArchivedIpGeolocationHint>> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta geográfica archivada supera el límite de bytes",
        ));
    }
    let query_ip = query_ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La IP consultada no es válida"))?
        .to_string();
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta geográfica archivada no contiene JSON válido",
        )
    })?;
    let Some(object) = document.as_object() else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta geográfica debe ser un objeto JSON",
        ));
    };
    if object.len() > MAX_OBJECT_FIELDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta geográfica supera el máximo de campos",
        ));
    }
    if object.contains_key("error") {
        return Ok(None);
    }
    if let Some(response_ip) = object.get("ip").and_then(Value::as_str) {
        let response_ip = response_ip.parse::<IpAddr>().map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta incluye una IP no válida",
            )
        })?;
        if response_ip.to_string() != query_ip {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La IP de la respuesta no coincide con la IP consultada",
            ));
        }
    }

    let country_name = bounded_text(object.get("country_name"))?;
    let region_name = bounded_text(object.get("region_name"))?;
    let city_name = bounded_text(object.get("city_name"))?;
    let latitude = bounded_coordinate(object.get("latitude"), -90.0, 90.0)?;
    let longitude = bounded_coordinate(object.get("longitude"), -180.0, 180.0)?;
    if country_name.is_none()
        && region_name.is_none()
        && city_name.is_none()
        && latitude.is_none()
        && longitude.is_none()
    {
        return Ok(None);
    }
    Ok(Some(ArchivedIpGeolocationHint {
        query_ip,
        country_name,
        region_name,
        city_name,
        latitude,
        longitude,
    }))
}

fn bounded_text(value: Option<&Value>) -> ChefResult<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(value) = value.as_str() else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Un campo de texto geográfico supera el límite CHEF",
        ));
    }
    Ok(Some(value.to_owned()))
}

fn bounded_coordinate(
    value: Option<&Value>,
    minimum: f64,
    maximum: f64,
) -> ChefResult<Option<f64>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let coordinate = match value {
        Value::Number(number) => number.as_f64(),
        Value::String(string) => string.parse::<f64>().ok(),
        _ => None,
    };
    let Some(coordinate) = coordinate else {
        return Ok(None);
    };
    if !coordinate.is_finite() || !(minimum..=maximum).contains(&coordinate) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Una coordenada geográfica está fuera de rango",
        ));
    }
    Ok(Some(coordinate))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_ip_geolocation;
    use crate::ErrorCode;

    #[test]
    fn parses_archived_location_for_matching_ip() {
        let response = br#"{"ip":"192.0.2.10","country_name":"Example","region_name":"North","city_name":"Sample","latitude":12.5,"longitude":"-45.25"}"#;
        let result = parse_archived_ip_geolocation("192.0.2.10", response)
            .unwrap()
            .unwrap();
        assert_eq!(result.country_name.as_deref(), Some("Example"));
        assert_eq!(result.latitude, Some(12.5));
        assert_eq!(result.longitude, Some(-45.25));
    }

    #[test]
    fn ignores_provider_errors_and_rejects_mismatched_ip_or_coordinates() {
        assert!(
            parse_archived_ip_geolocation(
                "192.0.2.10",
                br#"{"error":{"error_message":"no result"}}"#
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            parse_archived_ip_geolocation(
                "192.0.2.10",
                br#"{"ip":"192.0.2.11","country_name":"Example"}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_ip_geolocation(
                "192.0.2.10",
                br#"{"latitude":91,"country_name":"Example"}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_size_and_ip_validation() {
        let oversized = vec![b'x'; 64 * 1024 + 1];
        assert_eq!(
            parse_archived_ip_geolocation("192.0.2.10", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_ip_geolocation("example.com", b"{}")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
