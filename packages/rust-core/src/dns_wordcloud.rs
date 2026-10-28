//! Frecuencias de palabras locales para una ejecución CHEF.
//!
//! El núcleo recibe tokens ya extraídos de evidencia aceptada; no abre wordlists,
//! no parsea eventos sin validar y no comparte estado entre proyectos.

use std::collections::BTreeMap;

const MAX_WORDS: usize = 100_000;
const MAX_WORD_LENGTH: usize = 100;

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
        let mut tokens = std::collections::BTreeSet::new();
        let mut compound = String::new();
        let mut segment = String::new();
        let mut segments = Vec::new();

        let flush_segment = |segment: &mut String,
                             tokens: &mut std::collections::BTreeSet<String>,
                             segments: &mut Vec<String>| {
            if !segment.is_empty() {
                let value = std::mem::take(segment).to_lowercase();
                tokens.insert(value.clone());
                segments.push(value);
            }
        };

        let flush_compound =
            |compound: &mut String, tokens: &mut std::collections::BTreeSet<String>| {
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
                if include_acronyms && segments.len() > 1 {
                    let acronym: String = segments
                        .iter()
                        .filter_map(|word| word.chars().next())
                        .collect();
                    tokens.insert(acronym);
                }
                segments.clear();
            }
        }
        flush_segment(&mut segment, &mut tokens, &mut segments);
        flush_compound(&mut compound, &mut tokens);
        if include_acronyms && segments.len() > 1 {
            tokens.insert(
                segments
                    .iter()
                    .filter_map(|word| word.chars().next())
                    .collect(),
            );
        }

        let mut added = 0;
        for token in tokens {
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
