//! Extrae referencias de repositorio desde una URL archivada o texto de enlace.
//!
//! Las referencias no validan titularidad, pertenencia al objetivo ni permisos
//! para descargar o analizar el contenido del repositorio.

use std::collections::BTreeMap;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_BYTES: usize = 1024 * 1024;
const MAX_MATCHES: usize = 4096;
const MAX_REPOSITORIES: usize = 512;
const MAX_SLUG_BYTES: usize = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepositoryPlatform {
    Git,
    Docker,
    Postman,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepositoryLinkHint {
    pub platform: RepositoryPlatform,
    pub owner: String,
    pub repository: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedRepositoryLinkHints {
    pub repositories: Vec<RepositoryLinkHint>,
}

/// Extrae enlaces reconocidos por `code_repository` de texto archivado.
///
/// Se aceptan GitHub, GitLab.com/.org, Docker Hub y Postman. El texto no se
/// visita ni se interpreta como una orden para clonar o descargar contenido.
pub fn parse_archived_repository_links(
    source_text: &str,
) -> ChefResult<ArchivedRepositoryLinkHints> {
    if source_text.len() > MAX_INPUT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El texto archivado supera el límite de bytes",
        ));
    }
    let lowered = source_text.to_ascii_lowercase();
    let patterns = [
        ("hub.docker.com/r/", RepositoryPlatform::Docker, false),
        ("www.postman.com/", RepositoryPlatform::Postman, false),
        ("github.com/", RepositoryPlatform::Git, true),
        ("gitlab.com/", RepositoryPlatform::Git, false),
        ("gitlab.org/", RepositoryPlatform::Git, false),
    ];
    let mut repositories = BTreeMap::new();
    let mut matches_seen = 0;
    for (marker, platform, allow_www_prefix) in patterns {
        let mut cursor = 0;
        while let Some(relative) = lowered[cursor..].find(marker) {
            let start = cursor + relative;
            cursor = start + marker.len();
            if !valid_host_boundary(source_text, start, allow_www_prefix) {
                continue;
            }
            matches_seen += 1;
            if matches_seen > MAX_MATCHES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El texto archivado supera el máximo de referencias",
                ));
            }
            let path_start = cursor;
            let Some((owner, repo)) = parse_owner_repository(source_text, path_start) else {
                continue;
            };
            let repo_end = path_start + owner.len() + 1 + repo.len();
            if source_text.as_bytes().get(repo_end).is_some_and(|byte| {
                !matches!(
                    byte,
                    b'/' | b'?'
                        | b'#'
                        | b' '
                        | b'\t'
                        | b'\r'
                        | b'\n'
                        | b'"'
                        | b'\''
                        | b'<'
                        | b'>'
                        | b')'
                        | b']'
                )
            }) {
                continue;
            }
            let owner = owner.to_ascii_lowercase();
            let repo = repo.to_ascii_lowercase();
            let marker_host = marker.trim_end_matches('/');
            let path = format!("{owner}/{repo}");
            let url = format!("https://{marker_host}/{path}");
            repositories
                .entry(url.clone())
                .or_insert(RepositoryLinkHint {
                    platform,
                    owner,
                    repository: repo,
                    url,
                });
            if repositories.len() > MAX_REPOSITORIES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El texto supera el máximo CHEF de repositorios referenciados",
                ));
            }
        }
    }
    Ok(ArchivedRepositoryLinkHints {
        repositories: repositories.into_values().collect(),
    })
}

fn valid_host_boundary(value: &str, start: usize, allow_www_prefix: bool) -> bool {
    if start == 0 {
        return true;
    }
    let previous = value.as_bytes()[start - 1];
    if previous == b'.' {
        return allow_www_prefix
            && start >= 4
            && value
                .get(start - 4..start)
                .is_some_and(|prefix| prefix.eq_ignore_ascii_case("www."))
            && (start == 4
                || (!value.as_bytes()[start - 5].is_ascii_alphanumeric()
                    && value.as_bytes()[start - 5] != b'-'
                    && value.as_bytes()[start - 5] != b'.'));
    }
    !previous.is_ascii_alphanumeric() && previous != b'-' && previous != b'.'
}

fn parse_owner_repository(value: &str, start: usize) -> Option<(&str, &str)> {
    let bytes = value.as_bytes();
    let owner_end = read_slug_end(bytes, start)?;
    let slash = bytes.get(owner_end).copied()?;
    if slash != b'/' {
        return None;
    }
    let repository_start = owner_end + 1;
    Some((
        value.get(start..owner_end)?,
        value.get(repository_start..read_slug_end(bytes, repository_start)?)?,
    ))
}

fn read_slug_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while end < bytes.len()
        && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'-'))
    {
        end += 1;
        if end - start > MAX_SLUG_BYTES {
            return None;
        }
    }
    (end > start).then_some(end)
}

#[cfg(test)]
mod tests {
    use super::{RepositoryPlatform, parse_archived_repository_links};
    use crate::ErrorCode;

    #[test]
    fn extracts_canonical_references_for_supported_platforms() {
        let text = "https://GitHub.com/Org/Repo/issues https://gitlab.org/team/project/-/tree/main https://hub.docker.com/r/acme/api/tags https://www.postman.com/team/workspace/request";
        let result = parse_archived_repository_links(text).unwrap();
        assert_eq!(result.repositories.len(), 4);
        assert!(
            result
                .repositories
                .iter()
                .any(|repo| repo.platform == RepositoryPlatform::Docker)
        );
        assert!(
            result
                .repositories
                .iter()
                .any(|repo| repo.url == "https://github.com/org/repo")
        );
        assert!(
            result
                .repositories
                .iter()
                .any(|repo| repo.url == "https://hub.docker.com/r/acme/api")
        );
    }

    #[test]
    fn rejects_host_collisions_and_malformed_paths() {
        let text = "evilgithub.com/user/repo evil.github.com/user/repo evil.www.github.com/user/repo github.com.evil.test/user/repo github.com/.bad/repo gitlab.com/user/invalid.name postman.com/user/repo";
        assert!(
            parse_archived_repository_links(text)
                .unwrap()
                .repositories
                .is_empty()
        );
    }

    #[test]
    fn accepts_only_exact_hosts_or_the_supported_www_github_host() {
        let result = parse_archived_repository_links(
            "https://github.com/acme/core https://www.github.com/acme/web https://evil.www.github.com/acme/fake",
        )
        .unwrap();
        assert_eq!(result.repositories.len(), 2);
        assert!(
            result
                .repositories
                .iter()
                .any(|repository| repository.url == "https://github.com/acme/core")
        );
        assert!(
            result
                .repositories
                .iter()
                .any(|repository| repository.url == "https://github.com/acme/web")
        );
    }

    #[test]
    fn enforces_input_match_and_result_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_repository_links(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_matches = "github.com/owner/repo ".repeat(4097);
        assert_eq!(
            parse_archived_repository_links(&too_many_matches)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let many = (0..513)
            .map(|i| format!("github.com/owner/repo{i} "))
            .collect::<String>();
        assert_eq!(
            parse_archived_repository_links(&many).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
