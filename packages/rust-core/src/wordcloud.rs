//! Variaciones locales acotadas de etiquetas para pistas DNS.
//!
//! Este módulo adapta mutaciones por números, letras y modificadores operativos.
//! La segmentación probabilística se implementa en `dns_word_segmentation`.
//! No escribe archivos, no usa red y no resuelve nombres.

use std::collections::{BTreeMap, BTreeSet};

use crate::DnsMutationReason;

const DEVOPS_MODIFIERS: &[&str] = &[
    "www",
    "sub",
    "com",
    "test",
    "dev",
    "en",
    "api",
    "de",
    "m",
    "e",
    "staging",
    "metric",
    "a",
    "csg",
    "cdn",
    "host",
    "web",
    "net",
    "us",
    "the",
    "jp",
    "blog",
    "eu",
    "prod",
    "qa",
    "ssl",
    "mail",
    "online",
    "demo",
    "fr",
    "app",
    "init",
    "s",
    "fna",
    "br",
    "secure",
    "careers",
    "ppls",
    "it",
    "p",
    "static",
    "ru",
    "health",
    "free",
    "sn",
    "family",
    "stage",
    "admin",
    "d",
    "info",
    "mvm",
    "t",
    "am",
    "c",
    "in",
    "live",
    "i",
    "mobile",
    "uat",
    "stg",
    "ca",
    "new",
    "ri",
    "account",
    "of",
    "support",
    "pc",
    "shop",
    "dr",
    "service",
    "sasg",
    "west",
    "es",
    "at",
    "srv",
    "media",
    "h",
    "portal",
    "ar",
    "my",
    "id",
    "sas",
    "r",
    "hss",
    "video",
    "org",
    "server",
    "login",
    "la",
    "cms",
    "beta",
    "update",
    "vpn",
    "services",
    "int",
    "b",
    "assets",
    "pro",
    "east",
    "uk",
    "cloud",
    "ip",
    "mx",
    "store",
    "home",
    "production",
    "auto",
    "cn",
];

const MODIFIER_NUMBER_SUFFIXES: [&str; 3] = ["0", "1", "2"];
const COMMON_DNS_LABELS: [&str; 24] = [
    "www",
    "api",
    "admin",
    "mail",
    "app",
    "dev",
    "staging",
    "prod",
    "test",
    "login",
    "portal",
    "cdn",
    "web",
    "static",
    "secure",
    "vpn",
    "support",
    "blog",
    "shop",
    "service",
    "mobile",
    "assets",
    "cloud",
    "production",
];

/// Vocabulario local para sugerir nombres comunes bajo raíces DNS observadas.
/// Omite entradas de un carácter y números que aportan poco a una vista CHEF.
pub(crate) fn common_dns_labels() -> impl Iterator<Item = &'static str> {
    COMMON_DNS_LABELS.into_iter()
}

/// Produce bounded source-label, number and prefix/suffix modifier variants.
pub(crate) fn mutations(base: &str) -> BTreeMap<String, BTreeSet<DnsMutationReason>> {
    let base = base.to_ascii_lowercase();
    let mut output = BTreeMap::new();
    if base.is_empty() || !base.is_ascii() {
        return output;
    }

    record(&mut output, base.clone(), DnsMutationReason::WordCloudBase);
    for mutation in number_adjustments(&base) {
        if mutation != base {
            record(&mut output, mutation, DnsMutationReason::NumberAdjustment);
        }
    }
    for mutation in number_suffix_mutations(&base) {
        if mutation != base {
            record(&mut output, mutation, DnsMutationReason::NumericSuffix);
        }
    }

    let modifiers = modifiers();
    for modifier in modifiers {
        for delimiter in ["", ".", "-"] {
            record(
                &mut output,
                format!("{base}{delimiter}{modifier}"),
                DnsMutationReason::WordCloudModifier,
            );
            record(
                &mut output,
                format!("{modifier}{delimiter}{base}"),
                DnsMutationReason::WordCloudModifier,
            );
        }
    }
    output
}

fn modifiers() -> BTreeSet<String> {
    let mut modifiers = DEVOPS_MODIFIERS
        .iter()
        .map(|modifier| (*modifier).to_owned())
        .collect::<BTreeSet<_>>();
    modifiers.extend((b'a'..=b'z').map(|letter| char::from(letter).to_string()));
    modifiers.extend(MODIFIER_NUMBER_SUFFIXES.into_iter().map(str::to_owned));
    modifiers
}

fn number_adjustments(base: &str) -> BTreeSet<String> {
    let bytes = base.as_bytes();
    let mut digit_runs = Vec::new();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if !bytes[cursor].is_ascii_digit() {
            cursor += 1;
            continue;
        }
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor].is_ascii_digit() {
            cursor += 1;
        }
        digit_runs.push((start, cursor));
    }

    let mut output = BTreeSet::new();
    for (start, end) in digit_runs
        .into_iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let digits = &base[start..end];
        let Some(number) = digits.parse::<u32>().ok() else {
            continue;
        };
        let Some(max_width_number) = 10_u32
            .checked_pow(digits.len() as u32)
            .map(|value| value - 1)
        else {
            continue;
        };
        let low = number.saturating_sub(3);
        let high = number.saturating_add(3).min(max_width_number);
        for replacement in low..=high {
            let padded = format!("{replacement:0width$}", width = digits.len());
            output.insert(replace_range(base, start, end, &padded));
            if !digits.starts_with('0') {
                output.insert(replace_range(base, start, end, &replacement.to_string()));
            }
        }
    }

    output
}

fn number_suffix_mutations(base: &str) -> BTreeSet<String> {
    let bytes = base.as_bytes();
    let mut word_runs = Vec::new();
    let mut start = None;
    for (index, byte) in bytes.iter().copied().enumerate() {
        if byte.is_ascii_alphabetic() {
            start.get_or_insert(index);
        } else if let Some(begin) = start.take() {
            word_runs.push((begin, index));
        }
    }
    if let Some(begin) = start {
        word_runs.push((begin, bytes.len()));
    }

    let mut output = BTreeSet::new();
    for (_start, end) in word_runs
        .into_iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let after = &base[end..];
        if after.len() > 1 && !after.as_bytes()[0].is_ascii_digit() {
            for suffix in MODIFIER_NUMBER_SUFFIXES {
                output.insert(format!("{}{}{}", &base[..end], suffix, after));
            }
        }
    }

    for suffix in MODIFIER_NUMBER_SUFFIXES {
        output.insert(format!("{base}{suffix}"));
    }
    output.insert(base.to_owned());
    output
}

fn replace_range(base: &str, start: usize, end: usize, replacement: &str) -> String {
    format!("{}{replacement}{}", &base[..start], &base[end..])
}

fn record(
    output: &mut BTreeMap<String, BTreeSet<DnsMutationReason>>,
    mutation: String,
    reason: DnsMutationReason,
) {
    output.entry(mutation).or_default().insert(reason);
}
