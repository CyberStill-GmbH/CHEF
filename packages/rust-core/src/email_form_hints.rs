//! Resume presencia de controles HTML tipo email con atributo placeholder.
//!
//! Solo se conserva un conteo descriptivo del contenido archivado. No devuelve
//! el texto del placeholder, URL de formulario ni genera hallazgos o acciones.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_TAGS: usize = 8192;
const MAX_EMAIL_INPUTS: usize = 512;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ArchivedEmailFormHints {
    pub email_inputs_with_placeholder: u16,
}

/// Cuenta `<input>` con `type=email` y atributo `placeholder` en HTML archivado.
pub fn parse_archived_email_form_hints(html: &str) -> ChefResult<ArchivedEmailFormHints> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML archivado supera el límite de controles email",
        ));
    }
    let bytes = html.as_bytes();
    let mut cursor = 0;
    let mut tags_seen = 0;
    let mut count = 0usize;
    while let Some(open) = find_byte(bytes, b'<', cursor) {
        if bytes.get(open..open + 4) == Some(b"<!--") {
            let Some(comment_end) = find_bytes(bytes, b"-->", open + 4) else {
                break;
            };
            cursor = comment_end + 3;
            continue;
        }
        let name_start = open + 1;
        if bytes.get(name_start) == Some(&b'/')
            || bytes
                .get(name_start)
                .is_none_or(|byte| !byte.is_ascii_alphabetic())
        {
            cursor = open + 1;
            continue;
        }
        tags_seen += 1;
        if tags_seen > MAX_TAGS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML supera el máximo de etiquetas CHEF",
            ));
        }
        let Some(name_end) = scan_name(bytes, name_start) else {
            cursor = open + 1;
            continue;
        };
        let Some(tag_end) = find_tag_end(bytes, name_end) else {
            break;
        };
        if eq_ascii_case_insensitive(&bytes[name_start..name_end], b"script")
            || eq_ascii_case_insensitive(&bytes[name_start..name_end], b"style")
        {
            let closing = if eq_ascii_case_insensitive(&bytes[name_start..name_end], b"script") {
                b"</script".as_slice()
            } else {
                b"</style".as_slice()
            };
            cursor =
                find_ascii_case_insensitive(bytes, closing, tag_end + 1).unwrap_or(bytes.len());
            continue;
        }
        if eq_ascii_case_insensitive(&bytes[name_start..name_end], b"input")
            && has_email_type_and_placeholder(&bytes[name_end..tag_end])
        {
            count += 1;
            if count > MAX_EMAIL_INPUTS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El HTML supera el máximo de controles email CHEF",
                ));
            }
        }
        cursor = tag_end + 1;
    }
    Ok(ArchivedEmailFormHints {
        email_inputs_with_placeholder: count as u16,
    })
}

fn has_email_type_and_placeholder(attributes: &[u8]) -> bool {
    let mut cursor = 0;
    let mut is_email = false;
    let mut has_placeholder = false;
    while cursor < attributes.len() {
        skip_space(attributes, &mut cursor);
        if cursor >= attributes.len() || attributes[cursor] == b'/' {
            break;
        }
        let start = cursor;
        while cursor < attributes.len()
            && !attributes[cursor].is_ascii_whitespace()
            && !matches!(attributes[cursor], b'=' | b'/' | b'>')
        {
            cursor += 1;
        }
        if cursor == start {
            cursor += 1;
            continue;
        }
        let name = &attributes[start..cursor];
        skip_space(attributes, &mut cursor);
        let value = if attributes.get(cursor) == Some(&b'=') {
            cursor += 1;
            skip_space(attributes, &mut cursor);
            parse_attribute_value(attributes, &mut cursor)
        } else {
            None
        };
        if eq_ascii_case_insensitive(name, b"type") {
            is_email = value.is_some_and(|value| eq_ascii_case_insensitive(value, b"email"));
        }
        if eq_ascii_case_insensitive(name, b"placeholder") {
            has_placeholder = true;
        }
    }
    is_email && has_placeholder
}

fn parse_attribute_value<'a>(attributes: &'a [u8], cursor: &mut usize) -> Option<&'a [u8]> {
    let first = *attributes.get(*cursor)?;
    if matches!(first, b'\'' | b'"') {
        *cursor += 1;
        let start = *cursor;
        let end = attributes[start..].iter().position(|byte| *byte == first)? + start;
        *cursor = end + 1;
        Some(&attributes[start..end])
    } else {
        let start = *cursor;
        while *cursor < attributes.len()
            && !attributes[*cursor].is_ascii_whitespace()
            && attributes[*cursor] != b'>'
        {
            *cursor += 1;
        }
        Some(&attributes[start..*cursor])
    }
}

fn scan_name(bytes: &[u8], start: usize) -> Option<usize> {
    let mut end = start;
    while bytes
        .get(end)
        .is_some_and(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b':'))
    {
        end += 1;
    }
    (end > start).then_some(end)
}

fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, byte) in bytes.get(start..)?.iter().enumerate() {
        match (*byte, quote) {
            (b'\'' | b'"', None) => quote = Some(*byte),
            (current, Some(expected)) if current == expected => quote = None,
            (b'>', None) => return Some(start + offset),
            _ => {}
        }
    }
    None
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
}

fn find_byte(bytes: &[u8], byte: u8, from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .iter()
        .position(|current| *current == byte)
        .map(|offset| from + offset)
}

fn find_bytes(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window == needle)
        .map(|offset| from + offset)
}

fn find_ascii_case_insensitive(bytes: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| eq_ascii_case_insensitive(window, needle))
        .map(|offset| from + offset)
}

fn eq_ascii_case_insensitive(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_email_form_hints;
    use crate::ErrorCode;

    #[test]
    fn counts_email_controls_without_retaining_form_content() {
        let html = r#"<form><input TYPE='email' placeholder='Your email'><input type=email placeholder=""></form>"#;
        let result = parse_archived_email_form_hints(html).unwrap();
        assert_eq!(result.email_inputs_with_placeholder, 2);
    }

    #[test]
    fn ignores_non_email_controls_comments_and_script_text() {
        let html = r#"<!-- <input type=email placeholder=x> --><script>const x = '<input type=email placeholder=x>';</script><input type=text placeholder=x><input type=email>"#;
        assert_eq!(
            parse_archived_email_form_hints(html)
                .unwrap()
                .email_inputs_with_placeholder,
            0
        );
    }

    #[test]
    fn enforces_input_tag_count_and_malformed_comment_handling() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_email_form_hints(&oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let too_many_tags = "<div></div>".repeat(8193);
        assert_eq!(
            parse_archived_email_form_hints(&too_many_tags)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_email_form_hints("<!-- <input type=email placeholder=x>")
                .unwrap()
                .email_inputs_with_placeholder,
            0
        );
    }
}
