//! Resumen acotado de una respuesta GraphQL archivada.
//!
//! Adapta únicamente la transformación de respuesta del módulo de origen:
//! no consulta endpoints, no guarda archivos y no crea hallazgos. Los nombres
//! de tipos/campos, descripciones, argumentos y valores no se conservan.

use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_TYPES: usize = 2048;
const MAX_FIELDS: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGraphqlSchemaHint {
    /// Origen HTTP(S) saneado; nunca incluye ruta, consulta ni credenciales.
    pub scoped_origin: String,
    /// Huella para relacionar el resumen con la evidencia archivada original.
    pub response_sha256: String,
    pub type_count: usize,
    pub object_type_count: usize,
    pub field_count: usize,
    pub deprecated_field_count: usize,
    pub has_query_root: bool,
    pub has_mutation_root: bool,
}

/// Resume `data.__schema.types` en una respuesta ya archivada.
///
/// `source_url` se valida contra la raíz explícita para evitar asociar la
/// evidencia a un host fuera de alcance. Una respuesta que no contiene un
/// esquema GraphQL utilizable produce `Ok(None)`, no un hallazgo negativo.
pub fn summarize_archived_graphql_schema(
    query_domain: &str,
    source_url: &str,
    response: &[u8],
) -> ChefResult<Option<ArchivedGraphqlSchemaHint>> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta GraphQL archivada supera el límite de bytes",
        ));
    }

    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz consultada debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz consultada debe ser un dominio registrable ICANN",
        ));
    }
    let scoped_origin = match scoped_origin(source_url, &query_domain) {
        Some(origin) => origin,
        None => return Ok(None),
    };

    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta GraphQL archivada no contiene JSON válido",
        )
    })?;
    let Some(schema) = document
        .get("data")
        .and_then(Value::as_object)
        .and_then(|data| data.get("__schema"))
        .and_then(Value::as_object)
    else {
        return Ok(None);
    };
    let Some(types) = schema.get("types").and_then(Value::as_array) else {
        return Ok(None);
    };
    if types.is_empty() {
        return Ok(None);
    }
    if types.len() > MAX_TYPES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta GraphQL supera el máximo CHEF de tipos",
        ));
    }

    let mut object_type_count = 0usize;
    let mut field_count = 0usize;
    let mut deprecated_field_count = 0usize;
    for type_entry in types {
        let Some(type_object) = type_entry.as_object() else {
            continue;
        };
        if type_object.get("kind").and_then(Value::as_str) == Some("OBJECT") {
            object_type_count += 1;
        }
        let Some(fields) = type_object.get("fields").and_then(Value::as_array) else {
            continue;
        };
        field_count = field_count.checked_add(fields.len()).ok_or(ChefError::new(
            ErrorCode::LimitExceeded,
            "El recuento de campos GraphQL excede el límite numérico",
        ))?;
        if field_count > MAX_FIELDS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta GraphQL supera el máximo CHEF de campos",
            ));
        }
        deprecated_field_count += fields
            .iter()
            .filter(|field| {
                field
                    .as_object()
                    .and_then(|field| field.get("isDeprecated"))
                    .and_then(Value::as_bool)
                    == Some(true)
            })
            .count();
    }

    let digest = Sha256::digest(response);
    Ok(Some(ArchivedGraphqlSchemaHint {
        scoped_origin,
        response_sha256: format!("{digest:x}"),
        type_count: types.len(),
        object_type_count,
        field_count,
        deprecated_field_count,
        has_query_root: schema.get("queryType").is_some_and(valid_graphql_root),
        has_mutation_root: schema.get("mutationType").is_some_and(valid_graphql_root),
    }))
}

fn valid_graphql_root(value: &Value) -> bool {
    let Some(root) = value.as_object() else {
        return false;
    };
    root.get("kind").and_then(Value::as_str) == Some("OBJECT")
        && root
            .get("name")
            .and_then(Value::as_str)
            .is_some_and(|name| !name.trim().is_empty())
}

