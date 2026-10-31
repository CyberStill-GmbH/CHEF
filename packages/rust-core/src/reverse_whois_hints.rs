//! Extrae dominios relacionados de una respuesta HTML reverse-WHOIS archivada.
//!
//! Una fila válida solo es una referencia histórica del proveedor; no prueba
//! propiedad, control ni relación corporativa actual.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_ROWS: usize = 4096;
const MAX_CANDIDATES: usize = 512;
const MAX_CELL_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedReverseWhoisHints {
    pub query_domain: String,
    pub candidate_domains: Vec<String>,
}

/// Analiza filas HTML archivadas que contienen dominio y fecha de registro.
///
/// Se limita a filas con al menos dos celdas y una fecha `YYYY-MM-DD` válida;
/// el dominio consultado se excluye y los candidatos se ordenan y deduplican.
pub fn parse_archived_reverse_whois(
    html: &str,
    query_domain: &str,
) -> ChefResult<ArchivedReverseWhoisHints> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El HTML reverse-WHOIS archivado supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El dominio reverse-WHOIS no es válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta reverse-WHOIS debe ser una raíz registrable ICANN",
        ));
    }

    let mut candidates = BTreeSet::new();
    let mut cursor = 0;
    let mut rows_seen = 0;
    while let Some((_, open_end)) = find_open_tag(html, "tr", cursor) {
        let Some((row_end, close_end)) = find_close_tag(html, "tr", open_end) else {
            break;
        };
        rows_seen += 1;
        if rows_seen > MAX_ROWS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML reverse-WHOIS supera el máximo de filas",
            ));
        }
        cursor = close_end;
        let cells = parse_cells(&html[open_end..row_end])?;
        if cells.len() < 2 || !has_valid_date_prefix(&cells[1]) {
            continue;
        }
        let Some(domain) = canonical_dns_name(&cells[0]) else {
            continue;
        };
        if domain == query_domain {
            continue;
        }
        candidates.insert(domain);
        if candidates.len() > MAX_CANDIDATES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El HTML reverse-WHOIS supera el máximo de dominios candidatos",
            ));
        }
    }

    Ok(ArchivedReverseWhoisHints {
        query_domain,
        candidate_domains: candidates.into_iter().collect(),
    })
}

fn parse_cells(row: &str) -> ChefResult<Vec<String>> {
    let mut cells = Vec::new();
    let mut cursor = 0;
    while let Some((_, open_end)) = find_open_tag(row, "td", cursor) {
        let Some((cell_end, close_end)) = find_close_tag(row, "td", open_end) else {
            break;
        };
        let value = strip_tags(&row[open_end..cell_end]);
        if value.len() > MAX_CELL_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Una celda reverse-WHOIS supera el límite de bytes",
            ));
        }
        cells.push(decode_common_entities(&value).trim().to_owned());
        cursor = close_end;
        if cells.len() > 16 {
            break;
        }
    }
    Ok(cells)
}

fn find_open_tag(value: &str, tag: &str, from: usize) -> Option<(usize, usize)> {
    let needle = format!("<{tag}");
    let mut cursor = from;
    while let Some(start) = find_ascii_case_insensitive(value, &needle, cursor) {
        let after_name = start + needle.len();
        let next = value.as_bytes().get(after_name).copied()?;
        if next == b'>' || next.is_ascii_whitespace() {
            let open_end = value[after_name..].find('>')? + after_name + 1;
            return Some((start, open_end));
        }
        cursor = after_name;
    }
    None
}

fn find_close_tag(value: &str, tag: &str, from: usize) -> Option<(usize, usize)> {
    let needle = format!("</{tag}");
    let start = find_ascii_case_insensitive(value, &needle, from)?;
    let close_end = value[start..].find('>')? + start + 1;
    Some((start, close_end))
}

fn find_ascii_case_insensitive(value: &str, needle: &str, from: usize) -> Option<usize> {
    let bytes = value.as_bytes();
    let needle = needle.as_bytes();
    bytes
        .get(from..)?
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|relative| from + relative)
}

fn strip_tags(value: &str) -> String {
    let mut text = String::with_capacity(value.len());
    let mut inside_tag = false;
    for character in value.chars() {
        match character {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                text.push(' ');
            }
            _ if !inside_tag => text.push(character),
            _ => {}
        }
    }
    text
}

fn decode_common_entities(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

fn has_valid_date_prefix(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && (1..=12).contains(&value[5..7].parse::<u8>().unwrap_or_default())
        && (1..=31).contains(&value[8..10].parse::<u8>().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::parse_archived_reverse_whois;
    use crate::ErrorCode;

    #[test]
    fn extracts_domains_only_from_dated_archive_rows() {
        let html = r#"<table><tr><th>Domain</th><th>Date</th></tr><tr><td>Example.com</td><td>2024-05-20</td><td>Registrar</td></tr><tr><td><b>partner.net</b></td><td>2023-11-01</td><td>R &amp; R</td></tr><tr><td>example.com</td><td>2020-01-01</td></tr><tr><td>ignored.org</td><td>not-a-date</td></tr></table>"#;
        let result = parse_archived_reverse_whois(html, "example.com").unwrap();
        assert_eq!(result.candidate_domains, vec!["partner.net"]);
    }

    #[test]
    fn rejects_invalid_scope_and_ignores_malformed_rows() {
        let result = parse_archived_reverse_whois(
            "<tr><td>bad_domain</td><td>2024-01-01</td></tr><tr><td>unfinished",
            "example.com",
        )
        .unwrap();
        assert!(result.candidate_domains.is_empty());
        assert_eq!(
            parse_archived_reverse_whois("", "https://example.com")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_html_and_row_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_reverse_whois(&oversized, "example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let many_rows = "<tr><td>x</td></tr>".repeat(4097);
        assert_eq!(
            parse_archived_reverse_whois(&many_rows, "example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
