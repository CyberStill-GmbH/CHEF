//! Resumen determinista CHEF de registros ASN normalizados.
//!
//! Adapta la salida tabular local de `modules/report/asn.py`. No consulta
//! proveedores ni demuestra que una IP pertenezca a una organización.

use std::collections::BTreeMap;

use crate::{AsnRecord, ChefError, ChefResult, ErrorCode};

const MAX_RECORDS: usize = 100_000;
const MAX_DISTINCT_ASNS: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsnSummaryRow {
    pub asn: u32,
    pub subnet_count: usize,
    pub name: String,
    pub description: String,
    pub country: String,
}

/// Agrupa los registros por ASN, conserva el primer metadato y ordena el
/// resultado numéricamente como el reporte fuente. ASN 0 representa unknown
/// y nunca se publica como fila.
pub fn summarize_asn_records(records: &[AsnRecord]) -> ChefResult<Vec<AsnSummaryRow>> {
    if records.len() > MAX_RECORDS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El reporte ASN supera el máximo de registros CHEF",
        ));
    }

    let mut rows = BTreeMap::<u32, AsnSummaryRow>::new();
    for record in records {
        if record.asn == 0 {
            continue;
        }
        if record.subnets.len() > 4_096
            || [&record.name, &record.description, &record.country]
                .iter()
                .any(|value| value.len() > 1_024 || value.chars().any(char::is_control))
        {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "El registro ASN no cumple los límites CHEF",
            ));
        }
        rows.entry(record.asn).or_insert_with(|| AsnSummaryRow {
            asn: record.asn,
            subnet_count: record.subnets.len(),
            name: record.name.clone(),
            description: record.description.clone(),
            country: record.country.clone(),
        });
        if rows.len() > MAX_DISTINCT_ASNS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El reporte ASN supera la cantidad máxima de ASN",
            ));
        }
    }
    Ok(rows.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(asn: u32, subnet: &str, name: &str) -> AsnRecord {
        AsnRecord {
            asn,
            subnets: if subnet.is_empty() {
                Vec::new()
            } else {
                vec![subnet.to_owned()]
            },
            name: name.to_owned(),
            description: format!("Descripción {name}"),
            country: "PE".to_owned(),
        }
    }

    #[test]
    fn summary_is_sorted_deduplicated_and_ignores_unknown_asn() {
        let result = summarize_asn_records(&[
            record(64512, "192.0.2.0/24", "Primero"),
            record(0, "", "Desconocido"),
            record(64496, "198.51.100.0/24", "Otro"),
            record(64512, "203.0.113.0/24", "Duplicado"),
        ])
        .expect("registros normalizados válidos");

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].asn, 64_496);
        assert_eq!(result[1].asn, 64_512);
        assert_eq!(result[1].subnet_count, 1);
        assert_eq!(result[1].name, "Primero");
    }

    #[test]
    fn summary_rejects_unbounded_metadata_and_record_sets() {
        let mut invalid = record(64_496, "", "AS");
        invalid.name = "x".repeat(1_025);
        assert_eq!(
            summarize_asn_records(&[invalid]).unwrap_err().code,
            ErrorCode::InvalidInput
        );

        let too_many = vec![record(64_496, "", "AS"); MAX_RECORDS + 1];
        assert_eq!(
            summarize_asn_records(&too_many).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
    }
}
