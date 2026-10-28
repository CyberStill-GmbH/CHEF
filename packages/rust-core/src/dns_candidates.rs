//! Generación local y acotada de nombres DNS candidatos.
//!
//! Adapta la composición `prefijo.dominio` del helper de fuerza bruta sin
//! consultar DNS, descargar listas ni convertir candidatos en observaciones.

use std::collections::HashSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_PREFIXES: usize = 100_000;
const MAX_CANDIDATES: usize = 10_000;

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
