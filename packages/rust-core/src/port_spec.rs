//! Parser acotado de puertos/rangos heredado de `core.helpers.misc` de BBOT.
//!
//! Es una transformación pura para configuración; no abre sockets ni concede
//! autorización para consultar los puertos que devuelve.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_SPEC_BYTES: usize = 4 * 1024;
const MAX_PORT: u32 = u16::MAX as u32;

/// Convierte una lista separada por comas en puertos, expandiendo rangos
/// inclusivos (`"22,80,1000-1002"`). Conserva duplicados y el orden original,
/// como el helper de BBOT. Rechaza espacios, puertos cero, rangos invertidos y
/// entradas ambiguas; CHEF limita la cadena a 4 KiB.
pub fn parse_port_string(value: &str) -> ChefResult<Vec<u16>> {
    if value.is_empty() || value.len() > MAX_SPEC_BYTES {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La lista de puertos CHEF está vacía o supera 4 KiB",
        ));
    }

    let mut ports = Vec::new();
    for item in value.split(',') {
        if item.is_empty() {
            return Err(invalid_port_spec());
        }
        if let Some((start, end)) = item.split_once('-') {
            if end.contains('-') {
                return Err(invalid_port_spec());
            }
            let start = parse_port_number(start)?;
            let end = parse_port_number(end)?;
            if start >= end {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El inicio del rango de puertos CHEF debe ser menor que el final",
                ));
            }
            ports.extend((start..=end).map(|port| port as u16));
        } else {
            ports.push(parse_port_number(item)? as u16);
        }
    }

    Ok(ports)
}

fn parse_port_number(value: &str) -> ChefResult<u32> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid_port_spec());
    }
    let port = value.parse::<u32>().map_err(|_| invalid_port_spec())?;
    if port == 0 || port > MAX_PORT {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El puerto CHEF debe estar entre 1 y 65535",
        ));
    }
    Ok(port)
}

fn invalid_port_spec() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "La lista CHEF contiene un puerto o rango inválido",
    )
}
