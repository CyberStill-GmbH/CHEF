//! Generación local y acotada de nombres DNS candidatos.
//!
//! Adapta la composición `prefijo.dominio` del helper de fuerza bruta sin
//! consultar DNS, descargar listas ni convertir candidatos en observaciones.

use std::collections::HashSet;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_PREFIXES: usize = 100_000;
const MAX_CANDIDATES: usize = 10_000;
const MAX_DNS_WORDLIST_BYTES: usize = 5 * 1024 * 1024;
const MAX_DNS_BRUTE_DEPTH: usize = 127;

struct MutationSpec {
    label: String,
    rule_id: &'static str,
    modifier: Option<crate::DnsWordFrequency>,
    modifier_value: Option<String>,
    separator: Option<char>,
}

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

/// Evaluación local de profundidad y patrón numérico para preparar DNS brute.
/// No comprueba scope, wildcards, autorización, ni conectividad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsBruteRootAssessment {
    /// Raíz DNS canonicalizada.
    pub root: String,
    /// Profundidad que tendría el próximo nivel de candidatos.
    pub next_candidate_depth: usize,
    /// Profundidad configurada, normalizada al rango 1–127.
    pub max_depth: usize,
    /// Señal no concluyente de nombre generado automáticamente.
    pub excessive_digit_pattern: bool,
    /// Recomendación local para omitir la raíz al preparar fuerza bruta.
    pub recommended_skip: bool,
}

/// Candidato de mutación local con la etiqueta semilla y la regla explicable.
/// No representa una observación DNS ni incluye resultado de resolución.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMutationCandidate {
    pub name: String,
    pub root: String,
    pub seed_label: String,
    pub rule_id: &'static str,
    pub modifier: Option<crate::DnsWordFrequency>,
    pub modifier_value: Option<String>,
    pub separator: Option<char>,
    /// IDs deterministas de la evidencia usada como semilla, si la capa
    /// aplicativa los adjuntó. La generación aislada deja esta lista vacía.
    pub source_event_ids: Vec<String>,
    /// IDs de eventos que aportaron el término usado como modificador.
    pub modifier_source_event_ids: Vec<String>,
}

/// Resultado acotado de generar mutaciones DNS sin filesystem ni red.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsMutationBatch {
    pub candidates: Vec<DnsMutationCandidate>,
    pub skipped_invalid: usize,
    pub duplicates: usize,
    pub truncated: bool,
}

/// Adapta las condiciones locales de profundidad y exceso de dígitos del
/// módulo de origen. Una recomendación de omitir no descarta el activo en el
/// grafo ni sustituye la comprobación de autorización antes de cualquier red.
pub fn assess_dns_brute_root(root: &str, max_depth: usize) -> ChefResult<DnsBruteRootAssessment> {
    let Some(root) = crate::host::canonical_domain(root) else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz de fuerza bruta DNS no es un dominio válido",
        ));
    };
    let max_depth = max_depth.clamp(1, MAX_DNS_BRUTE_DEPTH);
    let next_candidate_depth = crate::host::subdomain_depth(&root).saturating_add(1);
    let excessive_digit_pattern = has_excessive_digits(&root);
    let recommended_skip = next_candidate_depth > max_depth || excessive_digit_pattern;

    Ok(DnsBruteRootAssessment {
        root,
        next_candidate_depth,
        max_depth,
        excessive_digit_pattern,
        recommended_skip,
    })
}

/// Propone variantes locales desde etiquetas observadas y conserva su semilla
/// y regla de generación. No marca el resultado como activo ni autoriza tráfico.
///
/// El subconjunto CHEF incluye etiqueta original, segmentos separados por
/// `-`, `_` o `.`, unión de esos segmentos, ajustes numéricos y combinaciones
/// prefijo/sufijo con letras, números y términos de nube. Se omiten las
/// mutaciones recursivas, recursos externos y consulta/resolución DNS.
pub fn generate_dns_mutation_candidates(
    root: &str,
    observed_labels: &[String],
    max_candidates: usize,
) -> ChefResult<DnsMutationBatch> {
    generate_dns_mutation_candidates_with_modifiers(root, observed_labels, &[], max_candidates)
}

/// Genera las variantes anteriores y añade prefijos/sufijos de las palabras
/// más frecuentes de una nube del mismo trabajo.
pub fn generate_dns_mutation_candidates_with_wordcloud(
    root: &str,
    observed_labels: &[String],
    word_cloud: &crate::DnsWordCloud,
    max_modifiers: usize,
    max_candidates: usize,
) -> ChefResult<DnsMutationBatch> {
    let modifiers = word_cloud.top_words(max_modifiers.min(128));
    generate_dns_mutation_candidates_with_modifiers(
        root,
        observed_labels,
        &modifiers,
        max_candidates,
    )
}

