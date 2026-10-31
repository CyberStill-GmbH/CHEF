//! Extrae referencias de perfil GitHub con enlaces web en scope.
//!
//! Procesa una respuesta GraphQL archivada. Omite correo electrónico y no
//! acredita titularidad, empleo ni relación real entre perfil y dominio.

use std::collections::BTreeMap;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_EDGES: usize = 4096;
const MAX_PROFILES: usize = 512;
const MAX_LOGIN_CHARS: usize = 64;
const MAX_URL_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGithubProfileHints {
    pub query_domain: String,
    pub profiles: Vec<GithubProfileHint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GithubProfileHint {
    pub login: String,
    pub profile_url: String,
    pub in_scope_website_url: String,
}

/// Extrae perfiles cuyo website URL archivado apunta a la raíz consultada.
///
/// Solo se conservan `login` y el enlace web, sin query/fragmento; las demás
/// propiedades de usuario, incluido email, se ignoran.
pub fn parse_archived_github_profile_search(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedGithubProfileHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de perfiles archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta de perfiles debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta de perfiles debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta de perfiles no contiene JSON válido",
        )
    })?;
    let edges = document
        .as_object()
        .and_then(|object| object.get("data"))
        .and_then(Value::as_object)
        .and_then(|data| data.get("search"))
        .and_then(Value::as_object)
        .and_then(|search| search.get("edges"))
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta GraphQL no contiene data.search.edges",
        ))?;
    if edges.len() > MAX_EDGES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de perfiles supera el máximo de resultados",
        ));
    }

    let mut profiles = BTreeMap::new();
    let suffix = format!(".{query_domain}");
    for edge in edges {
        let Some(node) = edge
            .as_object()
            .and_then(|object| object.get("node"))
            .and_then(Value::as_object)
        else {
            continue;
        };
        let Some(login) = node
            .get("login")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|login| valid_login(login))
        else {
            continue;
        };
        let Some(website_url) = node
            .get("websiteUrl")
            .and_then(Value::as_str)
            .and_then(|url| normalize_in_scope_website(url, &query_domain, &suffix))
        else {
            continue;
        };
        let login = login.to_ascii_lowercase();
        profiles.entry(login.clone()).or_insert(GithubProfileHint {
            profile_url: format!("https://github.com/{login}"),
            login,
            in_scope_website_url: website_url,
        });
        if profiles.len() > MAX_PROFILES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta supera el máximo CHEF de perfiles",
            ));
        }
    }
    Ok(ArchivedGithubProfileHints {
        query_domain,
        profiles: profiles.into_values().collect(),
    })
}

fn valid_login(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_LOGIN_CHARS
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

fn normalize_in_scope_website(value: &str, domain: &str, suffix: &str) -> Option<String> {
    if value.len() > MAX_URL_BYTES {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str().and_then(canonical_dns_name)?;
    if host != domain && !host.ends_with(suffix) {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_github_profile_search;
    use crate::ErrorCode;

    #[test]
    fn keeps_profiles_with_scoped_websites_and_omits_email() {
        let response = br#"{"data":{"search":{"edges":[{"node":{"login":"Alice","email":"alice@example.com","websiteUrl":"https://www.example.com/about?token=secret"}},{"node":{"login":"bob","email":"bob@example.com","websiteUrl":"https://outside.test"}},{"node":{"login":"mallory","email":"mallory@example.com"}}]}}}"#;
        let result = parse_archived_github_profile_search("EXAMPLE.COM.", response).unwrap();
        assert_eq!(result.profiles.len(), 1);
        assert_eq!(result.profiles[0].login, "alice");
        assert_eq!(
            result.profiles[0].in_scope_website_url,
            "https://www.example.com/about"
        );
    }

    #[test]
    fn validates_query_and_graphql_shape() {
        assert_eq!(
            parse_archived_github_profile_search(
                "evil-example.com",
                br#"{"data":{"search":{"edges":[]}}}"#
            )
            .unwrap()
            .profiles
            .len(),
            0
        );
        assert_eq!(
            parse_archived_github_profile_search("https://example.com", b"{}")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_github_profile_search("example.com", br#"{"data":{}}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_edge_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_github_profile_search("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let edges = vec![
            serde_json::json!({"node":{"login":"user","websiteUrl":"https://example.com"}});
            4097
        ];
        let response =
            serde_json::to_vec(&serde_json::json!({"data":{"search":{"edges":edges}}})).unwrap();
        assert_eq!(
            parse_archived_github_profile_search("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
