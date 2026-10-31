//! Extrae enlaces internos de un índice HTML archivado bajo `.git/`.
//!
//! Adapta la primera etapa de `modules/gitdumper.py`: identifica un índice y
//! devuelve referencias locales como pistas. No recorre enlaces, descarga
//! archivos, reconstruye el repositorio ni toca el filesystem.

use std::collections::BTreeMap;

use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, url_tools::canonical_http_url};

const MAX_HTML_BYTES: usize = 1024 * 1024;
const MAX_URL_BYTES: usize = 8 * 1024;
const MAX_ANCHORS: usize = 4096;
const MAX_ENTRIES: usize = 512;
const MAX_TAG_BYTES: usize = 8 * 1024;
const MAX_HREF_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitDirectoryEntryHint {
    pub url: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGitDirectoryListing {
    pub directory_url: String,
    pub entries: Vec<GitDirectoryEntryHint>,
}

/// Interpreta un índice Apache archivado dentro de un directorio `.git/`.
///
/// `Ok(None)` indica que el cuerpo no tiene el título esperado. Las entradas
/// se limitan al mismo origen y a descendientes de la ruta base; las query y
/// los fragmentos se descartan. Las referencias no prueban que el contenido
/// siga disponible ni autorizan su descarga.
pub fn parse_archived_git_directory_listing(
    directory_url: &str,
    html: &[u8],
) -> ChefResult<Option<ArchivedGitDirectoryListing>> {
    if html.len() > MAX_HTML_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El índice Git archivado supera el límite de bytes",
        ));
    }
    if directory_url.len() > MAX_URL_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La URL del índice Git supera el límite de bytes",
        ));
    }
    let mut base = parse_http_url(directory_url)?;
    let content = std::str::from_utf8(html).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El índice Git archivado no es texto UTF-8 válido",
        )
    })?;
    if !content.to_ascii_lowercase().contains("<title>index of") {
        return Ok(None);
    }

    let mut base_path = base.path().to_owned();
    if !base_path.ends_with('/') {
        base_path.push('/');
        base.set_path(&base_path);
    }
    if !base_path.to_ascii_lowercase().ends_with("/.git/") {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La URL del índice debe estar dentro de un directorio .git/",
        ));
    }
    let origin = base.origin();
    let directory_url = base.to_string();
    let mut entries = BTreeMap::new();

    for href in archived_anchor_hrefs(content)? {
        if href.is_empty()
            || href.len() > MAX_HREF_BYTES
            || href.chars().any(char::is_control)
            || href.contains('\\')
        {
            continue;
        }
        let Ok(mut resolved) = base.join(href) else {
            continue;
        };
        if resolved.origin() != origin
            || !matches!(resolved.scheme(), "http" | "https")
            || !resolved.username().is_empty()
            || resolved.password().is_some()
            || !resolved.path().starts_with(&base_path)
            || resolved.path() == base_path
        {
            continue;
        }
        let is_directory = href.ends_with('/') || resolved.path().ends_with('/');
        resolved.set_query(None);
        resolved.set_fragment(None);
        let url = resolved.to_string();
        entries
            .entry(url.clone())
            .or_insert(GitDirectoryEntryHint { url, is_directory });
        if entries.len() > MAX_ENTRIES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El índice Git supera el máximo CHEF de referencias",
            ));
        }
    }

    Ok(Some(ArchivedGitDirectoryListing {
        directory_url,
        entries: entries.into_values().collect(),
    }))
}

fn parse_http_url(value: &str) -> ChefResult<Url> {
    let canonical = canonical_http_url(value).ok_or_else(|| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La URL debe ser HTTP(S) válida y no contener credenciales",
        )
    })?;
    Url::parse(&canonical)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La URL archivada no es válida"))
}

fn archived_anchor_hrefs(html: &str) -> ChefResult<Vec<&str>> {
    let lowered = html.to_ascii_lowercase();
    let bytes = html.as_bytes();
    let mut cursor = 0;
    let mut hrefs = Vec::new();
    while let Some(relative_start) = lowered[cursor..].find("<a") {
        let start = cursor + relative_start;
        let tag_name_end = start + 2;
        if bytes
            .get(tag_name_end)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'/' | b'>'))
        {
            cursor = tag_name_end;
            continue;
        }
        let Some(end) = find_tag_end(bytes, start) else {
            break;
        };
        if end - start + 1 > MAX_TAG_BYTES {
            cursor = end + 1;
            continue;
        }
        let tag = &html[start..=end];
        if let Some(href) = find_href_attribute(tag) {
            if hrefs.len() == MAX_ANCHORS {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El índice Git supera el máximo de enlaces",
                ));
            }
            hrefs.push(href);
        }
        cursor = end + 1;
    }
    Ok(hrefs)
}

fn find_tag_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut quote = None;
    for (offset, byte) in bytes.iter().copied().enumerate().skip(start + 2) {
        match (quote, byte) {
            (Some(current), value) if current == value => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Some(offset),
            _ => {}
        }
        if offset - start >= MAX_TAG_BYTES {
            return None;
        }
    }
    None
}

fn find_href_attribute(tag: &str) -> Option<&str> {
    let bytes = tag.as_bytes();
    let mut cursor = 2;
    while cursor < bytes.len() {
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace() || *byte == b'/')
        {
            cursor += 1;
        }
        if cursor >= bytes.len() || bytes[cursor] == b'>' {
            break;
        }
        let name_start = cursor;
        while bytes
            .get(cursor)
            .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'=' | b'/' | b'>'))
        {
            cursor += 1;
        }
        if cursor == name_start {
            cursor += 1;
            continue;
        }
        let is_href = bytes[name_start..cursor].eq_ignore_ascii_case(b"href");
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        if bytes.get(cursor) != Some(&b'=') {
            continue;
        }
        cursor += 1;
        while bytes
            .get(cursor)
            .is_some_and(|byte| byte.is_ascii_whitespace())
        {
            cursor += 1;
        }
        let first = bytes.get(cursor).copied()?;
        let (value_start, value_end) = if matches!(first, b'\'' | b'"') {
            cursor += 1;
            let value_start = cursor;
            while bytes.get(cursor).is_some_and(|byte| *byte != first) {
                cursor += 1;
            }
            if bytes.get(cursor) != Some(&first) {
                return None;
            }
            (value_start, cursor)
        } else {
            let value_start = cursor;
            while bytes
                .get(cursor)
                .is_some_and(|byte| !byte.is_ascii_whitespace() && !matches!(byte, b'>'))
            {
                cursor += 1;
            }
            (value_start, cursor)
        };
        if is_href {
            return tag.get(value_start..value_end);
        }
        cursor = value_end + usize::from(matches!(first, b'\'' | b'"'));
    }
    None
}
