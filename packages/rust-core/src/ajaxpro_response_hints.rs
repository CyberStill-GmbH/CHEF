//! Extrae rutas con forma AJAXPro desde HTML archivado.
//!
//! La ruta es una pista observada en la captura, no una comprobación de
//! instalación o vulnerabilidad. No se hacen solicitudes ni se envían payloads.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_SCRIPT_TAGS: usize = 4096;
const MAX_PATH_HINTS: usize = 128;
const MAX_PATH_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedAjaxProPathHints {
    pub script_paths: Vec<String>,
}

/// Busca en etiquetas `<script>` rutas de `src` con terminación `.ashx` y
/// exactamente una coma, como el patrón observado por el módulo fuente.
pub fn parse_archived_ajaxpro_html(html: &str) -> ChefResult<ArchivedAjaxProPathHints> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML archivado supera el límite del parser AJAXPro",
        ));
    }
    let bytes = html.as_bytes();
    let mut cursor = 0;
    let mut script_tags = 0;
    let mut paths = BTreeSet::new();
    while let Some(start) = find_ascii_case_insensitive(bytes, b"<script", cursor) {
        script_tags += 1;
        if script_tags > MAX_SCRIPT_TAGS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML supera el máximo de etiquetas script",
            ));
        }
        let tag_start = start + b"<script".len();
        let Some(relative_end) = bytes[tag_start..].iter().position(|byte| *byte == b'>') else {
            break;
        };
        let tag_end = tag_start + relative_end;
        if let Some(path) = script_src_path(&bytes[tag_start..tag_end]) {
            if let Ok(path) = std::str::from_utf8(path) {
                paths.insert(path.to_owned());
                if paths.len() > MAX_PATH_HINTS {
                    return Err(ChefError::new(
                        ErrorCode::LimitExceeded,
                        "El HTML supera el máximo de rutas AJAXPro",
                    ));
                }
            }
        }
        cursor = tag_end + 1;
    }
    Ok(ArchivedAjaxProPathHints {
        script_paths: paths.into_iter().collect(),
    })
}

fn script_src_path(tag: &[u8]) -> Option<&[u8]> {
    let name = b"src=\"";
    let attribute_start = find_ascii_case_insensitive(tag, name, 0)? + name.len();
    let attribute_end = tag[attribute_start..]
        .iter()
        .position(|byte| *byte == b'"')?
        + attribute_start;
    let path = &tag[attribute_start..attribute_end];
    let comma = path.iter().position(|byte| *byte == b',')?;
    if path.len() < 6
        || path.len() > MAX_PATH_BYTES
        || !path
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b','))
        || path.iter().filter(|byte| **byte == b',').count() != 1
        || comma == 0
        || comma + 1 >= path.len() - 5
        || path.windows(5).last()? != b".ashx"
        || path.starts_with(b"//")
    {
        return None;
    }
    Some(path)
}

fn find_ascii_case_insensitive(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    if needle.is_empty()
        || from > haystack.len()
        || needle.len() > haystack.len().saturating_sub(from)
    {
        return None;
    }
    haystack[from..]
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|offset| from + offset)
}

#[cfg(test)]
mod tests {
    use super::parse_archived_ajaxpro_html;
    use crate::ErrorCode;

    #[test]
    fn extracts_ajaxpro_shaped_script_paths_from_archived_html() {
        let html = r#"<SCRIPT SRC="/ajaxpro/Foo,Bar.ashx"></SCRIPT><script src="/ajaxpro/Foo,Bar.ashx"></script>"#;
        let result = parse_archived_ajaxpro_html(html).unwrap();
        assert_eq!(result.script_paths, ["/ajaxpro/Foo,Bar.ashx"]);
    }

    #[test]
    fn ignores_non_script_paths_and_malformed_or_unrelated_sources() {
        let html = r#"<a href="/ajaxpro/Foo,Bar.ashx">link</a><script src="https://example.com/Foo,Bar.ashx"></script><script src="/AjaxPro/Foo.ashx"></script><script src="/x/Foo,Bar.dll"></script>"#;
        assert!(
            parse_archived_ajaxpro_html(html)
                .unwrap()
                .script_paths
                .is_empty()
        );
    }

    #[test]
    fn enforces_html_tag_and_path_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_ajaxpro_html(&oversized).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let too_many_tags = "<script></script>".repeat(4097);
        assert_eq!(
            parse_archived_ajaxpro_html(&too_many_tags)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_paths = (0..129)
            .map(|index| format!("<script src=\"/a/{index},b.ashx\"></script>"))
            .collect::<String>();
        assert_eq!(
            parse_archived_ajaxpro_html(&too_many_paths)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
