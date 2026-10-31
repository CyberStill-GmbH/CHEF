//! Extrae candidatos de archivo desde un listado S3 archivado.
//!
//! Este adaptador no consulta el bucket, descarga objetos, confirma que sigan
//! disponibles ni modifica el scope. La procedencia del XML debe conservarse
//! en el evento de importación que consuma estas pistas.

use roxmltree::{Document, Node, ParsingOptions};
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_LISTING_BYTES: usize = 1024 * 1024;
const MAX_LISTING_RECORDS: usize = 4_096;
const MAX_KEY_BYTES: usize = 1_024;
const MAX_HINTS: usize = 512;
const S3_NAMESPACE: &str = "http://s3.amazonaws.com/doc/2006-03-01/";

/// Candidato histórico derivado de un elemento `Contents/Key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedBucketFileHint {
    /// Origen HTTPS del bucket; nunca contiene query, fragmento ni credenciales.
    pub bucket_origin: String,
    /// Clave S3 exacta decodificada del XML.
    pub key: String,
    /// URL candidata en el mismo origen del bucket.
    pub url: String,
    /// Extensión en minúsculas, sin punto, si existe.
    pub extension: Option<String>,
    /// Ubicación del campo en el XML archivado.
    pub source_locator: String,
}

/// Normaliza un listado S3 XML archivado en un máximo de `file_limit` pistas.
///
/// Rechaza listados malformados en su totalidad; nunca devuelve resultados
/// parciales. Solo admite orígenes HTTPS de AWS S3 o DigitalOcean Spaces y
/// conserva los objetos bajo el mismo origen.
pub fn parse_archived_bucket_file_listing(
    bucket_url: &str,
    response: &[u8],
    file_limit: usize,
) -> ChefResult<Vec<ArchivedBucketFileHint>> {
    if response.len() > MAX_LISTING_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El listado de objetos archivado supera el límite de bytes CHEF",
        ));
    }
    if !(1..=MAX_HINTS).contains(&file_limit) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El límite de pistas de bucket debe estar entre 1 y 512",
        ));
    }

    let bucket_origin = parse_bucket_origin(bucket_url)?;
    let xml = std::str::from_utf8(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El listado S3 archivado debe ser UTF-8 válido",
        )
    })?;
    reject_dtd_and_entities(xml)?;
    check_xml_depth(xml)?;
    let document = Document::parse_with_options(
        xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: (MAX_LISTING_RECORDS * 24 + 256) as u32,
        },
    )
    .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "El listado S3 XML no es válido"))?;

    let root = document.root_element();
    if root.tag_name().name() != "ListBucketResult"
        || root.tag_name().namespace() != Some(S3_NAMESPACE)
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Se esperaba un ListBucketResult S3",
        ));
    }

    let contents = root
        .children()
        .filter(|node| {
            node.is_element()
                && node.tag_name().name() == "Contents"
                && node.tag_name().namespace() == Some(S3_NAMESPACE)
        })
        .collect::<Vec<_>>();
    if contents.len() > MAX_LISTING_RECORDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El listado S3 excede el máximo de objetos CHEF",
        ));
    }

    let mut hints = Vec::with_capacity(file_limit.min(contents.len()));
    for (index, content) in contents.into_iter().enumerate() {
        let mut key_nodes = s3_children(content, "Key");
        let Some(key_node) = key_nodes.next() else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Un elemento Contents no contiene una clave S3",
            ));
        };
        if key_nodes.next().is_some() {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Un elemento Contents contiene varias claves S3",
            ));
        }
        let key = key_node.text().unwrap_or_default();
        if key.is_empty() {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Una clave S3 archivada está vacía",
            ));
        }
        if key.len() > MAX_KEY_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Una clave S3 supera el máximo de bytes CHEF",
            ));
        }
        if key.chars().any(char::is_control) || key.contains('\\') {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Una clave S3 contiene caracteres no permitidos",
            ));
        }
        if key.split('/').any(|segment| matches!(segment, "." | "..")) {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Una clave S3 contiene segmentos de ruta ambiguos",
            ));
        }

        let extension = file_extension(key);
        if extension
            .as_deref()
            .is_some_and(is_default_extension_blacklisted)
        {
            continue;
        }

        let url = object_url(&bucket_origin, key)?;
        hints.push(ArchivedBucketFileHint {
            bucket_origin: bucket_origin.as_str().trim_end_matches('/').to_owned(),
            key: key.to_owned(),
            url: url.to_string(),
            extension,
            source_locator: format!("ListBucketResult/Contents[{}]/Key", index + 1),
        });
        if hints.len() >= file_limit {
            break;
        }
    }
    Ok(hints)
}

