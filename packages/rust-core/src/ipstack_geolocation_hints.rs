//! Adapta respuestas IPStack archivadas al contrato CHEF de geolocalización.
//!
//! La respuesta se trata como contexto histórico. Este parser no llama al
//! proveedor, no acepta ni conserva API keys y descarta campos ajenos al
//! resumen CHEF compartido.

use std::net::IpAddr;

use serde_json::{Map, Value, json};

use crate::{ArchivedIpGeolocationHint, ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 64 * 1024;
const MAX_OBJECT_FIELDS: usize = 64;

/// Normaliza un objeto JSON IPStack archivado para una IP consultada.
///
/// Se requiere que `ip` esté presente y coincida con la IP solicitada. Los
/// errores del proveedor y las respuestas sin ningún dato geográfico producen
/// `None`; las respuestas incompatibles o fuera de límites producen error.
pub fn parse_archived_ipstack_geolocation(
    query_ip: &str,
    response: &[u8],
) -> ChefResult<Option<ArchivedIpGeolocationHint>> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta IPStack archivada supera el límite de bytes",
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
            "La respuesta IPStack archivada no contiene JSON válido",
        )
    })?;
    let Some(object) = document.as_object() else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta IPStack debe ser un objeto JSON",
        ));
    };
    if object.len() > MAX_OBJECT_FIELDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta IPStack supera el máximo de campos",
        ));
    }
    if object.contains_key("error") {
        return Ok(None);
    }

    let response_ip = object
        .get("ip")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta IPStack no contiene la IP consultada",
            )
        })?
        .parse::<IpAddr>()
        .map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La IP de respuesta IPStack no es válida",
            )
        })?
        .to_string();
    if response_ip != query_ip {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La IP de respuesta IPStack no coincide con la IP consultada",
        ));
    }

    let sanitized = normalized_fields(object, &response_ip);
    let sanitized_bytes = serde_json::to_vec(&sanitized).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "No se pudo normalizar la respuesta IPStack",
        )
    })?;
    crate::ip_geolocation_hints::parse_archived_ip_geolocation(&query_ip, &sanitized_bytes)
}

fn normalized_fields(object: &Map<String, Value>, response_ip: &str) -> Value {
    json!({
        "ip": response_ip,
        "country_name": object.get("country_name").cloned().unwrap_or(Value::Null),
        "region_name": object.get("region_name").cloned().unwrap_or(Value::Null),
        "city_name": object.get("city").cloned().unwrap_or(Value::Null),
        "latitude": object.get("latitude").cloned().unwrap_or(Value::Null),
        "longitude": object.get("longitude").cloned().unwrap_or(Value::Null),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_ipstack_geolocation;
    use crate::ErrorCode;

    #[test]
    fn maps_ipstack_city_to_chef_location_and_discards_provider_secrets() {
        let response = br#"{
            "ip":"192.0.2.10",
            "country_name":"Example",
            "region_name":"North",
            "city":"Sample",
            "latitude":"12.5",
            "longitude":-45.25,
            "access_key":"must-not-be-retained",
            "zip":"12345",
            "location":{"country_flag":"https://example.invalid/flag.svg"}
        }"#;

        let result = parse_archived_ipstack_geolocation("192.0.2.10", response)
            .unwrap()
            .unwrap();

        assert_eq!(result.query_ip, "192.0.2.10");
        assert_eq!(result.country_name.as_deref(), Some("Example"));
        assert_eq!(result.region_name.as_deref(), Some("North"));
        assert_eq!(result.city_name.as_deref(), Some("Sample"));
        assert_eq!(result.latitude, Some(12.5));
        assert_eq!(result.longitude, Some(-45.25));
    }

    #[test]
    fn ignores_provider_errors_and_rejects_missing_or_mismatched_ip() {
        assert!(
            parse_archived_ipstack_geolocation(
                "192.0.2.10",
                br#"{"error":{"code":104,"info":"invalid access key"}}"#
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            parse_archived_ipstack_geolocation("192.0.2.10", br#"{"country_name":"Example"}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_ipstack_geolocation(
                "192.0.2.10",
                br#"{"ip":"192.0.2.11","country_name":"Example"}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn validates_json_size_field_count_text_and_coordinates() {
        assert_eq!(
            parse_archived_ipstack_geolocation("example.com", b"{}")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_ipstack_geolocation("192.0.2.10", b"[]")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_ipstack_geolocation("192.0.2.10", b"not-json")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );

        let oversized = vec![b'x'; 64 * 1024 + 1];
        assert_eq!(
            parse_archived_ipstack_geolocation("192.0.2.10", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let many_fields = format!(
            "{{\"ip\":\"192.0.2.10\",{}}}",
            (0..64)
                .map(|index| format!("\"extra{index}\":null"))
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(
            parse_archived_ipstack_geolocation("192.0.2.10", many_fields.as_bytes())
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_ipstack_geolocation(
                "192.0.2.10",
                br#"{"ip":"192.0.2.10","city":"Bad\nCity"}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_ipstack_geolocation(
                "192.0.2.10",
                br#"{"ip":"192.0.2.10","latitude":91}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }
}
