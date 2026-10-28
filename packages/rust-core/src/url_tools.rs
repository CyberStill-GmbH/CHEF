//! Normalización local de URL adaptada de `core/helpers/validators.py`.
//! Nunca visita la URL ni conserva credenciales, query o fragmento.

use url::Url;

pub(crate) fn canonical_http_url(value: &str) -> Option<String> {
    if value.is_empty()
        || value.len() > 8 * 1024
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") || has_userinfo(&url) || url.host().is_none() {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url.to_string())
}

/// Canonicaliza solo el origen para respuestas HTTP importadas. Los paths
/// pueden incluir identificadores o tokens; no se conservan sin una política
/// de exposición explícita por proyecto.
pub(crate) fn canonical_http_origin(value: &str) -> Option<String> {
    let canonical = canonical_http_url(value)?;
    let mut url = Url::parse(&canonical).ok()?;
    url.set_path("/");
    Some(url.to_string())
}

pub(crate) fn has_userinfo(url: &Url) -> bool {
    if !url.username().is_empty() || url.password().is_some() {
        return true;
    }
    url.as_str()
        .split_once("://")
        .and_then(|(_, remainder)| remainder.split(['/', '?', '#']).next())
        .is_some_and(|authority| authority.contains('@'))
}
