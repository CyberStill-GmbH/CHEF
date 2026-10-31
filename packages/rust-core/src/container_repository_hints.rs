//! Normaliza repositorios Docker desde respuestas JSON archivadas.
//!
//! Una referencia del registro no prueba quién controla la imagen ni que siga
//! publicada; el parser no realiza solicitudes ni descarga contenido.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_REPOSITORY_ENTRIES: usize = 4096;
const MAX_RESULTS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedContainerRepositories {
    pub repositories: Vec<ContainerRepositoryHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerRepositoryHint {
    pub namespace: String,
    pub name: String,
    pub url: String,
}

/// Extrae `namespace` y `name` de `results` en una respuesta archivada.
///
/// El namespace opcional es un filtro literal aportado por el caller; no se
/// usa para inferir propiedad. Los resultados se deduplican y ordenan.
pub fn parse_archived_container_repositories(
    response: &[u8],
    expected_namespace: Option<&str>,
    result_limit: usize,
) -> ChefResult<ArchivedContainerRepositories> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de contenedores archivada supera el límite de bytes",
        ));
    }
    if result_limit == 0 || result_limit > MAX_RESULTS {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El límite de repositorios debe estar entre 1 y 500",
        ));
    }
    if expected_namespace.is_some_and(|namespace| !valid_name(namespace)) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El namespace esperado no es válido",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de contenedores no contiene JSON válido",
        )
    })?;
    let results = document
        .as_object()
        .and_then(|object| object.get("results"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta debe contener una lista results válida",
        ))?;
    if results.len() > MAX_REPOSITORY_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta supera el máximo de entradas de repositorio",
        ));
    }

    let mut repositories = BTreeMap::new();
    for item in results {
        let Some(item) = item.as_object() else {
            continue;
        };
        let (Some(namespace), Some(name)) = (
            item.get("namespace").and_then(Value::as_str),
            item.get("name").and_then(Value::as_str),
        ) else {
            continue;
        };
        if !valid_name(namespace)
            || !valid_name(name)
            || expected_namespace.is_some_and(|expected| expected != namespace)
        {
            continue;
        }
        let hint = ContainerRepositoryHint {
            namespace: namespace.to_owned(),
            name: name.to_owned(),
            url: format!("https://hub.docker.com/r/{namespace}/{name}"),
        };
        repositories.entry(hint.url.clone()).or_insert(hint);
        if repositories.len() > MAX_RESULTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta supera el máximo CHEF de referencias de contenedor",
            ));
        }
    }
    Ok(ArchivedContainerRepositories {
        repositories: repositories.into_values().take(result_limit).collect(),
    })
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::parse_archived_container_repositories;
    use crate::ErrorCode;

    #[test]
    fn extracts_and_deduplicates_archived_container_references() {
        let response = br#"{"results":[{"namespace":"team","name":"api"},{"namespace":"team","name":"api"},{"namespace":"other","name":"worker"},{"namespace":"bad/name","name":"invalid"}]}"#;
        let result = parse_archived_container_repositories(response, Some("team"), 25).unwrap();
        assert_eq!(result.repositories.len(), 1);
        assert_eq!(
            result.repositories[0].url,
            "https://hub.docker.com/r/team/api"
        );
    }

    #[test]
    fn rejects_wrong_json_shape_and_invalid_filter() {
        assert_eq!(
            parse_archived_container_repositories(br#"{"results":{}}"#, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_container_repositories(br#"{"results":[]}"#, Some("a/b"), 10)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_entry_and_result_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_container_repositories(&oversized, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let entries = vec![serde_json::json!({"namespace":"team","name":"api"}); 4097];
        let response = serde_json::to_vec(&serde_json::json!({"results": entries})).unwrap();
        assert_eq!(
            parse_archived_container_repositories(&response, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_container_repositories(br#"{"results":[]}"#, None, 501)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