fn generate_dns_mutation_candidates_with_modifiers(
    root: &str,
    observed_labels: &[String],
    modifiers: &[crate::DnsWordFrequency],
    max_candidates: usize,
) -> ChefResult<DnsMutationBatch> {
    if observed_labels.len() > MAX_INPUT_PREFIXES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista de etiquetas DNS excede el límite CHEF",
        ));
    }
    let Some(root) = crate::host::canonical_domain(root) else {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La raíz para generar mutaciones DNS no es válida",
        ));
    };

    let limit = max_candidates.min(MAX_CANDIDATES);
    let mut candidates = Vec::with_capacity(observed_labels.len().min(limit));
    let mut seen = HashSet::with_capacity(observed_labels.len().min(limit));
    let mut skipped_invalid = 0;
    let mut duplicates = 0;
    let mut truncated = false;

    'seeds: for seed in observed_labels {
        let seed = seed.trim().to_ascii_lowercase();
        if seed.is_empty() || seed.len() > 63 || seed.chars().any(char::is_control) {
            skipped_invalid += 1;
            continue;
        }
        let mut mutations = vec![MutationSpec {
            label: seed.clone(),
            rule_id: "seed",
            modifier: None,
            modifier_value: None,
            separator: None,
        }];
        let segments: Vec<&str> = seed
            .split(['-', '_', '.'])
            .filter(|segment| !segment.is_empty())
            .collect();
        for segment in &segments {
            if *segment != seed {
                mutations.push(MutationSpec {
                    label: (*segment).to_owned(),
                    rule_id: "segment",
                    modifier: None,
                    modifier_value: None,
                    separator: None,
                });
            }
        }
        if segments.len() > 1 {
            mutations.push(MutationSpec {
                label: segments.concat(),
                rule_id: "separator-join",
                modifier: None,
                modifier_value: None,
                separator: None,
            });
        }
        mutations.extend(
            numeric_mutations(&seed)
                .into_iter()
                .map(|(label, rule_id)| MutationSpec {
                    label,
                    rule_id,
                    modifier: None,
                    modifier_value: None,
                    separator: None,
                }),
        );
        for number in 0..3 {
            add_modifier_mutations(
                &mut mutations,
                &seed,
                &number.to_string(),
                "numeric-prefix",
                "numeric-suffix",
                None,
            );
        }
        for letter in b'a'..=b'z' {
            let modifier = char::from(letter).to_string();
            add_modifier_mutations(
                &mut mutations,
                &seed,
                &modifier,
                "letter-prefix",
                "letter-suffix",
                None,
            );
        }
        for modifier in modifiers {
            if modifier.word == seed {
                continue;
            }
            add_modifier_mutations(
                &mut mutations,
                &seed,
                &modifier.word,
                "wordcloud-prefix",
                "wordcloud-suffix",
                Some(modifier.clone()),
            );
        }

        for mutation in mutations {
            let hostname = format!("{}.{root}", mutation.label);
            let Some(name) = crate::host::canonical_domain(&hostname) else {
                skipped_invalid += 1;
                continue;
            };
            if name == root || !name.ends_with(&format!(".{root}")) {
                skipped_invalid += 1;
                continue;
            }
            if !seen.insert(name.clone()) {
                duplicates += 1;
                continue;
            }
            if candidates.len() == limit {
                truncated = true;
                break 'seeds;
            }
            candidates.push(DnsMutationCandidate {
                name,
                root: root.clone(),
                seed_label: seed.clone(),
                rule_id: mutation.rule_id,
                modifier: mutation.modifier,
                modifier_value: mutation.modifier_value,
                separator: mutation.separator,
                source_event_ids: Vec::new(),
                modifier_source_event_ids: Vec::new(),
            });
        }
    }

    Ok(DnsMutationBatch {
        candidates,
        skipped_invalid,
        duplicates,
        truncated,
    })
}

fn add_modifier_mutations(
    mutations: &mut Vec<MutationSpec>,
    seed: &str,
    modifier_value: &str,
    prefix_rule: &'static str,
    suffix_rule: &'static str,
    frequency: Option<crate::DnsWordFrequency>,
) {
    for separator in [None, Some('.'), Some('-')] {
        let delimiter = separator.map_or_else(String::new, |value| value.to_string());
        mutations.push(MutationSpec {
            label: format!("{modifier_value}{delimiter}{seed}"),
            rule_id: prefix_rule,
            modifier: frequency.clone(),
            modifier_value: Some(modifier_value.to_owned()),
            separator,
        });
        mutations.push(MutationSpec {
            label: format!("{seed}{delimiter}{modifier_value}"),
            rule_id: suffix_rule,
            modifier: frequency.clone(),
            modifier_value: Some(modifier_value.to_owned()),
            separator,
        });
    }
}

fn numeric_mutations(seed: &str) -> Vec<(String, &'static str)> {
    let mut spans = Vec::new();
    let mut start = None;
    for (index, character) in seed.char_indices() {
        if character.is_ascii_digit() {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            spans.push((begin, index));
        }
    }
    if let Some(begin) = start {
        spans.push((begin, seed.len()));
    }

    let mut mutations = Vec::new();
    for (begin, end) in spans.into_iter().rev().take(3).rev() {
        let digits = &seed[begin..end];
        if digits.len() > 6 {
            continue;
        }
        let Ok(number) = digits.parse::<i32>() else {
            continue;
        };
        let min = (number - 3).max(0);
        let max = (number + 3).min(10_i32.pow(digits.len() as u32) - 1);
        for value in min..=max {
            let replacement = format!("{value:0width$}", width = digits.len());
            if replacement != digits {
                mutations.push((
                    format!("{}{replacement}{}", &seed[..begin], &seed[end..]),
                    "numeric-adjustment",
                ));
            }
        }
    }
    mutations
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
