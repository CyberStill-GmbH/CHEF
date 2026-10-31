//! Extrae nombres y versiones de bibliotecas desde salida archivada de Retire.js.
//!
//! El parser construye inventario técnico histórico. Ignora por completo las
//! listas de vulnerabilidades y no genera hallazgos ni niveles de riesgo.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_FILE_RESULTS: usize = 1024;
const MAX_COMPONENT_RECORDS: usize = 2048;
const MAX_COMPONENT_HINTS: usize = 1024;
const MAX_FIELD_CHARS: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct JavaScriptComponentHint {
    pub component: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedJavaScriptInventory {
    pub components: Vec<JavaScriptComponentHint>,
}

/// Normaliza exclusivamente `component` y `version` de la salida archivada.
///
/// Las propiedades `vulnerabilities` y cualquier severidad se descartan sin
/// inspeccionarse ni reinterpretarse.
pub fn parse_archived_javascript_inventory(
    response: &[u8],
) -> ChefResult<ArchivedJavaScriptInventory> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La salida JavaScript archivada supera el límite de bytes",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La salida JavaScript archivada no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La salida JavaScript debe ser un objeto JSON",
    ))?;
    let data = object
        .get("data")
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La salida JavaScript debe incluir una lista data",
        ))?;
    if data.len() > MAX_FILE_RESULTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La salida JavaScript supera el máximo de archivos",
        ));
    }

    let mut components = BTreeSet::new();
    let mut records_seen = 0;
    for file_result in data {
        let Some(results) = file_result
            .as_object()
            .and_then(|object| object.get("results"))
        else {
            continue;
        };
        let Some(results) = results.as_array() else {
            continue;
        };
        for result in results {
            records_seen += 1;
            if records_seen > MAX_COMPONENT_RECORDS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La salida JavaScript supera el máximo de registros de componentes",
                ));
            }
            let Some(result) = result.as_object() else {
                continue;
            };
            let Some(component) = bounded_field(result.get("component"))? else {
                continue;
            };
            let Some(version) = bounded_field(result.get("version"))? else {
                continue;
            };
            components.insert(JavaScriptComponentHint { component, version });
            if components.len() > MAX_COMPONENT_HINTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La salida JavaScript supera el máximo CHEF de componentes",
                ));
            }
        }
    }
    Ok(ArchivedJavaScriptInventory {
        components: components.into_iter().collect(),
    })
}

fn bounded_field(value: Option<&Value>) -> ChefResult<Option<String>> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_FIELD_CHARS || value.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Un campo de componente JavaScript supera el límite CHEF",
        ));
    }
    Ok(Some(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_javascript_inventory;
    use crate::ErrorCode;

    #[test]
    fn extracts_component_inventory_and_ignores_vulnerability_data() {
        let response = br#"{"data":[{"results":[{"component":"jquery","version":"3.7.1","vulnerabilities":[{"severity":"critical","identifiers":{"CVE":["CVE-2099-0001"]}}]},{"component":"React","version":"18.2.0","vulnerabilities":[{"severity":"high"}]},{"component":"jquery","version":"3.7.1"}]}]}"#;
        let inventory = parse_archived_javascript_inventory(response).unwrap();
        assert_eq!(inventory.components.len(), 2);
        assert!(
            inventory
                .components
                .iter()
                .all(|component| component.component != "Vulnerable JavaScript Library")
        );
    }

    #[test]
    fn ignores_incomplete_component_records_and_rejects_bad_shape() {
        let response = br#"{"data":[{"results":[{"component":"jquery"},{"version":"1.0"},{}]}]}"#;
        assert!(
            parse_archived_javascript_inventory(response)
                .unwrap()
                .components
                .is_empty()
        );
        assert_eq!(
            parse_archived_javascript_inventory(br#"{"data":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_component_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_javascript_inventory(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let results = (0..2049)
            .map(|index| serde_json::json!({"component":format!("lib-{index}"),"version":"1"}))
            .collect::<Vec<_>>();
        let response =
            serde_json::to_vec(&serde_json::json!({"data":[{"results":results}]})).unwrap();
        assert_eq!(
            parse_archived_javascript_inventory(&response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
