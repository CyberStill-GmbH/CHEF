//! Exportación local CSV de la evidencia que CHEF ha proyectado.
//! No accede al filesystem y no añade datos que el grafo no conserva.

use crate::{
    ChefError, ChefReconGraph, ChefResult, ErrorCode, ReconAsset, ReconAssetKind,
    validate_chef_recon_graph,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

const MAX_EXPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_EXPORT_ROWS: usize = 40_000;
const HEADERS: [&str; 11] = [
    "id_activo",
    "tipo_activo",
    "clave_activo",
    "calificacion",
    "id_evidencia",
    "id_evento_origen",
    "tipo_evento",
    "observado_en",
    "localizador",
    "id_evidencia_padre",
    "redactado",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefEvidenceCsvExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub file_name: String,
    pub content_type: &'static str,
    pub scope_status: &'static str,
    pub row_count: usize,
    pub csv: String,
}

#[derive(Debug)]
struct CsvRow<'a> {
    asset: Option<&'a ReconAsset>,
    evidence: &'a crate::ReconEvidence,
}

/// Construye un CSV CHEF determinista con una fila por vínculo asset-evidencia.
/// La evidencia sin activo CHEF también se conserva con las columnas del activo vacías.
pub fn export_evidence_csv(
    run_id: &str,
    graph: &ChefReconGraph,
) -> ChefResult<ChefEvidenceCsvExport> {
    validate_run_id(run_id)?;
    validate_chef_recon_graph(graph)?;

    let mut link_count = 0usize;
    let mut linked_ids = BTreeSet::new();
    for asset in &graph.assets {
        link_count = link_count
            .checked_add(asset.evidence_ids.len())
            .ok_or_else(row_limit_error)?;
        if link_count > MAX_EXPORT_ROWS {
            return Err(row_limit_error());
        }
        linked_ids.extend(asset.evidence_ids.iter().map(String::as_str));
    }
    let orphan_count = graph
        .evidence
        .iter()
        .filter(|evidence| !linked_ids.contains(evidence.id.as_str()))
        .count();
    if link_count
        .checked_add(orphan_count)
        .is_none_or(|count| count > MAX_EXPORT_ROWS)
    {
        return Err(row_limit_error());
    }

    let mut rows = Vec::new();
    let mut linked_evidence = BTreeMap::<&str, Vec<&ReconAsset>>::new();

    for asset in &graph.assets {
        for evidence_id in &asset.evidence_ids {
            linked_evidence
                .entry(evidence_id.as_str())
                .or_default()
                .push(asset);
        }
    }
    for evidence in &graph.evidence {
        if let Some(assets) = linked_evidence.get(evidence.id.as_str()) {
            for asset in assets {
                push_bounded_row(
                    &mut rows,
                    CsvRow {
                        asset: Some(asset),
                        evidence,
                    },
                )?;
            }
        } else {
            push_bounded_row(
                &mut rows,
                CsvRow {
                    asset: None,
                    evidence,
                },
            )?;
        }
    }
    rows.sort_by(|left, right| {
        (
            left.asset.map(|asset| asset.id.as_str()).unwrap_or(""),
            left.evidence.observed_at.as_str(),
            left.evidence.id.as_str(),
        )
            .cmp(&(
                right.asset.map(|asset| asset.id.as_str()).unwrap_or(""),
                right.evidence.observed_at.as_str(),
                right.evidence.id.as_str(),
            ))
    });

    let mut csv = String::new();
    write_row(&mut csv, HEADERS.iter().copied());
    for row in &rows {
        let asset = row.asset;
        let values = [
            asset.map(|value| value.id.as_str()).unwrap_or(""),
            asset.map(|value| asset_kind_name(value.kind)).unwrap_or(""),
            asset.map(|value| value.key.as_str()).unwrap_or(""),
            asset
                .and_then(|value| value.qualification.as_deref())
                .unwrap_or(""),
            row.evidence.id.as_str(),
            row.evidence.source_event_id.as_str(),
            row.evidence.event_type.as_str(),
            row.evidence.observed_at.as_str(),
            row.evidence.locator.as_str(),
            row.evidence.parent_evidence_id.as_deref().unwrap_or(""),
            if row.evidence.redacted {
                "true"
            } else {
                "false"
            },
        ];
        write_row(&mut csv, values);
        if csv.len() > MAX_EXPORT_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La exportación CSV CHEF supera el máximo de bytes",
            ));
        }
    }

    Ok(ChefEvidenceCsvExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        file_name: format!("chef-evidencia-{}.csv", run_id.replace(':', "-")),
        content_type: "text/csv; charset=utf-8",
        scope_status: "not_asserted",
        row_count: rows.len(),
        csv,
    })
}

fn push_bounded_row<'a>(rows: &mut Vec<CsvRow<'a>>, row: CsvRow<'a>) -> ChefResult<()> {
    if rows.len() >= MAX_EXPORT_ROWS {
        return Err(row_limit_error());
    }
    rows.push(row);
    Ok(())
}

fn row_limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "La exportación CSV CHEF supera el máximo de filas",
    )
}

fn validate_run_id(run_id: &str) -> ChefResult<()> {
    if !run_id.starts_with("run:")
        || run_id.len() == "run:".len()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El identificador de importación CHEF no es válido",
        ));
    }
    Ok(())
}

fn asset_kind_name(kind: ReconAssetKind) -> &'static str {
    match kind {
        ReconAssetKind::Domain => "domain",
        ReconAssetKind::IpAddress => "ip_address",
        ReconAssetKind::Service => "service",
        ReconAssetKind::WebEndpoint => "web_endpoint",
        ReconAssetKind::Technology => "technology",
        ReconAssetKind::Certificate => "certificate",
        ReconAssetKind::CandidateFinding => "candidate_finding",
        ReconAssetKind::Organization => "organization",
    }
}

fn write_row<'a>(output: &mut String, values: impl IntoIterator<Item = &'a str>) {
    let mut first = true;
    for value in values {
        if !first {
            output.push(',');
        }
        first = false;
        output.push('"');
        let spreadsheet_safe = neutralize_formula(value);
        for character in spreadsheet_safe.chars() {
            if character == '"' {
                output.push_str("\"\"");
            } else {
                output.push(character);
            }
        }
        output.push('"');
    }
    output.push_str("\r\n");
}

fn neutralize_formula(value: &str) -> String {
    let first_non_whitespace = value
        .chars()
        .find(|character| !character.is_whitespace() && *character != '\u{feff}');
    if first_non_whitespace.is_some_and(|character| matches!(character, '=' | '+' | '-' | '@')) {
        format!("'{value}")
    } else {
        value.to_owned()
    }
}
