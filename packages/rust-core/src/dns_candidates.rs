//! Generación local y acotada de nombres DNS candidatos.
//!
//! Adapta la composición `prefijo.dominio` del helper de fuerza bruta sin
//! consultar DNS, descargar listas ni convertir candidatos en observaciones.

use std::collections::HashSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_PREFIXES: usize = 100_000;
const MAX_CANDIDATES: usize = 10_000;
const MAX_DNS_WORDLIST_BYTES: usize = 5 * 1024 * 1024;

/// Señala nombres DNS con patrones numéricos que el módulo de origen omite al
/// preparar fuerza bruta. Es una pista de calidad; no invalida ni elimina el
/// nombre de evidencia CHEF.
pub fn has_excessive_digits(value: &str) -> bool {
    let mut digit_runs = 0;
    let mut run_length = 0;
    let mut long_run = false;

    for character in value.chars() {
        if character.is_ascii_digit() {
            run_length += 1;
        } else if run_length > 0 {
            digit_runs += 1;
            long_run |= run_length > 3;
            run_length = 0;
        }
    }
    if run_length > 0 {
        digit_runs += 1;
        long_run |= run_length > 3;
    }
    digit_runs > 2 || long_run
}

/// Resultado explicable de una generación local de candidatos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsCandidateBatch {
    /// Nombres canonicalizados, únicos y en el orden de entrada.
    pub names: Vec<String>,
    /// Prefijos vacíos o que no forman un nombre de dominio válido.
    pub skipped_invalid: usize,
    /// Indica que se encontraron candidatos válidos más allá del límite.
    pub truncated: bool,
}

/// Resultado de leer contenido de una lista entregada explícitamente por el
/// adaptador de aplicación. El núcleo no abre rutas ni descarga listas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWordlist {
    /// Prefijos únicos y en orden de primera aparición.
    pub prefixes: Vec<String>,
    /// Líneas vacías, comentarios o entradas inválidas ignoradas.
    pub skipped_lines: usize,
    /// Entradas repetidas después de quitar espacios exteriores.
    pub duplicates: usize,
    /// Indica que se encontraron más entradas válidas que el límite solicitado.
    pub truncated: bool,
}

/// Analiza una lista DNS UTF-8 acotada, sin consultar filesystem o red.
/// Ignora líneas vacías, comentarios con `#` y controles; elimina duplicados
/// de forma determinista y limita el resultado a 100 000 prefijos.
pub fn parse_dns_wordlist(bytes: &[u8], max_entries: usize) -> ChefResult<DnsWordlist> {
    if bytes.len() > MAX_DNS_WORDLIST_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista DNS excede el máximo de 5 MiB",
        ));
    }
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La lista DNS debe usar codificación UTF-8 válida",
        )
    })?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let limit = max_entries.min(MAX_INPUT_PREFIXES);
    let mut prefixes = Vec::with_capacity(text.lines().count().min(limit));
    let mut seen = HashSet::with_capacity(text.lines().count().min(limit));
    let mut skipped_lines = 0;
    let mut duplicates = 0;
    let mut truncated = false;

    for line in text.lines() {
        let prefix = line.trim();
        if prefix.is_empty() || prefix.starts_with('#') {
            skipped_lines += 1;
            continue;
        }
        if prefix.len() > 253 || prefix.chars().any(char::is_control) {
            skipped_lines += 1;
            continue;
        }
        if seen.contains(prefix) {
            duplicates += 1;
            continue;
        }
        if prefixes.len() == limit {
            truncated = true;
            continue;
        }
        seen.insert(prefix.to_owned());
        prefixes.push(prefix.to_owned());
    }

    Ok(DnsWordlist {
        prefixes,
        skipped_lines,
        duplicates,
        truncated,
    })
}

/// Compone una lista de prefijos con una raíz, sin hacer resolución DNS.
///
/// La función preserva el orden de primera aparición, normaliza los hosts y
/// deduplica después de canonicalizar. El límite efectivo nunca supera 10 000;
/// entradas mayores a 100 000 se rechazan antes de procesarlas.
pub fn generate_dns_name_candidates(
    root: &str,
    prefixes: &[String],
    max_candidates: usize,
) -> ChefResult<DnsCandidateBatch> {
    if prefixes.len() > MAX_INPUT_PREFIXES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista de prefijos DNS excede el límite CHEF",
        ));
    }
    let Some(root) = crate::host::canonical_domain(root) else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz para generar nombres DNS no es válida",
        ));
    };

    let limit = max_candidates.min(MAX_CANDIDATES);
    let mut names = Vec::with_capacity(prefixes.len().min(limit));
    let mut seen = HashSet::with_capacity(prefixes.len().min(limit));
    let mut skipped_invalid = 0;
    let mut truncated = false;

    for prefix in prefixes {
        let prefix = prefix.trim();
        if prefix.is_empty() || prefix.len() > 253_usize.saturating_sub(root.len() + 1) {
            skipped_invalid += 1;
            continue;
        }
        let candidate = format!("{prefix}.{root}");
        let Some(candidate) = crate::host::canonical_domain(&candidate) else {
            skipped_invalid += 1;
            continue;
        };
        if candidate == root || !candidate.ends_with(&format!(".{root}")) {
            skipped_invalid += 1;
            continue;
        }
        if seen.contains(&candidate) {
            continue;
        }
        if names.len() == limit {
            truncated = true;
            continue;
        }
        seen.insert(candidate.clone());
        names.push(candidate);
    }

    Ok(DnsCandidateBatch {
        names,
        skipped_invalid,
        truncated,
    })
}
