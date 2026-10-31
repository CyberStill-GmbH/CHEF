//! Identifica extensiones de recursos en una URL o Content-Type archivados.
//!
//! Adapta la clasificación de `filedownload.make_filename` para CHEF offline.
//! No construye rutas locales, no accede al sistema de archivos y no descarga
//! ni abre contenido. Una extensión es una pista del header/ruta, no valida el
//! formato real del cuerpo.

use sha2::{Digest, Sha256};
use url::Url;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_URL_BYTES: usize = 4096;
const MAX_CONTENT_TYPE_BYTES: usize = 256;

const ALLOWED_EXTENSIONS: &[&str] = &[
    "7z", "apk", "bak", "bash", "bashrc", "bz2", "cfg", "conf", "crt", "csv", "db", "dll", "doc",
    "docx", "exe", "ica", "indd", "ini", "jar", "json", "key", "log", "markdown", "md", "msi",
    "odg", "odp", "ods", "odt", "pdf", "pem", "pps", "ppsx", "ppt", "pptx", "ps1", "pub", "raw",
    "rar", "rdp", "rsa", "sh", "sql", "sqlite", "swp", "sxw", "tar", "tgz", "txt", "vbs", "war",
    "wpd", "xls", "xlsx", "xml", "xz", "yaml", "yml", "zip", "lzma",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileExtensionSource {
    UrlPath,
    ContentType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedResourceFileHint {
    /// Origen saneado del recurso; no incluye path, query ni fragmento.
    pub scoped_origin: String,
    pub extension: String,
    pub source: FileExtensionSource,
    /// Huella de la URL/header original para enlazar la evidencia archivada.
    pub evidence_sha256: String,
}

/// Clasifica un recurso descrito por evidencia archivada dentro del scope.
///
/// La extensión se toma primero del path de URL, como en la fuente. Si no hay
/// una extensión permitida, se usa un mapeo MIME local y deliberadamente
/// pequeño; no se descarga la base MIME remota usada por el módulo original.
pub fn classify_archived_resource(
    query_domain: &str,
    source_url: &str,
    content_type: Option<&str>,
) -> ChefResult<Option<ArchivedResourceFileHint>> {
    if source_url.len() > MAX_URL_BYTES
        || content_type.is_some_and(|value| value.len() > MAX_CONTENT_TYPE_BYTES)
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La evidencia de recurso supera los límites CHEF",
        ));
    }

    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La raíz consultada debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz consultada debe ser un dominio registrable ICANN",
        ));
    }
    let Some(url) = scoped_url(source_url, &query_domain) else {
        return Ok(None);
    };

    let (extension, source) = match extension_from_url(&url) {
        Some(extension) if is_allowed(&extension) => (extension, FileExtensionSource::UrlPath),
        _ => {
            let Some(extension) = content_type.and_then(extension_from_content_type) else {
                return Ok(None);
            };
            (extension.to_owned(), FileExtensionSource::ContentType)
        }
    };

    let scoped_origin = url.origin().ascii_serialization();
    let mut digest = Sha256::new();
    digest.update(source_url.as_bytes());
    digest.update([0]);
    if let Some(content_type) = content_type {
        digest.update(content_type.as_bytes());
    }
    let evidence_sha256 = format!("{:x}", digest.finalize());
    Ok(Some(ArchivedResourceFileHint {
        scoped_origin,
        extension,
        source,
        evidence_sha256,
    }))
}

fn scoped_url(value: &str, query_domain: &str) -> Option<Url> {
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return None;
    }
    let host = url.host_str().and_then(canonical_dns_name)?;
    let suffix = format!(".{query_domain}");
    if host != query_domain && !host.ends_with(&suffix) {
        return None;
    }
    url.set_query(None);
    url.set_fragment(None);
    Some(url)
}

fn extension_from_url(url: &Url) -> Option<String> {
    let filename = url.path_segments()?.next_back()?;
    let (_, extension) = filename.rsplit_once('.')?;
    (!extension.is_empty()
        && extension.len() <= 16
        && extension.bytes().all(|byte| byte.is_ascii_alphanumeric()))
    .then(|| extension.to_ascii_lowercase())
}

fn extension_from_content_type(content_type: &str) -> Option<&'static str> {
    let media_type = content_type.split(';').next()?.trim().to_ascii_lowercase();
    let extension = match media_type.as_str() {
        "application/pdf" => "pdf",
        "application/json" => "json",
        "application/zip" => "zip",
        "application/gzip" => "gz",
        "application/x-7z-compressed" => "7z",
        "application/x-rar-compressed" => "rar",
        "application/vnd.microsoft.portable-executable" => "exe",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/msword" => "doc",
        "application/xml" | "text/xml" => "xml",
        "text/csv" => "csv",
        "text/plain" => "txt",
        "application/sql" | "text/sql" => "sql",
        "application/x-sqlite3" => "sqlite",
        _ => return None,
    };
    is_allowed(extension).then_some(extension)
}

fn is_allowed(extension: &str) -> bool {
    ALLOWED_EXTENSIONS.contains(&extension)
}

#[cfg(test)]
mod tests {
    use super::{FileExtensionSource, classify_archived_resource};
    use crate::ErrorCode;

    #[test]
    fn classifies_scoped_url_and_content_type_without_retaining_sensitive_url_parts() {
        let from_url = classify_archived_resource(
            "example.com",
            "https://files.example.com/reports/report.PDF?token=private#section",
            Some("text/html"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(from_url.scoped_origin, "https://files.example.com");
        assert_eq!(from_url.extension, "pdf");
        assert_eq!(from_url.source, FileExtensionSource::UrlPath);
        assert!(!format!("{from_url:?}").contains("private"));

        let from_type = classify_archived_resource(
            "example.com",
            "https://example.com/download?id=private",
            Some("application/pdf; charset=binary"),
        )
        .unwrap()
        .unwrap();
        assert_eq!(from_type.extension, "pdf");
        assert_eq!(from_type.source, FileExtensionSource::ContentType);
    }

    #[test]
    fn rejects_out_of_scope_lookalikes_credentials_and_unsupported_types() {
        for url in [
            "https://evil-example.com/file.pdf",
            "https://user:secret@files.example.com/file.pdf",
            "file:///etc/passwd",
        ] {
            assert!(
                classify_archived_resource("example.com", url, None)
                    .unwrap()
                    .is_none()
            );
        }
        assert!(
            classify_archived_resource(
                "example.com",
                "https://example.com/download",
                Some("application/octet-stream")
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            classify_archived_resource("localhost", "https://example.com/a.pdf", None)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_input_limits_and_never_claims_body_format() {
        let long_url = format!("https://example.com/{}", "x".repeat(4096));
        assert_eq!(
            classify_archived_resource("example.com", &long_url, None)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            classify_archived_resource("example.com", "https://example.com/report.pdf", None)
                .unwrap()
                .unwrap()
                .extension,
            "pdf"
        );
    }
}
