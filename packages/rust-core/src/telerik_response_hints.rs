//! Busca marcadores Telerik en cuerpos HTTP ya archivados.
//!
//! Un marcador solo describe contenido presente en la captura. No valida un
//! componente desplegado, no confirma vulnerabilidad ni realiza solicitudes.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_BODY_BYTES: usize = 1024 * 1024;
const MAX_MARKER_OCCURRENCES: usize = 128;

const SERIALIZED_PARAMETERS_MARKER: &[u8] = b":{\"SerializedParameters\":\"";
const SERIALIZED_CONFIGURATION_MARKER: &[u8] = b"\"_serializedConfiguration\":\"";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TelerikResponseMarker {
    SerializedParameters,
    SerializedConfiguration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TelerikMarkerCount {
    pub marker: TelerikResponseMarker,
    pub occurrences: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedTelerikResponseHints {
    pub markers: Vec<TelerikMarkerCount>,
}

/// Cuenta hasta 128 apariciones de cada marcador en un cuerpo archivado.
pub fn parse_archived_telerik_response(body: &[u8]) -> ChefResult<ArchivedTelerikResponseHints> {
    if body.len() > MAX_BODY_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El cuerpo HTTP archivado supera el límite del parser Telerik",
        ));
    }
    let mut markers = Vec::new();
    for (marker, needle) in [
        (
            TelerikResponseMarker::SerializedParameters,
            SERIALIZED_PARAMETERS_MARKER,
        ),
        (
            TelerikResponseMarker::SerializedConfiguration,
            SERIALIZED_CONFIGURATION_MARKER,
        ),
    ] {
        let occurrences = count_occurrences(body, needle);
        if occurrences > MAX_MARKER_OCCURRENCES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El cuerpo HTTP supera el máximo de marcadores Telerik",
            ));
        }
        if occurrences > 0 {
            markers.push(TelerikMarkerCount {
                marker,
                occurrences: occurrences as u16,
            });
        }
    }
    Ok(ArchivedTelerikResponseHints { markers })
}

fn count_occurrences(body: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() || needle.len() > body.len() {
        return 0;
    }
    body.windows(needle.len())
        .filter(|window| *window == needle)
        .count()
}

#[cfg(test)]
mod tests {
    use super::{TelerikResponseMarker, parse_archived_telerik_response};
    use crate::ErrorCode;

    #[test]
    fn identifies_known_markers_without_generating_a_finding() {
        let body =
            br#"{"payload":{"SerializedParameters":"opaque"},"_serializedConfiguration":"opaque"}"#;
        let result = parse_archived_telerik_response(body).unwrap();
        assert_eq!(result.markers.len(), 2);
        assert!(result.markers.iter().any(|hint| {
            hint.marker == TelerikResponseMarker::SerializedParameters && hint.occurrences == 1
        }));
        assert!(result.markers.iter().any(|hint| {
            hint.marker == TelerikResponseMarker::SerializedConfiguration && hint.occurrences == 1
        }));
    }

    #[test]
    fn ignores_similar_unrelated_content() {
        let body = br#"{"SerializedParameters":"opaque","serializedConfiguration":"opaque","text":"Telerik"}"#;
        assert!(
            parse_archived_telerik_response(body)
                .unwrap()
                .markers
                .is_empty()
        );
    }

    #[test]
    fn enforces_body_and_marker_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_telerik_response(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let repeated = b":{\"SerializedParameters\":\"".repeat(129);
        assert_eq!(
            parse_archived_telerik_response(&repeated).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
