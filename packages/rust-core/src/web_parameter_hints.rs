//! Extrae nombres de parámetros ya presentes en una URL archivada.
//!
//! Adapta la representación `WEB_PARAMETER` como dato histórico observado en
//! una URL; no muta rutas, genera payloads ni realiza solicitudes HTTP.

use std::collections::BTreeSet;

use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_URL_BYTES: usize = 8192;
const MAX_QUERY_PAIRS: usize = 128;
const MAX_PARAMETER_NAME_CHARS: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedWebParameterHints {
    /// URL base sin valores de query ni fragmentos.
    pub url_without_query: String,
    /// Nombres de parámetros vistos; sus valores no se conservan.
    pub parameter_names: Vec<String>,
}

/// Extrae nombres de query de una URL HTTP(S) ya archivada.
pub fn parse_archived_web_parameters(url_value: &str) -> ChefResult<ArchivedWebParameterHints> {
    if url_value.len() > MAX_URL_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La URL archivada supera el límite de bytes",
        ));
    }
    let mut url = Url::parse(url_value)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La URL archivada no es válida"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Solo se aceptan URL HTTP(S) sin credenciales",
        ));
    }

    let mut parameter_names = BTreeSet::new();
    if let Some(query) = url.query() {
        if query.split('&').count() > MAX_QUERY_PAIRS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La URL archivada supera el máximo de parámetros",
            ));
        }
        for (name, _value) in url.query_pairs() {
            let name = name.trim();
            if name.is_empty() {
                continue;
            }
            if name.chars().count() > MAX_PARAMETER_NAME_CHARS || name.chars().any(char::is_control)
            {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "Un nombre de parámetro supera el límite CHEF",
                ));
            }
            parameter_names.insert(name.to_owned());
        }
    }
    url.set_query(None);
    url.set_fragment(None);
    Ok(ArchivedWebParameterHints {
        url_without_query: url.to_string(),
        parameter_names: parameter_names.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_web_parameters;
    use crate::ErrorCode;

    #[test]
    fn extracts_parameter_names_and_discards_values() {
        let result = parse_archived_web_parameters(
            "https://example.com/search?id=secret&id=other&%74ag=private#section",
        )
        .unwrap();
        assert_eq!(result.url_without_query, "https://example.com/search");
        assert_eq!(result.parameter_names, vec!["id", "tag"]);
        assert!(!result.url_without_query.contains("secret"));
    }

    #[test]
    fn rejects_non_http_urls_and_ignores_empty_names() {
        let result = parse_archived_web_parameters("https://example.com/?=empty&name=").unwrap();
        assert_eq!(result.parameter_names, vec!["name"]);
        assert_eq!(
            parse_archived_web_parameters("javascript:alert(1)")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_url_pair_and_name_limits() {
        let oversized = format!("https://example.com/?{}", "x".repeat(8192));
        assert_eq!(
            parse_archived_web_parameters(&oversized).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let too_many = (0..129)
            .map(|index| format!("p{index}=v"))
            .collect::<Vec<_>>()
            .join("&");
        assert_eq!(
            parse_archived_web_parameters(&format!("https://example.com/?{too_many}"))
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
