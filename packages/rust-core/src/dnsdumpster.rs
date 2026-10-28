//! Extrae subdominios candidatos de una respuesta DNSDumpster ya guardada.
//!
//! Adapta localmente la extracción de nombres de BBOT. No obtiene tokens, no
//! hace solicitudes HTTP y no convierte resultados del proveedor en DNS actual.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_HOST_TOKENS: usize = 16_384;
const MAX_CANDIDATES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsDumpsterSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
}

/// Extrae nombres descendientes de la raíz desde el cuerpo textual archivado.
///
/// La respuesta se procesa como texto no confiable, con límite total, límite
/// de tokens y salida determinista. Una entrada malformada o fuera de los
/// límites se rechaza completa; no se publican resultados parciales.
pub fn parse_dnsdumpster_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<DnsDumpsterSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta DNSDumpster supera el límite de bytes CHEF",
        ));
    }
    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta DNSDumpster debe ser una raíz registrable ICANN",
        ))?;
    let text = std::str::from_utf8(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta DNSDumpster no contiene texto UTF-8 válido",
        )
    })?;

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    let mut token_start = None;
    let mut token_count = 0;
    for (index, byte) in text.bytes().enumerate() {
        if is_host_token_byte(byte) {
            token_start.get_or_insert(index);
        } else if let Some(start) = token_start.take() {
            collect_candidate(
                &text[start..index],
                &suffix,
                &mut candidates,
                &mut token_count,
            )?;
        }
    }
    if let Some(start) = token_start {
        collect_candidate(&text[start..], &suffix, &mut candidates, &mut token_count)?;
    }

    Ok(DnsDumpsterSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
    })
}

fn is_host_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')
}

fn valid_ascii_hostname(value: &str) -> bool {
    value.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.as_bytes()[0].is_ascii_alphanumeric()
            && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
            && label
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    })
}

fn collect_candidate(
    token: &str,
    suffix: &str,
    candidates: &mut BTreeSet<String>,
    token_count: &mut usize,
) -> ChefResult<()> {
    if !token.contains('.') {
        return Ok(());
    }
    *token_count += 1;
    if *token_count > MAX_HOST_TOKENS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta DNSDumpster supera el límite CHEF de nombres examinados",
        ));
    }

    let candidate = token.strip_suffix('.').unwrap_or(token);
    if candidate.len() > 253 {
        return Ok(());
    }
    if !valid_ascii_hostname(candidate) {
        return Ok(());
    }
    let Some(candidate) = crate::host::canonical_domain(candidate) else {
        return Ok(());
    };
    if candidate != suffix[1..] && candidate.ends_with(suffix) {
        candidates.insert(candidate);
        if candidates.len() > MAX_CANDIDATES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta DNSDumpster supera el límite CHEF de subdominios",
            ));
        }
    }
    Ok(())
}