fn parse_bucket_origin(input: &str) -> ChefResult<Url> {
    let mut url = Url::parse(input).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El origen del bucket no es una URL válida",
        )
    })?;
    let host = url
        .host_str()
        .ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "El origen del bucket no tiene host",
            )
        })?
        .to_ascii_lowercase();
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || !(host.ends_with(".s3.amazonaws.com") || host.ends_with(".digitaloceanspaces.com"))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El origen debe ser un bucket HTTPS de AWS S3 o DigitalOcean Spaces, sin ruta ni credenciales",
        ));
    }
    url.set_path("/");
    Ok(url)
}

fn s3_children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(move |child| {
        child.is_element()
            && child.tag_name().name() == name
            && child.tag_name().namespace() == Some(S3_NAMESPACE)
    })
}

fn file_extension(key: &str) -> Option<String> {
    let filename = key.rsplit('/').next()?;
    let (stem, extension) = filename.rsplit_once('.')?;
    if stem.is_empty() || extension.is_empty() {
        return None;
    }
    Some(extension.to_ascii_lowercase())
}

fn is_default_extension_blacklisted(extension: &str) -> bool {
    matches!(
        extension,
        "png"
            | "jpg"
            | "bmp"
            | "ico"
            | "jpeg"
            | "gif"
            | "svg"
            | "webp"
            | "css"
            | "woff"
            | "woff2"
            | "ttf"
            | "eot"
            | "sass"
            | "scss"
            | "mp3"
            | "m4a"
            | "wav"
            | "flac"
            | "mp4"
            | "mkv"
            | "avi"
            | "wmv"
            | "mov"
            | "flv"
            | "webm"
    )
}

fn object_url(origin: &Url, key: &str) -> ChefResult<Url> {
    let mut url = origin.clone();
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "No se pudo construir la URL del objeto S3",
            )
        })?;
        segments.pop_if_empty();
        segments.extend(key.split('/'));
    }
    if url.origin() != origin.origin() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La URL del objeto no permanece en el origen del bucket",
        ));
    }
    Ok(url)
}

fn reject_dtd_and_entities(xml: &str) -> ChefResult<()> {
    let upper = xml.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "DTD y declaraciones de entidades XML están deshabilitados",
        ));
    }
    Ok(())
}

fn check_xml_depth(xml: &str) -> ChefResult<()> {
    let bytes = xml.as_bytes();
    let mut index = 0usize;
    let mut depth = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"<!--") {
            index = skip_through(bytes, index + 4, b"-->")?;
            continue;
        }
        if bytes[index..].starts_with(b"<![CDATA[") {
            index = skip_through(bytes, index + 9, b"]]>")?;
            continue;
        }
        if bytes[index..].starts_with(b"<?") {
            index = skip_through(bytes, index + 2, b"?>")?;
            continue;
        }
        let end = find_tag_end(bytes, index + 1)?;
        let body = &bytes[index + 1..end];
        if body.first() == Some(&b'/') {
            depth = depth.saturating_sub(1);
        } else if body.first() == Some(&b'!') {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Declaraciones XML no compatibles",
            ));
        } else if body.iter().rev().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'/') {
            depth += 1;
            if depth > 32 {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El listado XML supera la profundidad máxima CHEF",
                ));
            }
        }
        index = end + 1;
    }
    Ok(())
}

fn find_tag_end(bytes: &[u8], mut index: usize) -> ChefResult<usize> {
    let mut quote = None;
    while index < bytes.len() {
        let byte = bytes[index];
        match (quote, byte) {
            (Some(q), current) if q == current => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Ok(index),
            _ => {}
        }
        index += 1;
    }
    Err(ChefError::new(ErrorCode::InvalidInput, "XML malformado"))
}

