//! Normaliza enlaces de búsqueda de código archivados en GitHub.
//!
//! Solo conserva referencias a repositorios y archivos; no descarga ni analiza
//! el contenido enlazado y no infiere que contenga un dominio o secreto.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_ITEMS: usize = 2048;
const MAX_LINKS: usize = 512;
const MAX_URL_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGithubCodeSearchHints {
    pub repository_urls: Vec<String>,
    pub raw_file_urls: Vec<String>,
}

/// Extrae enlaces de `items[].html_url` y `items[].repository.html_url`.
///
/// Los enlaces de archivo `blob` se convierten a la forma de URL raw, sin
/// realizar la solicitud. Query y fragmento se omiten para no retener valores.
pub fn parse_archived_github_code_search(
    response: &[u8],
) -> ChefResult<ArchivedGithubCodeSearchHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de búsqueda de código archivada supera el límite",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de búsqueda de código no contiene JSON válido",
        )
    })?;
    let items = document
        .as_object()
        .and_then(|object| object.get("items"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de búsqueda de código debe incluir items",
        ))?;
    if items.len() > MAX_ITEMS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta supera el máximo de resultados de código",
        ));
    }

    let mut repository_urls = BTreeSet::new();
    let mut raw_file_urls = BTreeSet::new();
    for item in items {
        let Some(item) = item.as_object() else {
            continue;
        };
        if let Some(repository_url) = item
            .get("repository")
            .and_then(Value::as_object)
            .and_then(|repository| repository.get("html_url"))
            .and_then(Value::as_str)
            .and_then(normalize_repository_url)
        {
            repository_urls.insert(repository_url);
            ensure_link_limit(repository_urls.len())?;
        }
        if let Some(raw_url) = item
            .get("html_url")
            .and_then(Value::as_str)
            .and_then(to_raw_file_url)
        {
            raw_file_urls.insert(raw_url);
            ensure_link_limit(raw_file_urls.len())?;
        }
    }
    Ok(ArchivedGithubCodeSearchHints {
        repository_urls: repository_urls.into_iter().collect(),
        raw_file_urls: raw_file_urls.into_iter().collect(),
    })
}

fn normalize_repository_url(value: &str) -> Option<String> {
    if value.len() > MAX_URL_BYTES {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.host_str()?.eq_ignore_ascii_case("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let mut segments = url.path_segments()?;
    let owner = segments.next()?;
    let repository = segments.next()?;
    if !valid_slug(owner) || !valid_slug(repository) {
        return None;
    }
    url.set_path(&format!("/{owner}/{repository}"));
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

fn to_raw_file_url(value: &str) -> Option<String> {
    if value.len() > MAX_URL_BYTES {
        return None;
    }
    let url = Url::parse(value).ok()?;
    if url.scheme() != "https"
        || !url.host_str()?.eq_ignore_ascii_case("github.com")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let segments = url.path_segments()?.collect::<Vec<_>>();
    if segments.len() < 5
        || !valid_slug(segments[0])
        || !valid_slug(segments[1])
        || segments[2] != "blob"
        || segments[3].is_empty()
        || segments[4..].iter().all(|segment| segment.is_empty())
    {
        return None;
    }
    let raw_path = url.path().replacen("/blob/", "/", 1);
    let raw = Url::parse(&format!("https://raw.githubusercontent.com{raw_path}")).ok()?;
    (raw.path().len() <= MAX_URL_BYTES).then(|| raw.to_string())
}

fn valid_slug(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
}

fn ensure_link_limit(count: usize) -> ChefResult<()> {
    if count > MAX_LINKS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta supera el máximo CHEF de enlaces GitHub",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_github_code_search;
    use crate::ErrorCode;

    #[test]
    fn converts_archived_blob_links_without_fetching_file_content() {
        let response = br#"{"items":[{"html_url":"https://github.com/Org/Repo/blob/main/src/app.rs?plain=1#L2","repository":{"html_url":"https://github.com/Org/Repo"}},{"html_url":"https://github.com/Org/Repo/blob/main/README.md","repository":{"html_url":"https://github.com/Org/Repo"}},{"html_url":"https://evil.test/a/b/blob/main/file"}]}"#;
        let result = parse_archived_github_code_search(response).unwrap();
        assert_eq!(result.repository_urls, vec!["https://github.com/Org/Repo"]);
        assert_eq!(
            result.raw_file_urls,
            vec![
                "https://raw.githubusercontent.com/Org/Repo/main/README.md",
                "https://raw.githubusercontent.com/Org/Repo/main/src/app.rs"
            ]
        );
    }

    #[test]
    fn rejects_wrong_shapes_and_non_blob_links() {
        let result = parse_archived_github_code_search(
            br#"{"items":[{"html_url":"https://github.com/org/repo/tree/main"},{"repository":{"html_url":"https://evil.test/org/repo"}}]}"#,
        )
        .unwrap();
        assert!(result.repository_urls.is_empty());
        assert!(result.raw_file_urls.is_empty());
        assert_eq!(
            parse_archived_github_code_search(br#"{"items":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_item_and_link_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_github_code_search(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let items = vec![
            serde_json::json!({"html_url":"https://github.com/org/repo/blob/main/file"});
            2049
        ];
        let response = serde_json::to_vec(&serde_json::json!({"items": items})).unwrap();
        assert_eq!(
            parse_archived_github_code_search(&response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
