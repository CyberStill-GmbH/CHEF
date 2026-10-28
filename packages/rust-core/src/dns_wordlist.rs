//! Parser CHEF para listas DNS entregadas por el adaptador.
//!
//! Conserva únicamente prefijos ASCII válidos y estables. No lee archivos,
//! descarga listas ni consulta DNS; el llamador controla el origen del texto.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_WORDLIST_BYTES: usize = 2 * 1024 * 1024;
const MAX_WORDLIST_LINES: usize = 50_000;
const MAX_WORDLIST_ENTRIES: usize = 10_000;
const MAX_DNS_NAME_BYTES: usize = 253;

/// Contenido aceptado y recuentos de líneas descartadas al interpretar una lista.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWordlist {
    /// Entradas canónicas ordenadas y deduplicadas.
    pub entries: Vec<String>,
    /// Huella del texto original; la lista no se conserva en el grafo.
    pub content_sha256: String,
    pub duplicate_entries: usize,
    pub comment_lines: usize,
    pub blank_lines: usize,
    pub rejected_entries: usize,
}

/// Interpreta texto local como prefijos DNS, con límites antes de procesarlo.
///
/// Acepta una etiqueta o una secuencia de etiquetas separadas por puntos. Las
/// líneas vacías y comentarios `#` se omiten; `#` inicia comentario inline si
/// está precedido por espacio en blanco. No se aceptan wildcards ni nombres
/// Unicode porque este parser alimenta exclusivamente candidatos de host.
pub fn parse_dns_wordlist(input: &str) -> ChefResult<DnsWordlist> {
    if input.len() > MAX_WORDLIST_BYTES {
        return Err(limit_error("La lista DNS supera el máximo CHEF de bytes"));
    }

    let mut result = DnsWordlist {
        entries: Vec::new(),
        content_sha256: crate::identity::digest(input.as_bytes()),
        duplicate_entries: 0,
        comment_lines: 0,
        blank_lines: 0,
        rejected_entries: 0,
    };
    let mut entries = BTreeSet::new();

    for (line_index, line) in input.lines().enumerate() {
        if line_index >= MAX_WORDLIST_LINES {
            return Err(limit_error("La lista DNS excede el máximo CHEF de líneas"));
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            result.blank_lines += 1;
            continue;
        }
        if trimmed.starts_with('#') {
            result.comment_lines += 1;
            continue;
        }

        let entry = strip_inline_comment(trimmed).trim();
        if entry.is_empty() {
            result.comment_lines += 1;
            continue;
        }
        let Some(canonical) = canonical_prefix(entry) else {
            result.rejected_entries += 1;
            continue;
        };
        if entries.insert(canonical) {
            if entries.len() > MAX_WORDLIST_ENTRIES {
                return Err(limit_error(
                    "La lista DNS excede el máximo CHEF de entradas",
                ));
            }
        } else {
            result.duplicate_entries += 1;
        }
    }

    result.entries = entries.into_iter().collect();
    Ok(result)
}

fn strip_inline_comment(line: &str) -> &str {
    line.char_indices()
        .find(|(index, character)| {
            *character == '#'
                && *index > 0
                && line[..*index]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace)
        })
        .map_or(line, |(index, _)| &line[..index])
}

fn canonical_prefix(value: &str) -> Option<String> {
    if value.is_empty() || value.len() > MAX_DNS_NAME_BYTES || !value.is_ascii() {
        return None;
    }
    for label in value.split('.') {
        if label.is_empty()
            || label.len() > 63
            || label.starts_with('-')
            || label.ends_with('-')
            || !label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return None;
        }
    }
    Some(value.to_ascii_lowercase())
}

fn limit_error(message: &'static str) -> ChefError {
    ChefError::new(ErrorCode::LimitExceeded, message)
}
