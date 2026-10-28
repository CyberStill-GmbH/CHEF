//! Normalización acotada de páginas archivadas de búsqueda Censys DNS.
//!
//! Adapta los nombres DNS que BBOT extrae de SAN de certificados. No realiza
//! solicitudes ni convierte un nombre incluido en un certificado en una
//! observación DNS actual.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_CERTIFICATE_HITS: usize = 100;
const MAX_NAMES: usize = 16_384;
const MAX_CANDIDATES: usize = 1024;
const MAX_CURSOR_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CensysDnsCertificatePage {
    pub query_domain: String,
    pub candidates: Vec<String>,
    /// Cursor opaco de la página siguiente. CHEF no lo sigue ni crea solicitudes.
    pub next_cursor: Option<String>,
}

/// Extrae nombres descendientes de una raíz de una página Censys archivada.
///
/// La respuesta se trata como entrada no confiable. Se limita el tamaño, los
/// certificados, los nombres y el resultado; los SAN de otros dominios se
/// omiten porque un certificado puede contener varios dominios.
pub fn parse_censys_dns_certificate_page(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<CensysDnsCertificatePage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Censys supera el límite de bytes CHEF",
        ));
    }

    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| {
            domain.parse::<IpAddr>().is_err() && crate::host::is_registrable_domain(domain)
        })
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Censys debe ser una raíz registrable ICANN",
        ))?;

    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Censys no contiene JSON válido",
        )
    })?;
    if document
        .get("status")
        .and_then(Value::as_str)
        .is_none_or(|status| !status.eq_ignore_ascii_case("ok"))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Censys no tiene estado OK",
        ));
    }

    let result = document.get("result").ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta Censys no contiene result",
    ))?;
    let hits = result
        .get("hits")
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Censys no contiene una lista de certificados",
        ))?;
    if hits.len() > MAX_CERTIFICATE_HITS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La página Censys supera el límite de certificados",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    let mut name_count = 0;
    for hit in hits {
        let hit = hit.as_object().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La lista Censys contiene un certificado con formato inválido",
        ))?;
        let Some(names) = hit.get("names") else {
            continue;
        };
        let names = names.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Un certificado Censys contiene names con formato inválido",
        ))?;
        name_count += names.len();
        if name_count > MAX_NAMES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta Censys supera el límite de nombres",
            ));
        }

        for name in names {
            let name = name.as_str().ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "La lista Censys contiene un nombre SAN que no es texto",
            ))?;
            let name = name.trim();
            let name = name.strip_prefix("*.").unwrap_or(name);
            let Some(candidate) = crate::host::canonical_domain(name) else {
                continue;
            };
            if candidate.parse::<IpAddr>().is_ok()
                || candidate == query_domain
                || !candidate.ends_with(&suffix)
            {
                continue;
            }
            candidates.insert(candidate);
            if candidates.len() > MAX_CANDIDATES {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "La respuesta Censys supera el límite de candidatos CHEF",
                ));
            }
        }
    }

    let next_cursor = match result.pointer("/links/next") {
        None | Some(Value::Null) => None,
        Some(Value::String(cursor)) if cursor.is_empty() => None,
        Some(Value::String(cursor))
            if cursor.len() <= MAX_CURSOR_BYTES && !cursor.chars().any(char::is_control) =>
        {
            Some(cursor.clone())
        }
        Some(Value::String(_)) => {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El cursor Censys supera el límite o contiene controles",
            ));
        }
        _ => {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta Censys contiene un cursor inválido",
            ));
        }
    };

    Ok(CensysDnsCertificatePage {
        query_domain,
        candidates: candidates.into_iter().collect(),
        next_cursor,
    })
}
