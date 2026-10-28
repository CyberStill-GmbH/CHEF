//! Frecuencias de palabras locales para una ejecución CHEF.
//!
//! El núcleo recibe tokens ya extraídos de evidencia aceptada; no abre wordlists,
//! no parsea eventos sin validar y no comparte estado entre proyectos.

use std::collections::{BTreeMap, BTreeSet};

const MAX_WORDS: usize = 100_000;
const MAX_WORD_LENGTH: usize = 100;
const MAX_SEGMENT_LENGTH: usize = 32;

#[derive(Clone)]
struct Segmentation {
    score: u64,
    words: Vec<String>,
}

/// Frecuencia determinista de un token dentro de un único trabajo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsWordFrequency {
    pub word: String,
    pub count: u64,
}

/// Nube de palabras acotada, sin persistencia ni estado global entre proyectos.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DnsWordCloud {
    words: BTreeMap<String, u64>,
    rejected_words: usize,
}

impl DnsWordCloud {
    /// Añade un token ya extraído de una fuente de evidencia aceptada.
    /// Retorna `true` si se incorporó; tokens inválidos o fuera del límite se cuentan.
    pub fn add_word(&mut self, word: &str) -> bool {
        let word = word.trim().to_lowercase();
        if word.is_empty()
            || word.len() > MAX_WORD_LENGTH
            || word.chars().any(char::is_control)
            || !word.chars().any(char::is_alphanumeric)
        {
            self.rejected_words = self.rejected_words.saturating_add(1);
            return false;
        }

        if let Some(count) = self.words.get_mut(&word) {
            *count = count.saturating_add(1);
            return true;
        }
        if self.words.len() >= MAX_WORDS {
            self.rejected_words = self.rejected_words.saturating_add(1);
            return false;
        }
        self.words.insert(word, 1);
        true
    }

    /// Extrae tokens léxicos acotados y los incorpora a la nube. Conserva
    /// grupos alfanuméricos, variantes con `-`/`_`, segmentos y acrónimos;
    /// no realiza segmentación estadística de palabras concatenadas.
    pub fn absorb_text(&mut self, text: &str, include_acronyms: bool) -> usize {
        let mut added = 0;
        for token in extract_dns_words(text, include_acronyms) {
            if self.add_word(&token) {
                added += 1;
            }
        }
        added
    }

    /// Devuelve las palabras más frecuentes; los empates se ordenan por texto.
    pub fn top_words(&self, limit: usize) -> Vec<DnsWordFrequency> {
        let mut words: Vec<_> = self
            .words
            .iter()
            .map(|(word, count)| DnsWordFrequency {
                word: word.clone(),
                count: *count,
            })
            .collect();
        words.sort_by(|left, right| {
            right
                .count
                .cmp(&left.count)
                .then_with(|| left.word.cmp(&right.word))
        });
        words.truncate(limit.min(MAX_WORDS));
        words
    }

    /// Segmenta una palabra concatenada usando únicamente términos observados
    /// en esta nube. Solo devuelve particiones completas de dos o más términos;
    /// no usa ni descarga un corpus externo.
    pub fn split_known_compound(&self, word: &str) -> Option<Vec<String>> {
        let word = word.to_ascii_lowercase();
        if word.len() < 2
            || word.len() > MAX_WORD_LENGTH
            || !word.bytes().all(|byte| byte.is_ascii_alphanumeric())
        {
            return None;
        }

        let mut best: Vec<Option<Segmentation>> = vec![None; word.len() + 1];
        best[0] = Some(Segmentation {
            score: 0,
            words: Vec::new(),
        });
        for start in 0..word.len() {
            let Some(prefix) = best[start].clone() else {
                continue;
            };
            let max_end = (start + MAX_SEGMENT_LENGTH).min(word.len());
            for end in (start + 1)..=max_end {
                if start == 0 && end == word.len() {
                    continue;
                }
                let segment = &word[start..end];
                let Some(frequency) = self.words.get(segment).copied() else {
                    continue;
                };
                let mut candidate = prefix.clone();
                candidate.score = candidate.score.saturating_add(
                    (segment.len() as u64).saturating_mul(100 + frequency.min(1_000)),
                );
                candidate.words.push(segment.to_owned());
                let replace = best[end].as_ref().is_none_or(|current| {
                    candidate.score > current.score
                        || candidate.score == current.score
                            && (candidate.words.len() < current.words.len()
                                || candidate.words.len() == current.words.len()
                                    && candidate.words.as_slice() < current.words.as_slice())
                });
                if replace {
                    best[end] = Some(candidate);
                }
            }
        }
        best.pop()?
            .filter(|segmentation| segmentation.words.len() > 1)
            .map(|segmentation| segmentation.words)
    }

    pub(crate) fn frequency(&self, word: &str) -> Option<DnsWordFrequency> {
        self.words.get(word).map(|count| DnsWordFrequency {
            word: word.to_owned(),
            count: *count,
        })
    }

    /// Retiene solo las `limit` palabras de mayor frecuencia, igual que el
    /// truncado de la nube de origen, con desempate estable por palabra.
    pub fn truncate(&mut self, limit: usize) {
        let keep: std::collections::BTreeSet<String> = self
            .top_words(limit)
            .into_iter()
            .map(|entry| entry.word)
            .collect();
        self.words.retain(|word, _| keep.contains(word));
    }

    pub fn len(&self) -> usize {
        self.words.len()
    }

    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    pub fn rejected_words(&self) -> usize {
        self.rejected_words
    }
}

pub(crate) fn extract_dns_words(text: &str, include_acronyms: bool) -> BTreeSet<String> {
    let mut tokens = BTreeSet::new();
    let mut compound = String::new();
    let mut segment = String::new();
    let mut segments = Vec::new();

    let flush_segment =
        |segment: &mut String, tokens: &mut BTreeSet<String>, segments: &mut Vec<String>| {
            if !segment.is_empty() {
                let value = std::mem::take(segment).to_lowercase();
                tokens.insert(value.clone());
                segments.push(value);
            }
        };

    let flush_compound = |compound: &mut String, tokens: &mut BTreeSet<String>| {
        if !compound.is_empty() {
            tokens.insert(std::mem::take(compound).to_lowercase());
        }
    };

    for character in text.chars() {
        if character.is_alphanumeric() {
            compound.push(character);
            segment.push(character);
        } else if character == '-' || character == '_' {
            flush_segment(&mut segment, &mut tokens, &mut segments);
            if !compound.ends_with(['-', '_']) && !compound.is_empty() {
                compound.push(character);
            }
        } else {
            flush_segment(&mut segment, &mut tokens, &mut segments);
            flush_compound(&mut compound, &mut tokens);
            add_acronym(&segments, include_acronyms, &mut tokens);
            segments.clear();
        }
    }
    flush_segment(&mut segment, &mut tokens, &mut segments);
    flush_compound(&mut compound, &mut tokens);
    add_acronym(&segments, include_acronyms, &mut tokens);
    tokens
}

fn add_acronym(segments: &[String], enabled: bool, tokens: &mut BTreeSet<String>) {
    if enabled && segments.len() > 1 {
        tokens.insert(
            segments
                .iter()
                .filter_map(|word| word.chars().next())
                .collect(),
        );
    }
}
