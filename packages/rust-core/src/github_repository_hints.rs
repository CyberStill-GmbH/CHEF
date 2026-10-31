//! Normaliza referencias a repositorios desde respuestas GitHub archivadas.
//!
//! La presencia de un enlace no valida titularidad, relación con la organización
//! consultada, disponibilidad ni autorización para analizar su contenido.

use std::collections::BTreeMap;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_REPOSITORY_ENTRIES: usize = 4096;
const MAX_RESULTS: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGithubRepositoryHints {
    pub repositories: Vec<GithubRepositoryHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubRepositoryHint {
    pub owner: String,
    pub repository: String,
    pub url: String,
}

/// Extrae URLs canónicas `github.com/owner/repository` desde un JSON archivado.
///
/// `expected_owner`, si se proporciona, actúa solo como filtro literal y no
/// acredita la relación entre el usuario/organización y el repositorio.
pub fn parse_archived_github_repositories(
    response: &[u8],
    expected_owner: Option<&str>,
    result_limit: usize,
) -> ChefResult<ArchivedGithubRepositoryHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de repositorios archivada supera el límite de bytes",
        ));
    }
    if result_limit == 0 || result_limit > MAX_RESULTS {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El límite de repositorios debe estar entre 1 y 500",
        ));
    }
    let expected_owner = expected_owner
        .map(|owner| {
            if valid_slug(owner) {
                Ok(owner.to_ascii_lowercase())
            } else {
                Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El perfil esperado de GitHub no es válido",
                ))
            }
        })
        .transpose()?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de repositorios no contiene JSON válido",
        )
    })?;
    let repositories = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta de repositorios debe ser una lista JSON",
    ))?;
    if repositories.len() > MAX_REPOSITORY_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta supera el máximo de repositorios",
        ));
    }

    let mut hints = BTreeMap::new();
    for repository in repositories {
        let Some(url_value) = repository
            .as_object()
            .and_then(|object| object.get("html_url"))
            .and_then(Value::as_str)
        else {
            continue;
        };
        let Some(hint) = normalize_repository_url(url_value) else {
            continue;
        };
        if expected_owner
            .as_deref()
            .is_some_and(|owner| !hint.owner.eq_ignore_ascii_case(owner))
        {
            continue;
        }
        hints.entry(hint.url.clone()).or_insert(hint);
        if hints.len() > MAX_RESULTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta supera el máximo CHEF de referencias de repositorio",
            ));
        }
    }
    Ok(ArchivedGithubRepositoryHints {
        repositories: hints.into_values().take(result_limit).collect(),
    })
}

fn normalize_repository_url(value: &str) -> Option<GithubRepositoryHint> {
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
    let mut segments = url.path_segments()?;
    let owner = segments.next()?.to_ascii_lowercase();
    let repository = segments.next()?.trim_end_matches('/').to_ascii_lowercase();
    let extra_segment = segments.next();
    let has_extra_segment =
        extra_segment.is_some_and(|segment| !segment.is_empty()) || segments.next().is_some();
    if has_extra_segment || !valid_slug(&owner) || !valid_slug(&repository) {
        return None;
    }
    url.set_path(&format!("/{owner}/{repository}"));
    Some(GithubRepositoryHint {
        owner,
        repository,
        url: url.to_string(),
    })
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::parse_archived_github_repositories;
    use crate::ErrorCode;

    #[test]
    fn normalizes_deduplicates_and_filters_archived_repository_links() {
        let response = br#"[{"html_url":"https://github.com/Example/Api"},{"html_url":"https://github.com/example/api/"},{"html_url":"https://github.com/example/other"},{"html_url":"https://evil.test/example/repo"},{"html_url":"https://github.com/example/api/issues"}]"#;
        let result = parse_archived_github_repositories(response, Some("EXAMPLE"), 50).unwrap();
        assert_eq!(result.repositories.len(), 2);
        assert_eq!(result.repositories[0].url, "https://github.com/example/api");
        assert_eq!(result.repositories[1].repository, "other");
    }

    #[test]
    fn rejects_invalid_profile_and_response_shapes() {
        assert_eq!(
            parse_archived_github_repositories(b"[]", Some("https://github.com/example"), 10)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_github_repositories(br#"{"items":[]}"#, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_entry_and_result_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_github_repositories(&oversized, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let entries = vec![serde_json::json!({"html_url":"https://github.com/a/repo"}); 4097];
        let response = serde_json::to_vec(&entries).unwrap();
        assert_eq!(
            parse_archived_github_repositories(&response, None, 10)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_github_repositories(b"[]", None, 501)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
