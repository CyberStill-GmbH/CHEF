//! Adapta offline las respuestas archivadas de BuiltWith a nombres candidatos.
//!
//! No obtiene credenciales ni realiza solicitudes. Los nombres representan
//! referencias históricas de un proveedor y no observaciones DNS actuales.

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, ProviderSubdomainPage, parse_provider_subdomains};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_JSON_ENTRIES: usize = 2_048;
const MAX_CANDIDATE_FIELDS: usize = 8_192;

/// Normaliza dominios de `Results[].Result.Paths[]` y `Inbound`/`Outbound`
/// desde una respuesta BuiltWith archivada.
///
/// Conserva únicamente hostnames descendientes de la raíz ICANN consultada,
/// deduplicados y ordenados. La procedencia semántica de redirección y el
/// contenido detallado del proveedor se descartan; el llamador recibe pistas.
pub fn parse_builtwith_subdomains(
    query_domain: &str,
    response: &[u8],
) -> ChefResult<ProviderSubdomainPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta BuiltWith supera el límite de bytes CHEF",
        ));
    }

    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta BuiltWith no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta BuiltWith debe ser un objeto JSON",
    ))?;

    let mut candidates = Vec::new();
    let mut fields_seen = 0usize;
    if let Some(results) = object.get("Results") {
        let results = results.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Results de BuiltWith debe ser una lista",
        ))?;
        check_entry_limit(results.len())?;
        for result in results {
            let Some(paths) = result
                .get("Result")
                .and_then(Value::as_object)
                .and_then(|value| value.get("Paths"))
            else {
                continue;
            };
            let paths = paths.as_array().ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "Paths de BuiltWith debe ser una lista",
            ))?;
            check_entry_limit(paths.len())?;
            for path in paths {
                let Some(path) = path.as_object() else {
                    continue;
                };
                let domain = string_field(path.get("Domain"), &mut fields_seen)?;
                let subdomain = string_field(path.get("SubDomain"), &mut fields_seen)?;
                let Some(domain) = domain.filter(|value| !value.is_empty()) else {
                    continue;
                };
                let candidate = match subdomain.filter(|value| !value.is_empty()) {
                    Some(subdomain) => format!("{subdomain}.{domain}"),
                    None => domain.to_owned(),
                };
                push_if_valid_hostname(candidate, &mut candidates);
                check_candidate_limit(candidates.len())?;
            }
        }
    }

    for direction in ["Inbound", "Outbound"] {
        let Some(entries) = object.get(direction) else {
            continue;
        };
        let entries = entries.as_array().ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Inbound/Outbound de BuiltWith debe ser una lista",
        ))?;
        check_entry_limit(entries.len())?;
        for entry in entries {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            if let Some(domain) = string_field(entry.get("Domain"), &mut fields_seen)?
                .filter(|value| !value.is_empty())
            {
                push_if_valid_hostname(domain.to_owned(), &mut candidates);
                check_candidate_limit(candidates.len())?;
            }
        }
    }

    let mut normalized = Vec::new();
    for candidate in candidates {
        normalized.push(candidate);
        normalized.push("\n".to_owned());
    }
    parse_provider_subdomains(query_domain, normalized.concat().as_bytes())
}

fn push_if_valid_hostname(candidate: String, candidates: &mut Vec<String>) {
    let candidate = candidate.trim().trim_end_matches('.');
    if candidate.len() > 253 || candidate.contains('*') || !valid_ascii_hostname(candidate) {
        return;
    }
    candidates.push(candidate.to_owned());
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

fn string_field<'a>(
    value: Option<&'a Value>,
    fields_seen: &mut usize,
) -> ChefResult<Option<&'a str>> {
    let Some(value) = value else {
        return Ok(None);
    };
    *fields_seen += 1;
    if *fields_seen > MAX_CANDIDATE_FIELDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta BuiltWith supera el límite CHEF de campos",
        ));
    }
    match value {
        Value::String(value) => Ok(Some(value.as_str())),
        Value::Null => Ok(None),
        _ => Ok(None),
    }
}

fn check_entry_limit(count: usize) -> ChefResult<()> {
    if count > MAX_JSON_ENTRIES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta BuiltWith supera el límite CHEF de registros",
        ));
    }
    Ok(())
}

fn check_candidate_limit(count: usize) -> ChefResult<()> {
    if count > 2_048 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta BuiltWith supera el límite CHEF de nombres candidatos",
        ));
    }
    Ok(())
}
