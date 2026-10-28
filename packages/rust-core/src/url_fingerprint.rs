//! Huella estable para agrupar rutas URL con forma similar.
//!
//! Adapta el propósito de `core/helpers/url.py::hash_url` sin reutilizar el
//! hash no estable de Python. No forma parte de la identidad de activos ni
//! debe usarse para deduplicar o descartar evidencia CHEF.

use sha2::{Digest, Sha256};
use url::Url;

const MAX_URL_BYTES: usize = 8 * 1024;

/// Devuelve una huella SHA-256 estable de host/puerto y forma de ruta.
///
/// Las consultas, fragmentos y credenciales se excluyen. Los segmentos se
/// representan por clase de caracteres, tipo de parámetro y extensión, de
/// forma que rutas como `/users/123` y `/users/456` compartan huella. La URL
/// nunca se visita y los valores originales no aparecen en el resultado.
pub fn url_shape_fingerprint(value: &str) -> Option<String> {
    if value.len() > MAX_URL_BYTES {
        return None;
    }
    let canonical = crate::url_tools::canonical_http_url(value)?;
    let url = Url::parse(&canonical).ok()?;
    let authority = canonical.split_once("://")?.1.split('/').next()?.to_owned();

    let mut shape = format!("authority={authority}\n");
    for segment in url.path().split('/') {
        let charset = charset_mask(segment);
        let parameter = parameter_kind(segment);
        let extension = segment
            .rsplit_once('.')
            .map_or("", |(_, extension)| extension);
        shape.push_str(&format!("{charset}:{parameter}:{extension}\n"));
    }

    let digest = Sha256::digest(shape.as_bytes());
    Some(format!("sha256:{}", hex_digest(&digest)))
}

fn charset_mask(value: &str) -> u8 {
    let mut mask = 0;
    for byte in value.bytes() {
        mask |= if byte.is_ascii_lowercase() {
            1
        } else if byte.is_ascii_uppercase() {
            2
        } else if byte.is_ascii_digit() {
            4
        } else {
            8
        };
    }
    mask
}

fn parameter_kind(value: &str) -> u8 {
    if is_integer(value) {
        1
    } else if is_uuid_shape(value) {
        2
    } else {
        3
    }
}

fn is_integer(value: &str) -> bool {
    let digits = value
        .strip_prefix('+')
        .or_else(|| value.strip_prefix('-'))
        .unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_uuid_shape(value: &str) -> bool {
    let compact = value.replace('-', "");
    compact.len() == 32 && compact.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hex_digest(digest: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(digest.len() * 2);
    for byte in digest {
        result.push(HEX[(byte >> 4) as usize] as char);
        result.push(HEX[(byte & 0x0f) as usize] as char);
    }
    result
}
