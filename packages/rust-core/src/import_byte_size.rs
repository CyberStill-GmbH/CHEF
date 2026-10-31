//! Conversión acotada de límites de tamaño de importación para CHEF.
//!
//! Adapta `core/helpers/misc.py::human_to_bytes` y `bytes_to_human` a los
//! límites de importación local de CHEF. Solo convierte valores; no lee archivos.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_SIZE_TEXT_BYTES: usize = 64;
const MAX_IMPORT_BYTES: u64 = 2 * 1024 * 1024;
const UNITS: [(&str, u64); 6] = [
    ("B", 1),
    ("KB", 1024),
    ("MB", 1024 * 1024),
    ("GB", 1024 * 1024 * 1024),
    ("TB", 1024 * 1024 * 1024 * 1024),
    ("PB", 1024 * 1024 * 1024 * 1024 * 1024),
];

/// Convierte un límite como '1.5MB', '800K' o '120' a bytes.
///
/// Acepta sufijos decimales binarios de la fuente (B, K/KB, M/MB, etc.),
/// espacios y fracciones no negativas. El resultado no puede superar 2 MiB.
pub fn parse_import_byte_limit(value: &str) -> ChefResult<u64> {
    if value.len() > MAX_SIZE_TEXT_BYTES {
        return Err(limit_error());
    }
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid_size());
    }
    let split = trimmed
        .find(|character: char| character.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    let (number, suffix) = trimmed.split_at(split);
    let number = number.trim();
    let suffix = suffix.trim().to_ascii_uppercase();
    let unit = unit_multiplier(&suffix).ok_or_else(invalid_size)?;
    if number.is_empty()
        || number.starts_with('-')
        || number.matches('.').count() > 1
        || !number
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return Err(invalid_size());
    }
    let magnitude = number.parse::<f64>().map_err(|_| invalid_size())?;
    let bytes = magnitude * unit as f64;
    if !bytes.is_finite() || bytes < 0.0 || bytes > MAX_IMPORT_BYTES as f64 {
        return Err(limit_error());
    }
    Ok(bytes.trunc() as u64)
}

/// Formatea un tamaño dentro del máximo de importación como B, KB o MB.
pub fn format_import_byte_limit(bytes: u64) -> ChefResult<String> {
    if bytes > MAX_IMPORT_BYTES {
        return Err(limit_error());
    }
    let (unit, multiplier) = if bytes >= 1024 * 1024 {
        UNITS[2]
    } else if bytes >= 1024 {
        UNITS[1]
    } else {
        UNITS[0]
    };
    if multiplier == 1 {
        return Ok(format!("{bytes}{unit}"));
    }
    Ok(format!("{:.2}{unit}", bytes as f64 / multiplier as f64))
}

fn unit_multiplier(suffix: &str) -> Option<u64> {
    match suffix {
        "" | "B" => Some(1),
        "K" | "KB" => Some(UNITS[1].1),
        "M" | "MB" => Some(UNITS[2].1),
        "G" | "GB" => Some(UNITS[3].1),
        "T" | "TB" => Some(UNITS[4].1),
        "P" | "PB" => Some(UNITS[5].1),
        _ => None,
    }
}

fn invalid_size() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "El tamaño debe ser un número no negativo con sufijo B, K, M, G, T o P",
    )
}

fn limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "El tamaño supera el máximo CHEF de importación de 2 MiB",
    )
}

#[cfg(test)]
mod tests {
    use super::{MAX_SIZE_TEXT_BYTES, format_import_byte_limit, parse_import_byte_limit};
    use crate::ErrorCode;

    #[test]
    fn parses_source_units_and_fractions_for_import_limits() {
        assert_eq!(parse_import_byte_limit("120").unwrap(), 120);
        assert_eq!(parse_import_byte_limit("1.5 MB").unwrap(), 1_572_864);
        assert_eq!(parse_import_byte_limit("800K").unwrap(), 819_200);
        assert_eq!(parse_import_byte_limit("2MB").unwrap(), 2 * 1024 * 1024);
    }

    #[test]
    fn formats_import_limit_for_display() {
        assert_eq!(format_import_byte_limit(120).unwrap(), "120B");
        assert_eq!(format_import_byte_limit(1536).unwrap(), "1.50KB");
        assert_eq!(format_import_byte_limit(2 * 1024 * 1024).unwrap(), "2.00MB");
    }

    #[test]
    fn rejects_malformed_or_oversized_values() {
        assert_eq!(
            parse_import_byte_limit("2.5XB").unwrap_err().code,
            ErrorCode::InvalidInput
        );
        assert!(parse_import_byte_limit("-1MB").is_err());
        assert_eq!(
            parse_import_byte_limit("3MB").unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_import_byte_limit(&"1".repeat(MAX_SIZE_TEXT_BYTES + 1))
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert!(format_import_byte_limit(2 * 1024 * 1024 + 1).is_err());
    }
}
