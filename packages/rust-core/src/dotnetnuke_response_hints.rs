//! Detecta firmas descriptivas de DNN en una respuesta HTTP archivada.
//!
//! Un marcador histórico no demuestra versión, exposición ni vulnerabilidad.
//! El parser no hace OAST, solicitudes ni evaluaciones de seguridad activas.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_HEADER_BYTES: usize = 64 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DotNetNukeResponseMarker {
    CacheHeader,
    CompressionHeader,
    BrandComment,
    PlatformComment,
    CoreScriptReference,
    ProductMetaValue,
    ContentPaneIdentifier,
    ModuleClass,
    InstallWizardPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArchivedDotNetNukeResponseHints {
    pub markers: Vec<DotNetNukeResponseMarker>,
}

/// Analiza headers y body archivados para encontrar firmas locales de DNN.
pub fn parse_archived_dotnetnuke_response(
    headers: &str,
    body: &str,
) -> ChefResult<ArchivedDotNetNukeResponseHints> {
    if headers.len() > MAX_HEADER_BYTES || body.len() > MAX_BODY_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta archivada supera los límites del parser DNN",
        ));
    }
    let mut markers = Vec::new();
    if header_has_name(headers, "dnnoutputcache") {
        markers.push(DotNetNukeResponseMarker::CacheHeader);
    }
    if header_has_value(headers, "x-compressed-by", "dotnetnuke") {
        markers.push(DotNetNukeResponseMarker::CompressionHeader);
    }
    for (needle, marker) in [
        (
            "<!-- by dotnetnuke corporation",
            DotNetNukeResponseMarker::BrandComment,
        ),
        (
            "<!-- dnn platform",
            DotNetNukeResponseMarker::PlatformComment,
        ),
        (
            "/js/dnncore.js",
            DotNetNukeResponseMarker::CoreScriptReference,
        ),
        (
            "content=\",dotnetnuke,dnn",
            DotNetNukeResponseMarker::ProductMetaValue,
        ),
        (
            "dnn_contentpane",
            DotNetNukeResponseMarker::ContentPaneIdentifier,
        ),
        ("class=\"dnnmodule\"", DotNetNukeResponseMarker::ModuleClass),
        (
            "/install/installwizard.aspx",
            DotNetNukeResponseMarker::InstallWizardPath,
        ),
    ] {
        if contains_ascii_case_insensitive(body, needle) {
            markers.push(marker);
        }
    }
    Ok(ArchivedDotNetNukeResponseHints { markers })
}

fn header_has_name(headers: &str, expected_name: &str) -> bool {
    headers.lines().any(|line| {
        line.split_once(':')
            .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case(expected_name))
    })
}

fn header_has_value(headers: &str, expected_name: &str, expected_value: &str) -> bool {
    headers.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.trim().eq_ignore_ascii_case(expected_name)
                && value.trim().eq_ignore_ascii_case(expected_value)
        })
    })
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::{DotNetNukeResponseMarker, parse_archived_dotnetnuke_response};
    use crate::ErrorCode;

    #[test]
    fn reports_archived_platform_markers_as_hints_only() {
        let headers = "HTTP/1.1 200 OK\r\nDNNOutputCache: 1\r\nX-Compressed-By: DotNetNuke\r\n";
        let body =
            "<!-- DNN Platform --><script src='/js/dnncore.js'></script><div class=\"DnnModule\">";
        let result = parse_archived_dotnetnuke_response(headers, body).unwrap();
        assert!(
            result
                .markers
                .contains(&DotNetNukeResponseMarker::CacheHeader)
        );
        assert!(
            result
                .markers
                .contains(&DotNetNukeResponseMarker::CompressionHeader)
        );
        assert!(
            result
                .markers
                .contains(&DotNetNukeResponseMarker::PlatformComment)
        );
        assert!(
            result
                .markers
                .contains(&DotNetNukeResponseMarker::CoreScriptReference)
        );
        assert!(
            result
                .markers
                .contains(&DotNetNukeResponseMarker::ModuleClass)
        );
    }

    #[test]
    fn rejects_lookalike_headers_and_unrelated_html() {
        let headers = "X-DNNOutputCache: 1\r\nX-Compressed-By: DotNetNukeish\r\n";
        let body = "<!-- unrelated DNN platform --><div class=\"DnnModuleExtra\">";
        assert!(
            parse_archived_dotnetnuke_response(headers, body)
                .unwrap()
                .markers
                .is_empty()
        );
    }

    #[test]
    fn enforces_header_and_body_limits() {
        let long_headers = "x".repeat(64 * 1024 + 1);
        assert_eq!(
            parse_archived_dotnetnuke_response(&long_headers, "")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let long_body = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_dotnetnuke_response("", &long_body)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
