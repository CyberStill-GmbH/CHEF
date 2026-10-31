//! Extrae el formato declarado por un `Content-Disposition` archivado.
//!
//! No descarga ni abre el archivo, y el header no acredita que el cuerpo sea
//! un paquete válido. El nombre remoto se descarta; solo se conserva formato e ID.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_APP_ID_BYTES: usize = 256;
const MAX_HEADER_BYTES: usize = 8 * 1024;
const MAX_FILENAME_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AndroidPackageFormat {
    Apk,
    Xapk,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedApkAttachmentHint {
    pub app_id: String,
    pub format: AndroidPackageFormat,
}

/// Lee solo metadata de una respuesta HTTP archivada.
///
/// Una respuesta distinta de 200 o un header que no declara un APK/XAPK válido
/// produce `None`. El código 200/header tampoco valida el contenido del cuerpo.
pub fn parse_archived_apk_attachment(
    app_id: &str,
    status_code: u16,
    content_disposition: &str,
) -> ChefResult<Option<ArchivedApkAttachmentHint>> {
    if content_disposition.len() > MAX_HEADER_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Content-Disposition supera el límite CHEF",
        ));
    }
    let app_id = app_id.trim();
    if !valid_app_id(app_id) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El ID de aplicación contiene caracteres no válidos",
        ));
    }
    if status_code != 200 {
        return Ok(None);
    }
    let Some((disposition, filename)) = parse_disposition(content_disposition) else {
        return Ok(None);
    };
    if !disposition.eq_ignore_ascii_case("attachment")
        || filename.len() > MAX_FILENAME_BYTES
        || filename
            .chars()
            .any(|character| character.is_control() || matches!(character, '/' | '\\' | ':'))
    {
        return Ok(None);
    }
    let Some((_, extension)) = filename.rsplit_once('.') else {
        return Ok(None);
    };
    let format = match extension.to_ascii_lowercase().as_str() {
        "apk" => AndroidPackageFormat::Apk,
        "xapk" => AndroidPackageFormat::Xapk,
        _ => return Ok(None),
    };
    Ok(Some(ArchivedApkAttachmentHint {
        app_id: app_id.to_owned(),
        format,
    }))
}

fn valid_app_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_APP_ID_BYTES
        && !value.contains("..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn parse_disposition(header: &str) -> Option<(&str, &str)> {
    let mut parts = split_unquoted_parameters(header);
    let disposition = parts.next()?.trim();
    let mut filename = None;
    for part in parts {
        let Some((name, value)) = part.split_once('=') else {
            continue;
        };
        if !name.trim().eq_ignore_ascii_case("filename") {
            continue;
        }
        let value = value.trim();
        let unquoted = if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            &value[1..value.len() - 1]
        } else if value.contains('"') || value.contains('\'') {
            return None;
        } else {
            value
        };
        if unquoted.contains('\\') {
            return None;
        }
        filename = Some(unquoted);
        break;
    }
    Some((disposition, filename?))
}

fn split_unquoted_parameters(header: &str) -> impl Iterator<Item = &str> {
    let bytes = header.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    let mut escaped = false;
    for (index, byte) in bytes.iter().enumerate() {
        match *byte {
            b'\\' if quoted && !escaped => escaped = true,
            b'"' if !escaped => quoted = !quoted,
            b';' if !quoted => {
                parts.push(&header[start..index]);
                start = index + 1;
            }
            _ => escaped = false,
        }
    }
    parts.push(&header[start..]);
    parts.into_iter()
}

#[cfg(test)]
mod tests {
    use super::{AndroidPackageFormat, parse_archived_apk_attachment};
    use crate::ErrorCode;

    #[test]
    fn extracts_only_package_format_from_archived_attachment_header() {
        let result = parse_archived_apk_attachment(
            "com.example.app",
            200,
            "attachment; filename=\"remote-build.XAPK\"; size=1234",
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.app_id, "com.example.app");
        assert_eq!(result.format, AndroidPackageFormat::Xapk);
    }

    #[test]
    fn ignores_non_attachment_statuses_unsupported_formats_and_paths() {
        assert!(
            parse_archived_apk_attachment("com.example.app", 404, "attachment; filename=a.apk")
                .unwrap()
                .is_none()
        );
        for header in [
            "inline; filename=app.apk",
            "attachment; filename=app.zip",
            "attachment; filename=..\\app.apk",
            "attachment; filename=\"C:/app.apk\"",
        ] {
            assert!(
                parse_archived_apk_attachment("com.example.app", 200, header)
                    .unwrap()
                    .is_none()
            );
        }
    }

    #[test]
    fn rejects_invalid_ids_and_enforces_header_filename_limits() {
        assert_eq!(
            parse_archived_apk_attachment("../app", 200, "attachment; filename=a.apk")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        let oversized_header = "x".repeat(8 * 1024 + 1);
        assert_eq!(
            parse_archived_apk_attachment("com.example.app", 200, &oversized_header)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let oversized_filename = format!("attachment; filename=\"{}.apk\"", "x".repeat(256));
        assert!(
            parse_archived_apk_attachment("com.example.app", 200, &oversized_filename)
                .unwrap()
                .is_none()
        );
    }
}