fn skip_through(bytes: &[u8], mut from: usize, terminator: &[u8]) -> ChefResult<usize> {
    while from + terminator.len() <= bytes.len() {
        if bytes[from..].starts_with(terminator) {
            return Ok(from + terminator.len());
        }
        from += 1;
    }
    Err(ChefError::new(ErrorCode::InvalidInput, "XML malformado"))
}

#[cfg(test)]
mod tests {
    use super::{MAX_LISTING_RECORDS, parse_archived_bucket_file_listing};
    use crate::ErrorCode;

    const ORIGIN: &str = "https://testbucket.s3.amazonaws.com/";
    const LISTING: &[u8] = br#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Contents><Key>index.html</Key></Contents><Contents><Key>dir/test.pdf</Key></Contents><Contents><Key>style.CSS</Key></Contents><Contents><Key>space here.txt</Key></Contents></ListBucketResult>"#;

    #[test]
    fn parses_archived_bucket_keys_as_same_origin_hints() {
        let hints = parse_archived_bucket_file_listing(ORIGIN, LISTING, 50).unwrap();
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "index.html");
        assert_eq!(
            hints[0].url,
            "https://testbucket.s3.amazonaws.com/index.html"
        );
        assert_eq!(hints[0].extension.as_deref(), Some("html"));
        assert_eq!(hints[1].source_locator, "ListBucketResult/Contents[2]/Key");
        assert_eq!(
            hints[2].url,
            "https://testbucket.s3.amazonaws.com/space%20here.txt"
        );
        assert!(
            hints
                .iter()
                .all(|hint| hint.bucket_origin == "https://testbucket.s3.amazonaws.com")
        );
    }

    #[test]
    fn rejects_malformed_scope_spoofed_dtd_and_entity_xml() {
        for (origin, xml) in [
            ("https://example.com/", LISTING),
            ("https://user@testbucket.s3.amazonaws.com/", LISTING),
            ("https://testbucket.s3.amazonaws.com/path/", LISTING),
            ("https://testbucket.s3.amazonaws.com/?x=1", LISTING),
            (
                ORIGIN,
                br#"<!DOCTYPE x [<!ENTITY e SYSTEM "file:///etc/passwd">]><ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Contents><Key>&e;</Key></Contents></ListBucketResult>"#,
            ),
            (ORIGIN, b"<ListBucketResult"),
        ] {
            assert!(parse_archived_bucket_file_listing(origin, xml, 50).is_err());
        }
    }

    #[test]
    fn rejects_unsafe_keys_and_caps_hints_without_partial_xml_errors() {
        let traversal = br#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Contents><Key>../outside.pdf</Key></Contents></ListBucketResult>"#;
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, traversal, 50)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        let ambiguous = br#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Contents><Key>one.pdf</Key><Key>two.pdf</Key></Contents></ListBucketResult>"#;
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, ambiguous, 50)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, LISTING, 1)
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, LISTING, 0)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_xml_and_key_limits() {
        let oversized = vec![b' '; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, &oversized, 50)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let long_key = format!(
            "<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\"><Contents><Key>{}.txt</Key></Contents></ListBucketResult>",
            "x".repeat(1024)
        );
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, long_key.as_bytes(), 50)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let mut many_records =
            String::from("<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">");
        for _ in 0..=MAX_LISTING_RECORDS {
            many_records.push_str("<Contents><Key>file.pdf</Key></Contents>");
        }
        many_records.push_str("</ListBucketResult>");
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, many_records.as_bytes(), 50)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let nested_open = "<n>".repeat(32);
        let nested_close = "</n>".repeat(32);
        let too_deep = format!(
            "<ListBucketResult xmlns=\"http://s3.amazonaws.com/doc/2006-03-01/\">{nested_open}{nested_close}</ListBucketResult>"
        );
        assert_eq!(
            parse_archived_bucket_file_listing(ORIGIN, too_deep.as_bytes(), 50)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
