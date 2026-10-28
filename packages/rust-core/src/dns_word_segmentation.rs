//! Segmentación local de etiquetas DNS usando frecuencias de dominio.
//!
//! El léxico se compila dentro del núcleo: no accede al filesystem ni requiere
//! dependencias de descompresión en tiempo de ejecución. Las salidas conservan
//! solo tokens alfanuméricos útiles para derivar pistas CHEF.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

const DNS_WORD_FREQUENCIES: &str = include_str!("../data/dns-word-frequency.txt");
static MODEL: OnceLock<WordModel> = OnceLock::new();

struct WordModel {
    costs: HashMap<&'static str, f64>,
    max_word_bytes: usize,
}

impl WordModel {
    fn embedded() -> Self {
        let words = DNS_WORD_FREQUENCIES.lines().collect::<Vec<_>>();
        let count = words.len();
        let rank_scale = (count as f64).ln();
        let mut costs = HashMap::with_capacity(count);
        let mut max_word_bytes = 1;

        for (index, word) in words.into_iter().enumerate() {
            if word.is_empty() {
                continue;
            }
            max_word_bytes = max_word_bytes.max(word.len());
            // Igual que el modelo origen: Zipf por rango; duplicados conservan
            // el coste de su última aparición en el fichero.
            costs.insert(word, (((index + 1) as f64) * rank_scale).ln());
        }

        Self {
            costs,
            max_word_bytes,
        }
    }

    fn split(&self, value: &str) -> Vec<String> {
        let bytes = value.as_bytes();
        let mut costs = vec![0.0; bytes.len() + 1];
        let mut lengths = vec![0; bytes.len() + 1];

        for end in 1..=bytes.len() {
            let mut best = (f64::INFINITY, usize::MAX);
            for length in 1..=self.max_word_bytes.min(end) {
                let start = end - length;
                let word = &value[start..end];
                let word_cost = self.costs.get(word).copied().unwrap_or(f64::INFINITY);
                let candidate = (costs[start] + word_cost, length);
                if candidate.0 < best.0 || (candidate.0 == best.0 && candidate.1 < best.1) {
                    best = candidate;
                }
            }
            costs[end] = best.0;
            lengths[end] = best.1;
        }

        let mut reversed = Vec::new();
        let mut end = bytes.len();
        while end > 0 {
            let length = lengths[end];
            if length == 0 || length > end {
                break;
            }
            let token = &value[end - length..end];
            let mut new_token = token != "'";
            if new_token
                && !reversed.is_empty()
                && (reversed
                    .last()
                    .is_some_and(|previous: &String| previous == "'s")
                    || (token.as_bytes().last().is_some_and(u8::is_ascii_digit)
                        && reversed.last().is_some_and(|previous: &String| {
                            previous.as_bytes().first().is_some_and(u8::is_ascii_digit)
                        })))
            {
                reversed
                    .last_mut()
                    .expect("non-empty checked")
                    .insert_str(0, token);
                new_token = false;
            }
            if new_token {
                reversed.push(token.to_owned());
            }
            end -= length;
        }
        reversed.reverse();
        reversed
    }
}

/// Extrae coincidencias de los cuatro patrones DNS del mutador y agrega sus
/// segmentos probables. El orden de salida es estable y no depende de HashMap.
pub(crate) fn extract_words(label: &str) -> BTreeSet<String> {
    if !label.is_ascii() {
        return BTreeSet::new();
    }
    let normalized = label.to_ascii_lowercase();
    let patterns: [fn(u8) -> bool; 4] = [
        |byte| byte.is_ascii_alphabetic(),
        |byte| byte.is_ascii_alphabetic() || matches!(byte, b'_' | b'-'),
        |byte| byte.is_ascii_alphanumeric(),
        |byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'),
    ];
    let mut matches = BTreeSet::new();

    for accepts in patterns {
        let bytes = normalized.as_bytes();
        let mut start = None;
        for (index, byte) in bytes.iter().copied().enumerate() {
            if accepts(byte) {
                start.get_or_insert(index);
            } else if let Some(begin) = start.take() {
                matches.insert(&normalized[begin..index]);
            }
        }
        if let Some(begin) = start {
            matches.insert(&normalized[begin..]);
        }
    }

    let model = MODEL.get_or_init(WordModel::embedded);
    let mut words = BTreeSet::new();
    for matched in matches {
        words.insert(matched.to_owned());
        words.extend(model.split(matched).into_iter().filter(|word| {
            !word.is_empty() && word.bytes().all(|byte| byte.is_ascii_alphanumeric())
        }));
    }
    words
}
