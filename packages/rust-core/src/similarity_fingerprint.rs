//! Huella SimHash para comparar textos archivados sin ejecutar operaciones externas.
//!
//! Adaptación acotada de `core/helpers/simhash.py`. No clasifica contenido ni
//! valida respuestas: la similitud solo ayuda a priorizar revisión local.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_NORMALIZATION_FILTER_BYTES: usize = 4 * 1024;
const MAX_BITS: u8 = 64;
const XXH_PRIME_1: u64 = 11_400_714_785_074_694_791;
const XXH_PRIME_2: u64 = 14_029_467_366_897_019_727;
const XXH_PRIME_3: u64 = 1_609_587_929_392_839_161;
const XXH_PRIME_4: u64 = 9_650_029_242_287_828_579;
const XXH_PRIME_5: u64 = 2_870_177_450_012_600_261;

/// Calcula una huella SimHash de `bits` bits para texto UTF-8.
///
/// El modo de truncado conserva los primeros 2 048 y los últimos 1 024
/// caracteres, como la utilidad fuente. CHEF limita también el tamaño total
/// recibido a 64 KiB y acepta entre 1 y 64 bits.
pub fn compute_similarity_fingerprint(
    text: &str,
    bits: u8,
    truncate: bool,
    normalization_filter: Option<&str>,
) -> ChefResult<u64> {
    validate_bits(bits)?;
    if text.len() > MAX_INPUT_BYTES {
        return Err(limit_error());
    }
    if normalization_filter.is_some_and(|filter| filter.len() > MAX_NORMALIZATION_FILTER_BYTES) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El filtro de normalización supera el máximo CHEF de 4 KiB",
        ));
    }

    let bounded = if truncate {
        truncate_text(text)
    } else {
        text.to_owned()
    };
    let normalized = match normalization_filter {
        Some(filter) if !filter.is_empty() => bounded.replace(filter, ""),
        _ => bounded,
    };

    let mut bit_weights = [0i64; MAX_BITS as usize];
    let word_text = normalized
        .chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric() || *character == '_')
        .collect::<String>();
    let chars = word_text.chars().collect::<Vec<_>>();

    if chars.len() < 3 {
        add_feature(&mut bit_weights, bits, &chars.iter().collect::<String>());
    } else {
        for window in chars.windows(3) {
            let feature = window.iter().collect::<String>();
            add_feature(&mut bit_weights, bits, &feature);
        }
    }

    let mut fingerprint = 0u64;
    for (index, weight) in bit_weights[..usize::from(bits)].iter().enumerate() {
        if *weight >= 0 {
            fingerprint |= 1u64 << index;
        }
    }
    Ok(fingerprint)
}

/// Calcula la similitud de Hamming entre dos valores SimHash.
pub fn similarity_fingerprint_score(left: u64, right: u64, bits: u8) -> ChefResult<f64> {
    validate_bits(bits)?;
    let mask = if bits == MAX_BITS {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    let distance = ((left ^ right) & mask).count_ones();
    Ok(1.0 - f64::from(distance) / f64::from(bits))
}

fn validate_bits(bits: u8) -> ChefResult<()> {
    if bits == 0 || bits > MAX_BITS {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La huella SimHash debe usar entre 1 y 64 bits",
        ));
    }
    Ok(())
}

fn truncate_text(text: &str) -> String {
    let char_count = text.chars().count();
    if char_count <= 3_072 {
        return text.to_owned();
    }
    let first_end = text
        .char_indices()
        .nth(2_048)
        .map_or(text.len(), |(index, _)| index);
    let last_start = text
        .char_indices()
        .nth(char_count - 1_024)
        .map_or(text.len(), |(index, _)| index);
    let mut output = String::with_capacity(first_end + text.len() - last_start);
    output.push_str(&text[..first_end]);
    output.push_str(&text[last_start..]);
    output
}

fn add_feature(weights: &mut [i64; MAX_BITS as usize], bits: u8, feature: &str) {
    let hash = xxhash64(feature.as_bytes());
    for (index, weight) in weights[..usize::from(bits)].iter_mut().enumerate() {
        if (hash >> index) & 1 == 1 {
            *weight += 1;
        } else {
            *weight -= 1;
        }
    }
}

