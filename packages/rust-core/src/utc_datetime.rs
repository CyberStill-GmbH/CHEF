//! Normalización offline de fechas de importación al formato temporal CHEF.
//!
//! Adapta `models/helpers.py::utc_datetime_validator`: una fecha con offset se
//! convierte a UTC y una fecha sin zona se interpreta como UTC. No consulta el
//! reloj ni deriva el momento de una observación.

use chrono::{DateTime, Datelike, NaiveDateTime, SecondsFormat, Utc};

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_TIMESTAMP_BYTES: usize = 128;

/// Normaliza una fecha ISO 8601/RFC 3339 con o sin offset a UTC con milisegundos.
///
/// CHEF limita el texto a 128 bytes. Una fecha local sin zona se trata como UTC,
/// siguiendo el helper Python; no se aplica la zona horaria de la máquina.
pub fn normalize_utc_datetime(value: &str) -> ChefResult<String> {
    if value.len() > MAX_TIMESTAMP_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El texto de fecha supera el máximo CHEF de 128 bytes",
        ));
    }

    let date = if let Ok(with_offset) = DateTime::parse_from_rfc3339(value) {
        with_offset.with_timezone(&Utc)
    } else {
        NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f")
            .map(|naive| naive.and_utc())
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Fecha ISO 8601 CHEF inválida"))?
    };

    if !(1..=9999).contains(&date.year()) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Fecha fuera del rango de años admitido por el modelo CHEF",
        ));
    }

    Ok(date.to_rfc3339_opts(SecondsFormat::Millis, true))
}

#[cfg(test)]
mod tests {
    use super::{MAX_TIMESTAMP_BYTES, normalize_utc_datetime};
    use crate::ErrorCode;

    #[test]
    fn normalizes_offset_and_assumes_utc_for_naive_dates() {
        assert_eq!(
            normalize_utc_datetime("2026-10-31T12:30:00.123+05:30").unwrap(),
            "2026-10-31T07:00:00.123Z"
        );
        assert_eq!(
            normalize_utc_datetime("2026-10-31T07:00:00").unwrap(),
            "2026-10-31T07:00:00.000Z"
        );
        assert_eq!(
            normalize_utc_datetime("2026-10-31T07:00:00.123456").unwrap(),
            "2026-10-31T07:00:00.123Z"
        );
    }

    #[test]
    fn rejects_invalid_dates_and_timezone_abbreviations() {
        assert_eq!(
            normalize_utc_datetime("2026-02-30T07:00:00")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert!(normalize_utc_datetime("2026-10-31 07:00:00").is_err());
        assert!(normalize_utc_datetime("2026-10-31T07:00:00 PST").is_err());
        assert!(normalize_utc_datetime("0000-10-31T07:00:00").is_err());
    }

    #[test]
    fn bounds_the_untrusted_timestamp_text() {
        let text = format!("{}2026-10-31T07:00:00", " ".repeat(MAX_TIMESTAMP_BYTES));
        assert_eq!(
            normalize_utc_datetime(&text).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
