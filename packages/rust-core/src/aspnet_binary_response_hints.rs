//! Resume el tipo de contenido declarado por una respuesta HTTP archivada.
//!
//! `application/x-msdownload` solo describe el header. No confirma que haya un
//! DLL, que sea accesible, que el servidor lo entregue completo o que exista una
//! vulnerabilidad. No se hacen probes ni se procesa el cuerpo binario.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_CONTENT_TYPE_BYTES: usize = 8 * 1024;
const MAX_MEDIA_TYPE_BYTES: usize = 255;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedAspNetBinaryResponseHint {
    pub status_code: u16,
    pub media_type: Option<String>,
    pub declares_windows_download_media_type: bool,
}

/// Normaliza una metadata `Content-Type` archivada sin interpretar el contenido.
pub fn parse_archived_aspnet_binary_response(
    status_code: u16,
    content_type: &str,
) -> ChefResult<ArchivedAspNetBinaryResponseHint> {
    if content_type.len() > MAX_CONTENT_TYPE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Content-Type supera el límite CHEF",
        ));
    }
    if content_type.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Content-Type contiene caracteres de control",
        ));
    }
    let media_type = content_type
        .split(';')
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(media_type) = media_type
        && (!valid_media_type(media_type) || media_type.len() > MAX_MEDIA_TYPE_BYTES)
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Content-Type no contiene un tipo MIME válido",
        ));
    }
    let media_type = media_type.map(str::to_ascii_lowercase);
    let declares_windows_download_media_type =
        status_code == 200 && media_type.as_deref() == Some("application/x-msdownload");
    Ok(ArchivedAspNetBinaryResponseHint {
        status_code,
        media_type,
        declares_windows_download_media_type,
    })
}

fn valid_media_type(value: &str) -> bool {
    let Some((type_name, subtype)) = value.split_once('/') else {
        return false;
    };
    !type_name.is_empty()
        && !subtype.is_empty()
        && !subtype.contains('/')
        && type_name
            .bytes()
            .chain(subtype.bytes())
            .all(|byte| byte.is_ascii_alphanumeric() || b"!#$&^_.+-".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_aspnet_binary_response;
    use crate::ErrorCode;

    #[test]
    fn reports_only_archived_media_type_metadata() {
        let result = parse_archived_aspnet_binary_response(
            200,
            "Application/X-MsDownload; name=example.dll",
        )
        .unwrap();
        assert_eq!(result.status_code, 200);
        assert_eq!(
            result.media_type.as_deref(),
            Some("application/x-msdownload")
        );
        assert!(result.declares_windows_download_media_type);
    }

    #[test]
    fn rejects_non_success_lookalikes_and_unrelated_media_types() {
        for (status, content_type) in [
            (206, "application/x-msdownload"),
            (200, "application/x-msdownloadish"),
            (200, "application/octet-stream"),
            (200, "application/x-msdownload; filename=bad\r\nheader"),
        ] {
            let result = parse_archived_aspnet_binary_response(status, content_type);
            if content_type.contains('\r') {
                assert_eq!(result.unwrap_err().code, ErrorCode::InvalidInput);
            } else {
                assert!(!result.unwrap().declares_windows_download_media_type);
            }
        }
    }

    #[test]
    fn enforces_header_and_mime_limits() {
        assert_eq!(
            parse_archived_aspnet_binary_response(200, &"x".repeat(8 * 1024 + 1))
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_aspnet_binary_response(200, &format!("{}/x", "a".repeat(256)),)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
