//! Generador de etiquetas de muestra DNS con entropía del sistema operativo.

use std::{collections::BTreeSet, fmt};

const SAMPLE_COUNT: usize = 5;
const LABEL_LENGTH: usize = 10;
const MAX_LABEL_ATTEMPTS: usize = 32;
const ACCEPTED_RANDOM_BYTE_LIMIT: u8 = 234;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsWildcardLabelGenerationError {
    RandomSourceUnavailable,
    UniqueLabelLimit,
}

impl fmt::Display for DnsWildcardLabelGenerationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::RandomSourceUnavailable => {
                "No se pudo obtener aleatoriedad del sistema para muestras DNS"
            }
            Self::UniqueLabelLimit => "No se generaron etiquetas DNS únicas dentro del límite",
        })
    }
}

impl std::error::Error for DnsWildcardLabelGenerationError {}

/// Produce cinco etiquetas DNS únicas, en minúsculas y con diez letras ASCII.
/// No resuelve nombres ni realiza consultas.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemDnsWildcardSampleLabels;

impl SystemDnsWildcardSampleLabels {
    pub fn generate() -> Result<[String; SAMPLE_COUNT], DnsWildcardLabelGenerationError> {
        let mut labels = Vec::with_capacity(SAMPLE_COUNT);
        let mut unique = BTreeSet::new();
        for _ in 0..MAX_LABEL_ATTEMPTS {
            if labels.len() == SAMPLE_COUNT {
                break;
            }
            let candidate = random_label()?;
            if unique.insert(candidate.clone()) {
                labels.push(candidate);
            }
        }
        labels
            .try_into()
            .map_err(|_| DnsWildcardLabelGenerationError::UniqueLabelLimit)
    }
}

fn random_label() -> Result<String, DnsWildcardLabelGenerationError> {
    let mut label = String::with_capacity(LABEL_LENGTH);
    while label.len() < LABEL_LENGTH {
        let mut random_bytes = [0_u8; 16];
        getrandom::fill(&mut random_bytes)
            .map_err(|_| DnsWildcardLabelGenerationError::RandomSourceUnavailable)?;
        for byte in random_bytes {
            if byte >= ACCEPTED_RANDOM_BYTE_LIMIT {
                continue;
            }
            label.push(char::from(b'a' + byte % 26));
            if label.len() == LABEL_LENGTH {
                break;
            }
        }
    }
    Ok(label)
}
