//! Normaliza metadatos de workflows GitHub desde una respuesta archivada.
//!
//! Solo conserva catálogo descriptivo; no accede a API, ejecuciones, logs o
//! artefactos, y no escribe contenido al filesystem.

use std::collections::BTreeMap;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_WORKFLOWS: usize = 2048;
const MAX_WORKFLOW_HINTS: usize = 512;
const MAX_TEXT_CHARS: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGithubWorkflows {
    pub workflows: Vec<GithubWorkflowHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubWorkflowHint {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: Option<String>,
    pub url: Option<String>,
}

/// Extrae workflows descriptivos de una respuesta GitHub archivada.
///
/// Un workflow registrado no implica que haya corrido, esté habilitado o sea
/// seguro. Los elementos incompletos se omiten y los IDs deduplican el listado.
pub fn parse_archived_github_workflows(response: &[u8]) -> ChefResult<ArchivedGithubWorkflows> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de workflows archivada supera el límite de bytes",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de workflows no contiene JSON válido",
        )
    })?;
    let workflows = document
        .as_object()
        .and_then(|object| object.get("workflows"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta debe incluir una lista workflows",
        ))?;
    if workflows.len() > MAX_WORKFLOWS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta supera el máximo de workflows",
        ));
    }

    let mut hints = BTreeMap::new();
    for workflow in workflows {
        let Some(workflow) = workflow.as_object() else {
            continue;
        };
        if workflow.len() > 64 {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Un registro workflow supera el máximo de campos",
            ));
        }
        let Some(id) = parse_id(workflow.get("id")) else {
            continue;
        };
        let Some(name) = bounded_text(workflow.get("name"))? else {
            continue;
        };
        let Some(path) = bounded_text(workflow.get("path"))? else {
            continue;
        };
        let state = bounded_text(workflow.get("state"))?;
        let url = workflow
            .get("html_url")
            .and_then(Value::as_str)
            .and_then(normalize_github_url);
        hints.entry(id).or_insert(GithubWorkflowHint {
            id,
            name,
            path,
            state,
            url,
        });
        if hints.len() > MAX_WORKFLOW_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta supera el máximo CHEF de workflows",
            ));
        }
    }
    Ok(ArchivedGithubWorkflows {
        workflows: hints.into_values().collect(),
    })
}

fn parse_id(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Number(number) => number.as_u64(),
        Value::String(value) => value.parse().ok(),
        _ => None,
    }
}

fn bounded_text(value: Option<&Value>) -> ChefResult<Option<String>> {
    let Some(value) = value.and_then(Value::as_str) else {
        return Ok(None);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > MAX_TEXT_CHARS || value.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Un campo de workflow supera el límite CHEF",
        ));
    }
    Ok(Some(value.to_owned()))
}

fn normalize_github_url(value: &str) -> Option<String> {
    let mut url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.host_str()?.eq_ignore_ascii_case("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    url.set_host(Some("github.com")).ok()?;
    Some(url.to_string().trim_end_matches('/').to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_github_workflows;
    use crate::ErrorCode;

    #[test]
    fn extracts_catalog_metadata_and_deduplicates_by_id() {
        let response = br#"{"workflows":[{"id":12,"name":"Build","path":".github/workflows/build.yml","state":"active","html_url":"https://github.com/org/repo/actions/workflows/build.yml"},{"id":"12","name":"Duplicate","path":"build.yml"},{"id":13,"name":"Release","path":".github/workflows/release.yml","state":"disabled","html_url":"https://evil.test/org/repo"}]}"#;
        let result = parse_archived_github_workflows(response).unwrap();
        assert_eq!(result.workflows.len(), 2);
        assert_eq!(result.workflows[0].name, "Build");
        assert_eq!(
            result.workflows[0].url.as_deref(),
            Some("https://github.com/org/repo/actions/workflows/build.yml")
        );
        assert_eq!(result.workflows[1].url, None);
    }

    #[test]
    fn rejects_wrong_shapes_and_omits_incomplete_rows() {
        let result = parse_archived_github_workflows(
            br#"{"workflows":[{"id":1,"name":"","path":"workflow.yml"},null]}"#,
        )
        .unwrap();
        assert!(result.workflows.is_empty());
        assert_eq!(
            parse_archived_github_workflows(br#"{"workflows":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_workflow_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_github_workflows(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let workflows = vec![serde_json::json!({"id":1,"name":"Build","path":"build.yml"}); 2049];
        let response = serde_json::to_vec(&serde_json::json!({"workflows": workflows})).unwrap();
        assert_eq!(
            parse_archived_github_workflows(&response).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
