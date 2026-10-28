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
