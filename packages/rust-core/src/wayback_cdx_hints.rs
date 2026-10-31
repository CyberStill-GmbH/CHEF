//! Extrae URL históricas de una respuesta CDX archivada.
//!
//! Adapta la conversión de filas de `_parse_cdx_response` sin consultar el
//! archivo web. Los URL resultantes son referencias históricas, no rutas vivas.

use std::collections::BTreeSet;

use serde_json::Value;
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CDX_ROWS: usize = 4096;
const MAX_URL_HINTS: usize = 512;
const MAX_URL_CHARS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedWaybackCdxHints {
    pub query_domain: String,
    /// URL histórica normalizada, sin query ni fragmento.
    pub url_hints: Vec<String>,
}

/// Extrae la primera columna de filas CDX y conserva URL dentro de scope.
///
/// La primera fila se considera cabecera, como en el parser de origen. Solo se
/// aceptan URL HTTP(S) cuyo host sea la raíz consultada o un descendiente.
pub fn parse_archived_wayback_cdx(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedWaybackCdxHints> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta CDX archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta CDX debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta CDX debe ser una raíz registrable ICANN",
        ));
    }
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta CDX no contiene JSON válido",
        )
    })?;
    let rows = document.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta CDX debe ser una matriz JSON",
    ))?;
    if rows.len() > MAX_CDX_ROWS + 1 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta CDX supera el máximo de filas",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut url_hints = BTreeSet::new();
    for row in rows.iter().skip(1) {
        let Some(value) = row.as_array().and_then(|columns| columns.first()) else {
            continue;
        };
        let Some(value) = value.as_str() else {
            continue;
        };
        if value.len() > MAX_URL_CHARS {
            continue;
        }
        let Ok(mut url) = Url::parse(value) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        let Some(host) = url.host_str().and_then(canonical_dns_name) else {
            continue;
        };
        if host != query_domain && !host.ends_with(&suffix) {
            continue;
        }
        url.set_query(None);
        url.set_fragment(None);
        url_hints.insert(url.to_string());
        if url_hints.len() > MAX_URL_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta CDX supera el máximo de URL históricas",
            ));
        }
    }

    Ok(ArchivedWaybackCdxHints {
        query_domain,
        url_hints: url_hints.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_wayback_cdx;
    use crate::ErrorCode;

    #[test]
    fn extracts_in_scope_archived_urls_without_query_values() {
        let response = br#"[["original","timestamp"],["https://example.com/login?token=secret#form","20250101"],["http://api.example.com/v1","20240101"],["https://evil-example.com/path","20240101"],["javascript:alert(1)","20240101"]]"#;
        let result = parse_archived_wayback_cdx("EXAMPLE.COM.", response).unwrap();
        assert_eq!(
            result.url_hints,
            vec!["http://api.example.com/v1", "https://example.com/login"]
        );
    }

    #[test]
    fn rejects_invalid_scope_and_json_shapes() {
        assert_eq!(
            parse_archived_wayback_cdx("not a domain", b"[]")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_wayback_cdx("example.com", br#"{"rows":[]}"#)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_row_limits() {
        let oversized = vec![b'x'; 2 * 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_wayback_cdx("example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let rows = vec![vec!["https://example.com/a", "20240101"]; 4098];
        let response = serde_json::to_vec(&rows).unwrap();
        assert_eq!(
            parse_archived_wayback_cdx("example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
