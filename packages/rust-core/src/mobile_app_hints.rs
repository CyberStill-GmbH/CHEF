//! Extrae identificadores candidatos de enlaces Play Store archivados.
//!
//! El resultado identifica texto presente en una captura; no confirma que la
//! aplicación exista, esté publicada o pertenezca a una organización concreta.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_LINK_MARKERS: usize = 4096;
const MAX_APP_IDS: usize = 512;
const MAX_APP_ID_BYTES: usize = 256;
const PLAY_STORE_MARKER: &str = "/store/apps/details?id=";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedMobileAppHints {
    pub app_ids: Vec<String>,
}

/// Extrae valores de enlaces Play Store de una captura HTML ya archivada.
///
/// Replica el patrón de enlace del módulo fuente con límites y deduplicación,
/// sin visitar la tienda ni verificar los paquetes encontrados.
pub fn parse_archived_mobile_app_ids(html: &str) -> ChefResult<ArchivedMobileAppHints> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML de aplicaciones archivado supera el límite de bytes",
        ));
    }
    let bytes = html.as_bytes();
    let marker = PLAY_STORE_MARKER.as_bytes();
    let mut cursor = 0;
    let mut markers_seen = 0;
    let mut app_ids = BTreeSet::new();
    while let Some(relative) = bytes.get(cursor..).and_then(|tail| {
        tail.windows(marker.len())
            .position(|window| window == marker)
    }) {
        let start = cursor + relative + marker.len();
        cursor = start;
        markers_seen += 1;
        if markers_seen > MAX_LINK_MARKERS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La captura supera el máximo de enlaces Play Store",
            ));
        }
        let mut end = start;
        while end < bytes.len()
            && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'.' | b'_' | b'-'))
        {
            end += 1;
        }
        let candidate = &html[start..end];
        if valid_candidate_id(candidate) {
            app_ids.insert(candidate.to_owned());
            if app_ids.len() > MAX_APP_IDS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La captura supera el máximo CHEF de identificadores de aplicación",
                ));
            }
        }
    }
    Ok(ArchivedMobileAppHints {
        app_ids: app_ids.into_iter().collect(),
    })
}

fn valid_candidate_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_APP_ID_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
}

#[cfg(test)]
mod tests {
    use super::{PLAY_STORE_MARKER, parse_archived_mobile_app_ids};
    use crate::ErrorCode;

    #[test]
    fn extracts_and_deduplicates_archived_store_links() {
        let html = r#"<a href="/store/apps/details?id=com.example.app">App</a><a href="/store/apps/details?id=com.example.app&amp;hl=en">Again</a><a href="/store/apps/details?id=org.sample-tool">Tool</a>"#;
        let result = parse_archived_mobile_app_ids(html).unwrap();
        assert_eq!(result.app_ids, vec!["com.example.app", "org.sample-tool"]);
    }

    #[test]
    fn ignores_malformed_ids_and_unrelated_paths() {
        let html = r#"/store/apps/details?id=.bad /store/apps/details?id=bad..id /store/apps/other?id=com.example.ok"#;
        let result = parse_archived_mobile_app_ids(html).unwrap();
        assert!(result.app_ids.is_empty());
    }

    #[test]
    fn enforces_html_link_and_result_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_mobile_app_ids(&oversized).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let too_many_links = PLAY_STORE_MARKER.repeat(4097);
        assert_eq!(
            parse_archived_mobile_app_ids(&too_many_links)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let many_ids = (0..513)
            .map(|index| format!("{PLAY_STORE_MARKER}com.example.app{index}"))
            .collect::<String>();
        assert_eq!(
            parse_archived_mobile_app_ids(&many_ids).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
