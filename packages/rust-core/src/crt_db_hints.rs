//! Adapta filas archivadas de `modules/crt_db.py` a candidatos CHEF.
//!
//! Acepta únicamente resultados JSON ya guardados de la consulta PostgreSQL
//! crt.sh. No abre conexiones, no ejecuta SQL y no interpreta candidatos como
//! resoluciones DNS actuales.

use serde_json::{Value, json};

use crate::{ChefError, ChefResult, CrtShSubdomainPage, ErrorCode};

// Deja margen para el ID sintético al delegar la validación en `crtsh.rs`.
const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ROWS: usize = 2_048;
const MAX_NAME_VALUE_BYTES: usize = 16 * 1024;

/// Normaliza filas `name_value` archivadas de la salida de `crt_db.py`.
///
/// Solo conserva descendientes estrictos de la raíz registrable indicada,
/// usando el mismo filtro, deduplicación y límite de candidatos que el
/// adaptador JSON crt.sh existente.
pub fn parse_archived_crt_db_rows(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<CrtShSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las filas crt_db archivadas superan el límite CHEF de bytes",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta crt_db debe ser una raíz registrable ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "Las filas crt_db archivadas no contienen JSON válido",
        )
    })?;
    let rows = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "Las filas crt_db archivadas deben ser una lista JSON",
    ))?;
    if rows.len() > MAX_ROWS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las filas crt_db archivadas superan el límite CHEF de registros",
        ));
    }

    let mut certificates = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let object = row.as_object().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Una fila crt_db archivada no es un objeto JSON",
        ))?;
        let Some(name_value) = object.get("name_value") else {
            continue;
        };
        let Some(name_value) = name_value.as_str() else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "El campo name_value de crt_db no es texto",
            ));
        };
        if name_value.len() > MAX_NAME_VALUE_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Un campo name_value de crt_db supera el límite CHEF",
            ));
        }
        certificates.push(json!({
            "id": index + 1,
            "name_value": name_value,
        }));
    }

    let compatible_response = serde_json::to_vec(&certificates).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "No se pudieron normalizar las filas crt_db",
        )
    })?;
    crate::parse_crtsh_subdomains(&query_domain, &compatible_response)
}

#[cfg(test)]
mod tests {
    use super::{MAX_NAME_VALUE_BYTES, MAX_RESPONSE_BYTES, MAX_ROWS, parse_archived_crt_db_rows};
    use crate::ErrorCode;
    use serde_json::json;

    #[test]
    fn parses_archived_database_rows_as_historical_descendant_candidates() {
        let response = br#"[
          {"name_value":"WWW.Example.com\napi.example.com"},
          {"name_value":"api.example.com\n*.example.com"},
          {"name_value":"example.com\nevil-example.com\nexample.com.attacker.test"},
          {"ignored":"no SAN in this row"}
        ]"#;

        let result = parse_archived_crt_db_rows("example.com", response).unwrap();

        assert_eq!(result.query_domain, "example.com");
        assert_eq!(
            result.candidates,
            vec!["api.example.com", "www.example.com"]
        );
    }

    #[test]
    fn rejects_invalid_roots_rows_and_name_value_shapes_atomically() {
        for (root, response) in [
            ("localhost", b"[]".as_slice()),
            ("example.com", b"{}".as_slice()),
            ("example.com", b"not-json".as_slice()),
            ("example.com", b"[null]".as_slice()),
            ("example.com", br#"[{"name_value":42}]"#.as_slice()),
        ] {
            assert_eq!(
                parse_archived_crt_db_rows(root, response).unwrap_err().code,
                ErrorCode::InvalidInput
            );
        }
    }

    #[test]
    fn enforces_response_row_and_name_limits() {
        let oversized = vec![b' '; MAX_RESPONSE_BYTES + 1];
        assert_eq!(
            parse_archived_crt_db_rows("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let too_many_rows = vec![json!({"name_value":"a.example.com"}); MAX_ROWS + 1];
        let encoded = serde_json::to_vec(&too_many_rows).unwrap();
        assert_eq!(
            parse_archived_crt_db_rows("example.com", &encoded)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let oversized_name = json!([{"name_value":"x".repeat(MAX_NAME_VALUE_BYTES + 1)}]);
        assert_eq!(
            parse_archived_crt_db_rows(
                "example.com",
                &serde_json::to_vec(&oversized_name).unwrap()
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
    }
}