fn scoped_origin(source_url: &str, query_domain: &str) -> Option<String> {
    if source_url.len() > 4096 {
        return None;
    }
    let url = Url::parse(source_url).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str().and_then(canonical_dns_name)?;
    let suffix = format!(".{query_domain}");
    if host != query_domain && !host.ends_with(&suffix) {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::summarize_archived_graphql_schema;
    use crate::ErrorCode;

    #[test]
    fn summarizes_scoped_archived_schema_without_retaining_names_or_secrets() {
        let response = br#"{"data":{"__schema":{"queryType":{"kind":"OBJECT","name":"SecretQuery"},"mutationType":{"kind":"OBJECT","name":"Mutation"},"types":[{"kind":"OBJECT","name":"InternalCustomer","fields":[{"name":"email","isDeprecated":true},{"name":"token","isDeprecated":false}]},{"kind":"SCALAR","name":"SecretScalar"}]}}}"#;
        let summary = summarize_archived_graphql_schema(
            "example.com",
            "https://api.example.com/graphql?token=private",
            response,
        )
        .unwrap()
        .unwrap();

        assert_eq!(summary.scoped_origin, "https://api.example.com");
        assert_eq!(summary.type_count, 2);
        assert_eq!(summary.object_type_count, 1);
        assert_eq!(summary.field_count, 2);
        assert_eq!(summary.deprecated_field_count, 1);
        assert!(summary.has_query_root);
        assert!(summary.has_mutation_root);
        assert_eq!(summary.response_sha256.len(), 64);
        let rendered = format!("{summary:?}");
        assert!(!rendered.contains("SecretQuery"));
        assert!(!rendered.contains("InternalCustomer"));
        assert!(!rendered.contains("email"));
        assert!(!rendered.contains("private"));
    }

    #[test]
    fn rejects_ambiguous_root_and_omits_out_of_scope_or_credential_urls() {
        assert_eq!(
            summarize_archived_graphql_schema(
                "localhost",
                "https://api.example.com/graphql",
                br#"{"data":{"__schema":{"types":[{"kind":"OBJECT"}]}}}"#,
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
        let response = br#"{"data":{"__schema":{"types":[{"kind":"OBJECT"}]}}}"#;
        assert!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://evil-example.com/graphql",
                response
            )
            .unwrap()
            .is_none()
        );
        assert!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://user:secret@api.example.com/graphql",
                response
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn returns_no_hint_for_non_schema_json_and_rejects_invalid_json() {
        assert!(
            summarize_archived_graphql_schema("example.com", "https://example.com/graphql", b"{}")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://example.com/graphql",
                b"not-json",
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn ignores_root_values_without_object_kind_and_nonempty_name() {
        let response = br#"{"data":{"__schema":{"queryType":{"kind":"OBJECT","name":"  "},"mutationType":"not-a-type","types":[{"kind":"OBJECT","name":"Query"}]}}}"#;
        let summary = summarize_archived_graphql_schema(
            "example.com",
            "https://example.com/graphql",
            response,
        )
        .unwrap()
        .unwrap();
        assert!(!summary.has_query_root);
        assert!(!summary.has_mutation_root);

        let response = br#"{"data":{"__schema":{"queryType":{"name":"Query"},"mutationType":{"kind":"SCALAR","name":"Mutation"},"types":[{"kind":"OBJECT","name":"Query"}]}}}"#;
        let summary = summarize_archived_graphql_schema(
            "example.com",
            "https://example.com/graphql",
            response,
        )
        .unwrap()
        .unwrap();
        assert!(!summary.has_query_root);
        assert!(!summary.has_mutation_root);
    }

    #[test]
    fn enforces_response_type_and_field_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://example.com/graphql",
                &oversized,
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );

        let types = vec![serde_json::json!({"kind":"SCALAR"}); 2049];
        let response =
            serde_json::to_vec(&serde_json::json!({"data":{"__schema":{"types":types}}})).unwrap();
        assert_eq!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://example.com/graphql",
                &response,
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );

        let many_fields = vec![serde_json::json!({"isDeprecated":false}); 8193];
        let response = serde_json::to_vec(&serde_json::json!({"data":{"__schema":{"types":[{"kind":"OBJECT","fields":many_fields}]}}})).unwrap();
        assert_eq!(
            summarize_archived_graphql_schema(
                "example.com",
                "https://example.com/graphql",
                &response,
            )
            .unwrap_err()
            .code,
            ErrorCode::LimitExceeded
        );
    }
}
