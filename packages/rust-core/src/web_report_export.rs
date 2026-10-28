//! Informe HTML local de evidencia web proyectada al grafo CHEF.
//!
//! Genera HTML estático y autocontenido a partir de activos y relaciones
//! validados. No incluye payloads HTTP, carga recursos remotos ni ejecuta JS.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{
    ChefError, ChefReconGraph, ChefResult, ErrorCode, ReconAsset, ReconAssetKind,
    ReconRelationshipKind, validate_chef_recon_graph,
};

const MAX_EXPORT_BYTES: usize = 2 * 1024 * 1024;
const MAX_REPORT_ASSETS: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefWebEvidenceReportExport {
    pub schema_version: &'static str,
    pub run_id: String,
    pub file_name: String,
    pub content_type: &'static str,
    pub scope_status: &'static str,
    pub endpoint_count: usize,
    pub candidate_finding_count: usize,
    pub html: String,
}

/// Construye un informe web CHEF local con relaciones y evidencia explícitas.
/// Las tecnologías y los hallazgos solo se asocian al host del grafo, no a una
/// URL individual, porque CHEF no conserva la cadena de ancestros del origen.
pub fn export_web_evidence_report(
    run_id: &str,
    graph: &ChefReconGraph,
) -> ChefResult<ChefWebEvidenceReportExport> {
    validate_run_id(run_id)?;
    validate_chef_recon_graph(graph)?;
    if graph.assets.len() > MAX_REPORT_ASSETS {
        return Err(limit_error());
    }

    let assets = graph
        .assets
        .iter()
        .map(|asset| (asset.id.as_str(), asset))
        .collect::<BTreeMap<_, _>>();
    let evidence = graph
        .evidence
        .iter()
        .map(|item| (item.id.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let endpoints = graph
        .assets
        .iter()
        .filter(|asset| asset.kind == ReconAssetKind::WebEndpoint)
        .collect::<Vec<_>>();
    let mut hosts_by_endpoint = BTreeMap::<&str, Vec<&ReconAsset>>::new();
    for relation in graph
        .relationships
        .iter()
        .filter(|relation| relation.kind == ReconRelationshipKind::ServesEndpoint)
    {
        if let Some(host) = assets.get(relation.from_asset_id.as_str()) {
            hosts_by_endpoint
                .entry(relation.to_asset_id.as_str())
                .or_default()
                .push(*host);
        }
    }
    let web_hosts = hosts_by_endpoint
        .values()
        .flatten()
        .filter(|asset| {
            matches!(
                asset.kind,
                ReconAssetKind::Domain | ReconAssetKind::IpAddress
            )
        })
        .map(|asset| asset.id.as_str())
        .collect::<BTreeSet<_>>();
    let candidate_finding_count = graph
        .relationships
        .iter()
        .filter(|relation| {
            relation.kind == ReconRelationshipKind::ReportsCandidate
                && web_hosts.contains(relation.from_asset_id.as_str())
        })
        .count();

    let mut html = String::with_capacity(16_384);
    html.push_str("<!doctype html><html lang=\"es\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str("<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">");
    html.push_str("<title>Informe de evidencia web CHEF</title><style>");
    html.push_str("body{font:16px system-ui,sans-serif;max-width:1100px;margin:2rem auto;padding:0 1rem;color:#172033;background:#f8fafc}h1,h2{color:#102a43}table{width:100%;border-collapse:collapse;margin:1rem 0 2rem;background:white}th,td{text-align:left;vertical-align:top;padding:.65rem;border:1px solid #cbd5e1;overflow-wrap:anywhere}th{background:#e2e8f0}code{font-size:.9em}.note{padding:1rem;border-left:4px solid #2563eb;background:#eff6ff}.muted{color:#475569}</style></head><body>");
    html.push_str("<h1>Informe de evidencia web CHEF</h1><p>Importación: <code>");
    html.push_str(&escape_html(run_id));
    html.push_str("</code></p><p class=\"note\">El informe resume observaciones y relaciones conservadas localmente. No afirma alcance, propiedad ni vulnerabilidades confirmadas. Los hallazgos se muestran como candidatos.</p>");
    html.push_str("<p class=\"muted\">URLs y tecnologías proceden del grafo CHEF. No se incluyen cuerpos, cabeceras, rutas importadas, parámetros ni payloads HTTP.</p>");

    html.push_str("<h2>Endpoints web</h2><table><thead><tr><th>Host relacionado</th><th>Endpoint</th><th>Calificación</th><th>Evidencia</th></tr></thead><tbody>");
    for endpoint in &endpoints {
        html.push_str("<tr><td>");
        let mut hosts = hosts_by_endpoint
            .get(endpoint.id.as_str())
            .cloned()
            .unwrap_or_default();
        hosts.sort_by(|left, right| left.key.cmp(&right.key));
        hosts.dedup_by(|left, right| left.id == right.id);
        if hosts.is_empty() {
            html.push_str("Sin relación de host conservada");
        } else {
            html.push_str(
                &hosts
                    .iter()
                    .map(|host| escape_html(&host.key))
                    .collect::<Vec<_>>()
                    .join("<br>"),
            );
        }
        html.push_str("</td><td><code>");
        html.push_str(&escape_html(&endpoint.key));
        html.push_str("</code></td><td>");
        html.push_str(&escape_html(
            endpoint
                .qualification
                .as_deref()
                .unwrap_or("sin calificación"),
        ));
        html.push_str("</td><td>");
        render_evidence(&mut html, &endpoint.evidence_ids, &evidence);
        html.push_str("</td></tr>");
        if html.len() > MAX_EXPORT_BYTES {
            return Err(limit_error());
        }
    }
    if endpoints.is_empty() {
        html.push_str("<tr><td colspan=\"4\">No hay endpoints web proyectados para esta importación.</td></tr>");
    }
    html.push_str("</tbody></table>");

    html.push_str("<h2>Tecnologías relacionadas con hosts web</h2><table><thead><tr><th>Host</th><th>Tecnología</th><th>Evidencia</th></tr></thead><tbody>");
    let technology_rows = graph
        .relationships
        .iter()
        .filter(|relation| {
            relation.kind == ReconRelationshipKind::UsesTechnology
                && web_hosts.contains(relation.from_asset_id.as_str())
        })
        .collect::<Vec<_>>();
    for relation in &technology_rows {
        render_related_asset_row(&mut html, relation, &assets, &evidence);
    }
    if technology_rows.is_empty() {
        html.push_str(
            "<tr><td colspan=\"3\">No hay tecnologías relacionadas conservadas.</td></tr>",
        );
    }
    html.push_str("</tbody></table>");

    html.push_str("<h2>Hallazgos candidatos relacionados con hosts web</h2><table><thead><tr><th>Host</th><th>ID de candidato</th><th>Estado</th><th>Evidencia</th></tr></thead><tbody>");
    let finding_rows = graph
        .relationships
        .iter()
        .filter(|relation| {
            relation.kind == ReconRelationshipKind::ReportsCandidate
                && web_hosts.contains(relation.from_asset_id.as_str())
        })
        .collect::<Vec<_>>();
    for relation in &finding_rows {
        let Some(host) = assets.get(relation.from_asset_id.as_str()) else {
            continue;
        };
        let Some(candidate) = assets.get(relation.to_asset_id.as_str()) else {
            continue;
        };
        html.push_str("<tr><td><code>");
        html.push_str(&escape_html(&host.key));
        html.push_str("</code></td><td><code>");
        html.push_str(&escape_html(&candidate.id));
        html.push_str("</code></td><td>candidato; no confirmado</td><td>");
        render_evidence(&mut html, &relation.evidence_ids, &evidence);
        html.push_str("</td></tr>");
    }
    if finding_rows.is_empty() {
        html.push_str(
            "<tr><td colspan=\"4\">No hay hallazgos candidatos relacionados conservados.</td></tr>",
        );
    }
    html.push_str("</tbody></table><footer class=\"muted\">Generado localmente por CHEF. scopeStatus: not_asserted.</footer></body></html>");

    if html.len() > MAX_EXPORT_BYTES {
        return Err(limit_error());
    }
    Ok(ChefWebEvidenceReportExport {
        schema_version: "1.0.0",
        run_id: run_id.to_owned(),
        file_name: format!("chef-informe-web-{}.html", run_id.replace(':', "-")),
        content_type: "text/html; charset=utf-8",
        scope_status: "not_asserted",
        endpoint_count: endpoints.len(),
        candidate_finding_count,
        html,
    })
}

fn render_related_asset_row(
    html: &mut String,
    relation: &crate::ReconRelationship,
    assets: &BTreeMap<&str, &ReconAsset>,
    evidence: &BTreeMap<&str, &crate::ReconEvidence>,
) {
    let (Some(host), Some(asset)) = (
        assets.get(relation.from_asset_id.as_str()),
        assets.get(relation.to_asset_id.as_str()),
    ) else {
        return;
    };
    html.push_str("<tr><td><code>");
    html.push_str(&escape_html(&host.key));
    html.push_str("</code></td><td><code>");
    html.push_str(&escape_html(&asset.key));
    html.push_str("</code></td><td>");
    render_evidence(html, &relation.evidence_ids, evidence);
    html.push_str("</td></tr>");
}

fn render_evidence(
    html: &mut String,
    evidence_ids: &[String],
    evidence: &BTreeMap<&str, &crate::ReconEvidence>,
) {
    if evidence_ids.is_empty() {
        html.push_str("Sin evidencia vinculada");
        return;
    }
    html.push_str("<ul>");
    for id in evidence_ids {
        html.push_str("<li><code>");
        html.push_str(&escape_html(id));
        if let Some(item) = evidence.get(id.as_str()) {
            html.push_str("</code> — ");
            html.push_str(&escape_html(&item.event_type));
            html.push_str(" — ");
            html.push_str(&escape_html(&item.observed_at));
            if let Some(parent) = item.parent_evidence_id.as_deref() {
                html.push_str(" — linaje: <code>");
                html.push_str(&escape_html(parent));
                html.push_str("</code>");
            }
        } else {
            html.push_str("</code>");
        }
        html.push_str("</li>");
    }
    html.push_str("</ul>");
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
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

fn limit_error() -> ChefError {
    ChefError::new(
        ErrorCode::LimitExceeded,
        "El informe web CHEF supera el máximo de activos o bytes",
    )
}