fn xxhash64(input: &[u8]) -> u64 {
    let mut cursor = 0usize;
    let mut hash;
    if input.len() >= 32 {
        let mut v1 = XXH_PRIME_1.wrapping_add(XXH_PRIME_2);
        let mut v2 = XXH_PRIME_2;
        let mut v3 = 0u64;
        let mut v4 = 0u64.wrapping_sub(XXH_PRIME_1);
        while cursor <= input.len() - 32 {
            v1 = round(v1, read_u64(&input[cursor..]));
            v2 = round(v2, read_u64(&input[cursor + 8..]));
            v3 = round(v3, read_u64(&input[cursor + 16..]));
            v4 = round(v4, read_u64(&input[cursor + 24..]));
            cursor += 32;
        }
        hash = v1
            .rotate_left(1)
            .wrapping_add(v2.rotate_left(7))
            .wrapping_add(v3.rotate_left(12))
            .wrapping_add(v4.rotate_left(18));
        for lane in [v1, v2, v3, v4] {
            hash ^= round(0, lane);
            hash = hash.wrapping_mul(XXH_PRIME_1).wrapping_add(XXH_PRIME_4);
        }
    } else {
        hash = XXH_PRIME_5;
    }
    hash = hash.wrapping_add(input.len() as u64);

    while cursor + 8 <= input.len() {
        hash ^= round(0, read_u64(&input[cursor..]));
        hash = hash
            .rotate_left(27)
            .wrapping_mul(XXH_PRIME_1)
            .wrapping_add(XXH_PRIME_4);
        cursor += 8;
    }
    if cursor + 4 <= input.len() {
        hash ^= u64::from(read_u32(&input[cursor..])).wrapping_mul(XXH_PRIME_1);
        hash = hash
            .rotate_left(23)
            .wrapping_mul(XXH_PRIME_2)
            .wrapping_add(XXH_PRIME_3);
        cursor += 4;
    }
    while cursor < input.len() {
        hash ^= u64::from(input[cursor]).wrapping_mul(XXH_PRIME_5);
        hash = hash.rotate_left(11).wrapping_mul(XXH_PRIME_1);
        cursor += 1;
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(XXH_PRIME_2);
    hash ^= hash >> 29;
    hash = hash.wrapping_mul(XXH_PRIME_3);
    hash ^ (hash >> 32)
}

fn round(accumulator: u64, lane: u64) -> u64 {
    accumulator
        .wrapping_add(lane.wrapping_mul(XXH_PRIME_2))
        .rotate_left(31)
        .wrapping_mul(XXH_PRIME_1)
}

fn read_u64(bytes: &[u8]) -> u64 {
    u64::from_le_bytes(bytes[..8].try_into().expect("8-byte lane"))
}

fn read_u32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes(bytes[..4].try_into().expect("4-byte lane"))
}

fn limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "El texto supera el máximo CHEF de 64 KiB",
    )
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_INPUT_BYTES, compute_similarity_fingerprint, similarity_fingerprint_score, xxhash64,
    };

    #[test]
    fn preserves_source_fingerprint_behavior_and_normalization() {
        let left = compute_similarity_fingerprint("Hello, world!", 64, true, None).unwrap();
        let same = compute_similarity_fingerprint("hello world", 64, true, None).unwrap();
        let normalized =
            compute_similarity_fingerprint("token: alpha", 64, true, Some("token: ")).unwrap();
        let plain = compute_similarity_fingerprint("alpha", 64, true, None).unwrap();

        assert_eq!(left, same);
        assert_eq!(normalized, plain);
        assert!(similarity_fingerprint_score(left, same, 64).unwrap() > 0.99);
    }

    #[test]
    fn truncates_long_text_and_similarity_masks_unused_bits() {
        let prefix = "a".repeat(2_048);
        let suffix = "z".repeat(1_024);
        let text = format!("{prefix}middle{suffix}");
        let fingerprint = compute_similarity_fingerprint(&text, 8, true, None).unwrap();
        let expected =
            compute_similarity_fingerprint(&format!("{prefix}{suffix}"), 8, true, None).unwrap();
        assert_eq!(fingerprint, expected);
        assert_eq!(fingerprint & !0xff, 0);
        assert_eq!(similarity_fingerprint_score(0xff, 0x00, 8).unwrap(), 0.0);
        assert_eq!(similarity_fingerprint_score(0xff, 0x00, 4).unwrap(), 0.0);
    }

    #[test]
    fn rejects_invalid_bit_counts_and_unbounded_text() {
        assert!(compute_similarity_fingerprint("x", 0, true, None).is_err());
        assert!(compute_similarity_fingerprint("x", 65, true, None).is_err());
        assert!(compute_similarity_fingerprint("x", 64, true, Some(&"f".repeat(4097))).is_err());
        assert!(
            compute_similarity_fingerprint(&"x".repeat(MAX_INPUT_BYTES + 1), 64, false, None)
                .is_err()
        );
        assert!(similarity_fingerprint_score(0, 0, 0).is_err());
    }

    #[test]
    fn uses_standard_xxhash64_seed_zero() {
        assert_eq!(xxhash64(b""), 0xef46_db37_51d8_e999);
        assert_eq!(xxhash64(b"a"), 0xd24e_c4f1_a98c_6e5b);
    }
}
