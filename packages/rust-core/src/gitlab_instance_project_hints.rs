//! Extrae proyectos de una respuesta GitLab self-hosted archivada.
//!
//! Cada URL debe compartir exactamente el origin del GitLab explícito que
//! aportó el caller. La referencia no prueba control ni relación del proyecto.

use std::collections::BTreeMap;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_PROJECT_ENTRIES: usize = 4096;
const MAX_PROJECT_HINTS: usize = 512;
const MAX_URL_BYTES: usize = 4096;
const MAX_PATH_SEGMENTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGitlabInstanceProjectHints {
    pub instance_origin: String,
    pub projects: Vec<GitlabInstanceProjectHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitlabInstanceProjectHint {
    pub path: String,
    pub url: String,
}

/// Valida proyectos de un array JSON de GitLab self-hosted archivado.
///
/// `instance_base_url` debe ser un origin explícito, sin path/query/fragmento.
/// El parser solo normaliza URLs que coinciden exactamente con ese origin.
pub fn parse_archived_gitlab_instance_projects(
    instance_base_url: &str,
    response: &[u8],
) -> ChefResult<ArchivedGitlabInstanceProjectHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta GitLab self-hosted supera el límite de bytes",
        ));
    }
    let base = parse_instance_origin(instance_base_url)?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta GitLab self-hosted no contiene JSON válido",
        )
    })?;
    let entries = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta GitLab self-hosted debe ser una lista JSON",
    ))?;
    if entries.len() > MAX_PROJECT_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta GitLab supera el máximo de entradas",
        ));
    }

    let mut projects = BTreeMap::new();
    for entry in entries {
        let Some(web_url) = entry
            .as_object()
            .and_then(|object| object.get("web_url"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(project) = normalize_project_url(web_url, &base) else {
            continue;
        };
        projects.entry(project.url.clone()).or_insert(project);
        if projects.len() > MAX_PROJECT_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta GitLab supera el máximo CHEF de proyectos",
            ));
        }
    }
    Ok(ArchivedGitlabInstanceProjectHints {
        instance_origin: base.origin().ascii_serialization(),
        projects: projects.into_values().collect(),
    })
}

fn parse_instance_origin(value: &str) -> ChefResult<Url> {
    let url = Url::parse(value).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El origin GitLab self-hosted no es una URL válida",
        )
    })?;
    if value.len() > MAX_URL_BYTES
        || !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El origin GitLab debe contener solo esquema, host y puerto opcional",
        ));
    }
    Ok(url)
}

fn normalize_project_url(value: &str, base: &Url) -> Option<GitlabInstanceProjectHint> {
    if value.len() > MAX_URL_BYTES || value.chars().any(char::is_control) {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || url.origin() != base.origin()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let mut segments = url.path_segments()?.collect::<Vec<_>>();
    if segments.last() == Some(&"") {
        segments.pop();
    }
    if segments.len() < 2
        || segments.len() > MAX_PATH_SEGMENTS
        || segments.iter().any(|segment| {
            segment.is_empty()
                || *segment == "."
                || *segment == ".."
                || segment.chars().count() > 128
        })
    {
        return None;
    }
    let path = segments.join("/");
    url.set_query(None);
    url.set_fragment(None);
    Some(GitlabInstanceProjectHint {
        path,
        url: url.to_string().trim_end_matches('/').to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_gitlab_instance_projects;
    use crate::ErrorCode;

    #[test]
    fn keeps_only_projects_from_the_explicit_instance_origin() {
        let response = br#"[{"web_url":"https://gitlab.internal.example/group/project?ref=secret"},{"web_url":"https://other.internal.example/group/project"},{"web_url":"https://gitlab.internal.example:8443/group/project"}]"#;
        let result =
            parse_archived_gitlab_instance_projects("https://gitlab.internal.example", response)
                .unwrap();
        assert_eq!(result.instance_origin, "https://gitlab.internal.example");
        assert_eq!(result.projects.len(), 1);
        assert_eq!(result.projects[0].path, "group/project");
        assert_eq!(
            result.projects[0].url,
            "https://gitlab.internal.example/group/project"
        );
    }

    #[test]
    fn rejects_ambiguous_origin_and_invalid_json_shape() {
        for base in [
            "",
            "https://user@gitlab.internal.example",
            "https://gitlab.internal.example/group",
            "file:///gitlab",
        ] {
            assert_eq!(
                parse_archived_gitlab_instance_projects(base, b"[]")
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidInput
            );
        }
        assert_eq!(
            parse_archived_gitlab_instance_projects(
                "https://gitlab.internal.example",
                br#"{"web_url":"https://gitlab.internal.example/group/project"}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_entry_and_result_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_gitlab_instance_projects("https://gitlab.internal.example", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_entries = vec![serde_json::json!({}); 4097];
        let response = serde_json::to_vec(&too_many_entries).unwrap();
        assert_eq!(
            parse_archived_gitlab_instance_projects("https://gitlab.internal.example", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let projects = (0..513)
            .map(|index| {
                serde_json::json!({"web_url": format!("https://gitlab.internal.example/group/project{index}")})
            })
            .collect::<Vec<_>>();
        let response = serde_json::to_vec(&projects).unwrap();
        assert_eq!(
            parse_archived_gitlab_instance_projects("https://gitlab.internal.example", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
