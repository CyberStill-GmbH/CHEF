//! Clasifica respuestas HTTP archivadas que podrían corresponder a `.git/config`.
//!
//! Adapta la detección de `modules/git.py` sin realizar solicitudes ni retener
//! el cuerpo. El resultado es una pista histórica, no una vulnerabilidad
//! confirmada ni una comprobación de disponibilidad actual.

use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedGitExposureHint {
    /// URL archivada que respondió con un contenido compatible con Git config.
    pub config_url: String,
    /// Directorio `.git/` derivado localmente de la URL de respuesta.
    pub repository_directory_url: String,
}

/// Clasifica una respuesta archivada para el chequeo de `.git/config` de BBOT.
///
/// Solo acepta status 200, una ruta literal `/.git/config`, mismo origen que
/// el objetivo suministrado, y un cuerpo UTF-8 acotado que contenga la sección
/// `[core]`. HTML y redirecciones entre orígenes se descartan. No consulta el
/// objetivo, no almacena el cuerpo y no afirma exposición actual.
pub fn classify_archived_git_config(
    target_url: &str,
    response_url: &str,
    status_code: u16,
    body: &[u8],
) -> ChefResult<Option<ArchivedGitExposureHint>> {
    if body.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta archivada supera el límite de bytes",
        ));
    }
    if status_code != 200 || body.is_empty() {
        return Ok(None);
    }

    let mut target = parse_http_url(target_url)?;
    let mut response = parse_http_url(response_url)?;
    target.set_query(None);
    target.set_fragment(None);
    let mut file_target = target.clone();
    let file_path = target.path().trim_end_matches('/');
    file_target.set_path(file_path);
    let mut directory_target = target.clone();
    let mut directory_path = directory_target.path().to_owned();
    if !directory_path.ends_with('/') {
        directory_path.push('/');
    }
    directory_target.set_path(&directory_path);
    let expected_paths = [
        file_target
            .join(".git/config")
            .ok()
            .map(|url| url.path().to_owned()),
        directory_target
            .join(".git/config")
            .ok()
            .map(|url| url.path().to_owned()),
    ];
    if response.origin() != target.origin()
        || !expected_paths
            .iter()
            .flatten()
            .any(|path| path.as_str() == response.path())
    {
        return Ok(None);
    }

    let text = std::str::from_utf8(body).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta archivada no contiene texto UTF-8 válido",
        )
    })?;
    let trimmed = text.trim_start_matches(char::is_whitespace);
    if starts_with_html_document(trimmed) || !text.lines().any(|line| line.trim() == "[core]") {
        return Ok(None);
    }

    let response_path = response.path().to_owned();
    let Some(directory_path) = response_path.strip_suffix("config") else {
        return Ok(None);
    };
    response.set_path(directory_path);
    response.set_query(None);
    response.set_fragment(None);
    let mut config = parse_http_url(response_url)?;
    config.set_query(None);
    config.set_fragment(None);

    Ok(Some(ArchivedGitExposureHint {
        config_url: config.to_string(),
        repository_directory_url: response.to_string(),
    }))
}

fn parse_http_url(value: &str) -> ChefResult<Url> {
    let url = Url::parse(value)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La URL archivada no es válida"))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La URL debe usar HTTP(S) y no contener credenciales",
        ));
    }
    Ok(url)
}

fn starts_with_html_document(value: &str) -> bool {
    let value = value.as_bytes();
    [b"<html".as_slice(), b"<body".as_slice()]
        .iter()
        .any(|prefix| {
            value
                .get(..prefix.len())
                .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
        })
}
