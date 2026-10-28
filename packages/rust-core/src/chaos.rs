//! Parser offline para respuestas archivadas de ProjectDiscovery Chaos.
//!
//! Adapta el filtrado de ruido PTR y el colapso de inundaciones de BBOT. No
//! realiza solicitudes, no consulta DNS y no interpreta el colapso como wildcard.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_PREFIXES: usize = 50_000;
const MAX_CANDIDATES: usize = 16_384;
const PER_PARENT_CAP: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosParentCollapse {
    pub parent: String,
    pub dropped_children: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChaosSubdomainPage {
    pub query_domain: String,
    pub provider_domain: Option<String>,
    pub candidates: Vec<String>,
    pub collapsed_parents: Vec<ChaosParentCollapse>,
}

/// Normaliza una respuesta Chaos archivada dentro del dominio consultado.
///
/// Los prefijos con cuatro octetos iniciales se descartan como ruido PTR. Si
/// un padre tiene más de 300 hijos únicos, se devuelve solo ese padre y se
/// informa cuántos hijos se colapsaron. Las colecciones finales son estables.
pub fn parse_chaos_subdomain_response(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ChaosSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Chaos supera el límite de bytes CHEF",
        ));
    }

    let query_domain = crate::host::canonical_domain(query_domain)
        .filter(|domain| {
            crate::host::is_registrable_domain(domain) || crate::host::is_subdomain(domain)
        })
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta Chaos debe ser un dominio registrable o subdominio ICANN",
        ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Chaos no contiene JSON válido",
        )
    })?;
    if !document.is_object() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Chaos debe ser un objeto JSON",
        ));
    }

    let empty_page = |provider_domain| ChaosSubdomainPage {
        query_domain: query_domain.clone(),
        provider_domain,
        candidates: Vec::new(),
        collapsed_parents: Vec::new(),
    };
    let Some(domain) = document.get("domain").and_then(Value::as_str) else {
        return Ok(empty_page(None));
    };
    let domain = domain.trim();
    if domain.is_empty() {
        return Ok(empty_page(None));
    }
    let provider_domain = crate::host::canonical_domain(domain)
        .filter(|domain| crate::host::is_registrable_domain(domain))
        .ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Chaos no identifica una raíz registrable ICANN",
        ))?;
    if query_domain != provider_domain && !query_domain.ends_with(&format!(".{provider_domain}")) {
        return Err(ChefError::new(
            ErrorCode::OutOfScope,
            "La consulta Chaos no pertenece al dominio declarado por el proveedor",
        ));
    }

    let Some(subdomains) = document.get("subdomains") else {
        return Ok(empty_page(Some(provider_domain)));
    };
    if subdomains.is_null() {
        return Ok(empty_page(Some(provider_domain)));
    }
    let prefixes = subdomains.as_array().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta Chaos no contiene una lista de prefijos",
    ))?;
    if prefixes.len() > MAX_PREFIXES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Chaos supera el límite de prefijos",
        ));
    }

    let suffix = format!(".{query_domain}");
    let mut full_names = BTreeSet::new();
    for prefix in prefixes {
        let prefix = prefix.as_str().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta Chaos contiene un prefijo que no es texto",
        ))?;
        let prefix = prefix
            .trim()
            .trim_matches(|character| matches!(character, '.' | '*'));
        if prefix.is_empty() || looks_like_ptr_noise(prefix) {
            continue;
        }
        let Some(full_name) = crate::host::canonical_domain(&format!("{prefix}.{provider_domain}"))
        else {
            continue;
        };
        if full_name.ends_with(&suffix) {
            full_names.insert(full_name);
        }
    }

    let mut parent_children: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for name in full_names {
        let Some((_, parent)) = name.split_once('.') else {
            continue;
        };
        parent_children
            .entry(parent.to_owned())
            .or_default()
            .insert(name);
    }

    let mut candidates = BTreeSet::new();
    let mut collapsed_parents = Vec::new();
    for (parent, children) in parent_children {
        if parent != query_domain && children.len() > PER_PARENT_CAP {
            candidates.insert(parent.clone());
            collapsed_parents.push(ChaosParentCollapse {
                parent,
                dropped_children: children.len(),
            });
        } else {
            candidates.extend(children);
        }
        if candidates.len() > MAX_CANDIDATES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta Chaos supera el límite de candidatos CHEF",
            ));
        }
    }

    Ok(ChaosSubdomainPage {
        query_domain,
        provider_domain: Some(provider_domain),
        candidates: candidates.into_iter().collect(),
        collapsed_parents,
    })
}

fn looks_like_ptr_noise(prefix: &str) -> bool {
    has_four_octets_prefix(prefix, b'.', false) || has_four_octets_prefix(prefix, b'-', true)
}

fn has_four_octets_prefix(value: &str, separator: u8, allow_dot_after_fourth: bool) -> bool {
    let bytes = value.as_bytes();
    let mut start = 0;
    for octet_index in 0..4 {
        let end = bytes[start..]
            .iter()
            .position(|byte| {
                *byte == separator || (octet_index == 3 && allow_dot_after_fourth && *byte == b'.')
            })
            .map_or(bytes.len(), |offset| start + offset);
        let octet = &bytes[start..end];
        if octet.is_empty() || octet.len() > 3 || !octet.iter().all(u8::is_ascii_digit) {
            return false;
        }
        if octet_index < 3 {
            if end == bytes.len() || bytes[end] != separator {
                return false;
            }
            start = end + 1;
        } else {
            return end == bytes.len() || bytes[end] == separator || bytes[end] == b'.';
        }
    }
    false
}
