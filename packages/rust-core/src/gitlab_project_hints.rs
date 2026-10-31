//! Normaliza referencias a proyectos desde una lista REST de GitLab archivada.
//!
//! Un `web_url` observado no acredita quién controla el proyecto ni su relación
//! con un grupo/usuario. El parser no consulta GitLab ni sigue enlaces.

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
pub struct ArchivedGitlabProjectHints {
    pub projects: Vec<GitlabProjectHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitlabProjectHint {
    pub path: String,
    pub url: String,
}

/// Extrae URLs `web_url` de un array de proyectos archivado y elimina query y
/// fragmento. Solo acepta HTTPS en `gitlab.com` o `gitlab.org`.
pub fn parse_archived_gitlab_projects(response: &[u8]) -> ChefResult<ArchivedGitlabProjectHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de proyectos GitLab supera el límite de bytes",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de proyectos GitLab no contiene JSON válido",
        )
    })?;
    let entries = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta de proyectos GitLab debe ser una lista JSON",
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
        let Some(project) = normalize_project_url(web_url) else {
            continue;
        };
        projects.entry(project.url.clone()).or_insert(project);
        if projects.len() > MAX_PROJECT_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta GitLab supera el máximo CHEF de referencias",
            ));
        }
    }
    Ok(ArchivedGitlabProjectHints {
        projects: projects.into_values().collect(),
    })
}

fn normalize_project_url(value: &str) -> Option<GitlabProjectHint> {
    if value.len() > MAX_URL_BYTES || value.chars().any(char::is_control) {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str()?.to_ascii_lowercase().as_str(),
            "gitlab.com" | "gitlab.org"
        )
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
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
                || segment == &"."
                || segment == &".."
                || segment.chars().count() > 128
        })
    {
        return None;
    }
    let path = segments.join("/");
    url.set_query(None);
    url.set_fragment(None);
    Some(GitlabProjectHint {
        path,
        url: url.to_string().trim_end_matches('/').to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_gitlab_projects;
    use crate::ErrorCode;

    #[test]
    fn extracts_canonical_archived_project_urls_without_namespace_claims() {
        let response = br#"[{"web_url":"https://gitlab.com/group/subgroup/project?ref=private#readme","namespace":{"full_path":"elsewhere"}},{"web_url":"https://gitlab.org/team/tool"},{"web_url":"https://gitlab.com/group/subgroup/project?ref=duplicate"}]"#;
        let result = parse_archived_gitlab_projects(response).unwrap();
        assert_eq!(result.projects.len(), 2);
        assert!(result.projects.iter().any(|project| {
            project.path == "group/subgroup/project"
                && project.url == "https://gitlab.com/group/subgroup/project"
        }));
        assert!(
            result
                .projects
                .iter()
                .any(|project| project.path == "team/tool")
        );
    }

    #[test]
    fn rejects_untrusted_hosts_and_invalid_response_shapes() {
        let response = br#"[{"web_url":"https://gitlab.com.evil.test/team/repo"},{"web_url":"https://gitlab.com@evil.test/team/repo"},{"web_url":"http://gitlab.com/team/repo"},{"web_url":"https://gitlab.com/single"}]"#;
        assert!(
            parse_archived_gitlab_projects(response)
                .unwrap()
                .projects
                .is_empty()
        );
        assert_eq!(
            parse_archived_gitlab_projects(br#"{"web_url":"https://gitlab.com/a/b"}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_entry_and_result_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_gitlab_projects(&oversized).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let too_many_entries = vec![serde_json::json!({}); 4097];
        let response = serde_json::to_vec(&too_many_entries).unwrap();
        assert_eq!(
            parse_archived_gitlab_projects(&response).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let projects = (0..513)
            .map(|index| {
                serde_json::json!({"web_url": format!("https://gitlab.com/group/project{index}")})
            })
            .collect::<Vec<_>>();
        let response = serde_json::to_vec(&projects).unwrap();
        assert_eq!(
            parse_archived_gitlab_projects(&response).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
