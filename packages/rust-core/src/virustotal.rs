//! Normalización acotada de páginas VirusTotal ya recibidas.
//!
//! El módulo no realiza solicitudes ni trata resultados de terceros como
//! observaciones DNS actuales. Solo entrega nombres candidatos que otra capa
//! puede publicar como pistas con procedencia explícita.

use std::collections::BTreeSet;
use std::net::IpAddr;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_PAGE_ENTRIES: usize = 256;
const MAX_CURSOR_BYTES: usize = 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirusTotalSubdomainPage {
    pub query_domain: String,
    pub candidates: Vec<String>,
    /// Cursor opaco para que el adaptador construya otra solicitud contra el
    /// endpoint fijo; nunca se sigue directamente `links.next`.
    pub next_cursor: Option<String>,
}

/// Valida una página de la relación `/domains/{domain}/subdomains`.
///
/// El cuerpo se usa solo en memoria. Cada `data[].id` debe ser un dominio
/// descendiente de la consulta; cualquier dato fuera de esa raíz rechaza la
/// página completa para evitar publicar resultados parciales o cruzados.
pub fn parse_virustotal_subdomain_page(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<VirusTotalSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta VirusTotal supera el límite de bytes",
        ));
    }

    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| {
            domain.parse::<IpAddr>().is_err() && crate::host::is_registrable_domain(domain)
        })
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta VirusTotal debe ser un dominio raíz válido",
        ))?;

    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta VirusTotal no contiene JSON válido",
        )
    })?;
    let entries = document
        .get("data")
        .and_then(Value::as_array)
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta VirusTotal no contiene una lista de dominios",
        ))?;
    if entries.len() > MAX_PAGE_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La página VirusTotal supera el límite de registros",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut candidates = BTreeSet::new();
    for entry in entries {
        if entry.get("type").and_then(Value::as_str) != Some("domain") {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta VirusTotal contiene un objeto que no es dominio",
            ));
        }
        let candidate = entry
            .get("id")
            .and_then(Value::as_str)
            .and_then(crate::host::canonical_domain)
            .filter(|domain| domain.parse::<IpAddr>().is_err())
            .ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta VirusTotal contiene un nombre de dominio inválido",
            ))?;
        if candidate == query_domain || !candidate.ends_with(&suffix) {
            return Err(ChefError::new(
                ErrorCode::OutOfScope,
                "La respuesta VirusTotal contiene un dominio fuera de la consulta",
            ));
        }
        candidates.insert(candidate);
    }

    let has_next = match document.pointer("/links/next") {
        None | Some(Value::Null) => false,
        Some(Value::String(next)) => !next.is_empty(),
        _ => {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La respuesta VirusTotal contiene un enlace de continuación inválido",
            ));
        }
    };
    let next_cursor = if has_next {
        let cursor = document
            .get("meta")
            .and_then(|meta| meta.get("cursor"))
            .and_then(Value::as_str)
            .filter(|cursor| {
                !cursor.is_empty()
                    && cursor.len() <= MAX_CURSOR_BYTES
                    && !cursor.chars().any(char::is_control)
            })
            .ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "La página VirusTotal indica continuación sin cursor válido",
            ))?;
        Some(cursor.to_owned())
    } else {
        None
    };

    Ok(VirusTotalSubdomainPage {
        query_domain,
        candidates: candidates.into_iter().collect(),
        next_cursor,
    })
}
