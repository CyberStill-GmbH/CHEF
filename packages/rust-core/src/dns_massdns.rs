//! Parser local y acotado de resultados JSONL de MassDNS.
//!
//! Adapta la lectura de respuestas de `core/helpers/dns/brute.py` para permitir
//! revisar archivos entregados explícitamente. No ejecuta el resolver ni
//! convierte sus respuestas en observaciones confirmadas de CHEF.

use std::collections::HashSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_BYTES: usize = 5 * 1024 * 1024;
const MAX_INPUT_LINES: usize = 100_000;
const MAX_LINE_BYTES: usize = 64 * 1024;
const MAX_ANSWERS: usize = 100_000;
const MAX_RDATA_BYTES: usize = 4 * 1024;

/// Respuesta normalizada con locator de línea en el archivo importado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResolverAnswer {
    pub line_number: usize,
    pub name: String,
    pub record_type: String,
    pub data: String,
}

/// Resumen explícito de la lectura; los descartes no desaparecen en silencio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResolverImportBatch {
    pub answers: Vec<DnsResolverAnswer>,
    pub skipped_invalid: usize,
    pub outside_root: usize,
    pub duplicates: usize,
    pub truncated: bool,
}

/// Lee JSONL de MassDNS como dato importado y conserva solo la primera
/// respuesta de cada línea, igual que el helper de origen. Los nombres fuera
/// de `root` se cuentan y excluyen; los resultados retenidos siguen siendo
/// evidencia importada de terceros, no una verificación hecha por CHEF.
pub fn parse_massdns_jsonl(
    bytes: &[u8],
    root: &str,
    max_answers: usize,
) -> ChefResult<DnsResolverImportBatch> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La salida JSONL DNS supera el máximo CHEF de 5 MiB",
        ));
    }
    let root = crate::host::canonical_domain(root).ok_or_else(invalid_root)?;
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La salida JSONL DNS debe usar codificación UTF-8 válida",
        )
    })?;
    if text.lines().count() > MAX_INPUT_LINES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La salida JSONL DNS supera 100 000 líneas",
        ));
    }

    let limit = max_answers.min(MAX_ANSWERS);
    let mut answers = Vec::with_capacity(limit.min(1024));
    let mut seen = HashSet::with_capacity(limit.min(1024));
    let mut skipped_invalid = 0;
    let mut outside_root = 0;
    let mut duplicates = 0;
    let mut truncated = false;

    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.len() > MAX_LINE_BYTES {
            skipped_invalid += 1;
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            skipped_invalid += 1;
            continue;
        };
        let Some(answer) = value
            .get("data")
            .and_then(|data| data.get("answers"))
            .and_then(Value::as_array)
            .and_then(|answers| answers.first())
        else {
            skipped_invalid += 1;
            continue;
        };
        let Some(raw_name) = answer.get("name").and_then(Value::as_str) else {
            skipped_invalid += 1;
            continue;
        };
        let Some(name) = crate::host::canonical_domain(raw_name) else {
            skipped_invalid += 1;
            continue;
        };
        if name == root || !name.ends_with(&format!(".{root}")) {
            outside_root += 1;
            continue;
        }
        let Some(record_type) = answer
            .get("type")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= 16
                    && value.bytes().all(|byte| byte.is_ascii_alphanumeric())
            })
        else {
            skipped_invalid += 1;
            continue;
        };
        let Some(data) = answer
            .get("data")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| {
                !value.is_empty()
                    && value.len() <= MAX_RDATA_BYTES
                    && !value.chars().any(char::is_control)
            })
        else {
            skipped_invalid += 1;
            continue;
        };

        let record_type = record_type.to_ascii_uppercase();
        let key = (name.clone(), record_type.clone(), data.to_owned());
        if !seen.insert(key) {
            duplicates += 1;
            continue;
        }
        if answers.len() == limit {
            truncated = true;
            continue;
        }
        answers.push(DnsResolverAnswer {
            line_number: index + 1,
            name,
            record_type,
            data: data.to_owned(),
        });
    }

    Ok(DnsResolverImportBatch {
        answers,
        skipped_invalid,
        outside_root,
        duplicates,
        truncated,
    })
}

fn invalid_root() -> ChefError {
    ChefError::new(ErrorCode::InvalidInput, "La raíz DNS no es válida")
}
