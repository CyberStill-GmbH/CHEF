//! Adaptación funcional de eventos BBOT al modelo de reconocimiento CHEF.
//!
//! El grafo CHEF solo crea activos tipados y relaciones respaldadas por una
//! observación concreta. Los eventos originales se conservan aparte para
//! auditoría y para ampliar el mapeo sin perder información de origen.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{BbotEvent, BbotEventGraph, ChefError, ChefResult, ErrorCode, identity};

const MAX_ASSETS: usize = 20_000;
const MAX_EVIDENCE: usize = 10_000;
const MAX_RELATIONSHIPS: usize = 40_000;
const MAX_KEY_BYTES: usize = 2_048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconAssetKind {
    Domain,
    IpAddress,
    Service,
    WebEndpoint,
    Technology,
    Certificate,
    CandidateFinding,
    Organization,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReconAsset {
    pub id: String,
    pub kind: ReconAssetKind,
    pub key: String,
    /// Estado de verificación de URLs; `candidate` evita presentar hallazgos
    /// como vulnerabilidades confirmadas.
    pub qualification: Option<String>,
    pub first_seen: String,
    pub last_seen: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReconEvidence {
    pub id: String,
    pub source_event_id: String,
    pub parent_evidence_id: Option<String>,
    pub event_type: String,
    pub observed_at: String,
    pub locator: String,
    pub module: Option<String>,
    pub redacted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReconRelationshipKind {
    ResolvesTo,
    ParentDomainOf,
    ExposesService,
    ServesEndpoint,
    UsesTechnology,
    PresentsCertificate,
    ReportsCandidate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ReconRelationship {
    pub id: String,
    pub from_asset_id: String,
    pub to_asset_id: String,
    pub kind: ReconRelationshipKind,
    pub assertion: String,
    pub evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefReconGraph {
    pub schema_version: String,
    pub assets: Vec<ReconAsset>,
    pub evidence: Vec<ReconEvidence>,
    pub relationships: Vec<ReconRelationship>,
    /// Conteo explícito de tipos preservados en el archivo original pero aún
    /// sin regla funcional CHEF. No equivale a eventos descartados.
    pub unmapped_event_types: BTreeMap<String, usize>,
}

/// Proyecta los eventos BBOT útiles para reconocimiento CHEF a activos,
/// evidencias y relaciones concretas. Los tipos aún no adaptados quedan
/// contabilizados y siguen disponibles en el grafo fuente.
pub fn adapt_bbot_to_chef(graph: &BbotEventGraph) -> ChefResult<ChefReconGraph> {
    crate::validate_bbot_event_graph(graph)?;
    if graph.events.len() > MAX_EVIDENCE {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La importación BBOT supera el límite de evidencia CHEF",
        ));
    }

    let mut assets = BTreeMap::<String, ReconAsset>::new();
    let mut relationships = BTreeMap::<String, ReconRelationship>::new();
    let mut evidence = Vec::with_capacity(graph.events.len());
    let mut unmapped_event_types = BTreeMap::<String, usize>::new();
    let events_by_id = graph
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect::<BTreeMap<_, _>>();

    for event in &graph.events {
        let evidence_id = identity::id("chef-evidence", &[&event.id]);
        let evidence_item = ReconEvidence {
            id: evidence_id.clone(),
            source_event_id: event.id.clone(),
            parent_evidence_id: event
                .parent_id
                .as_deref()
                .map(|parent| identity::id("chef-evidence", &[parent])),
            event_type: event.event_type.clone(),
            observed_at: event.timestamp.clone(),
            locator: event.locator.clone(),
            module: event.module.clone(),
            redacted: event.data_redacted || event.metadata_redacted,
        };
        evidence.push(evidence_item);

        let mapped = match event.event_type.as_str() {
            "DNS_NAME" | "DNS_NAME_UNRESOLVED" => {
                add_named_asset(&mut assets, ReconAssetKind::Domain, event, &evidence_id)
            }
            "DNS_NAME_HINT"
                if crate::dns_mutation_candidates::is_generated_dns_mutation_hint(event)
                    || crate::speculate::is_generated_dns_rdata_hint(event) =>
            {
                add_named_asset(&mut assets, ReconAssetKind::Domain, event, &evidence_id)
            }
            "IP_ADDRESS" => {
                add_named_asset(&mut assets, ReconAssetKind::IpAddress, event, &evidence_id)
            }
            "OPEN_TCP_PORT" => add_service(&mut assets, event, &evidence_id, "tcp", false),
            "TCP_PORT_HINT" => add_service(&mut assets, event, &evidence_id, "tcp", true),
            "OPEN_UDP_PORT" => add_service(&mut assets, event, &evidence_id, "udp", false),
            "URL" => add_url(&mut assets, event, &evidence_id, "observed"),
            "HTTP_RESPONSE" => add_url(&mut assets, event, &evidence_id, "observed"),
            "URL_UNVERIFIED" => add_url(&mut assets, event, &evidence_id, "unverified"),
            "URL_HINT" => add_url(&mut assets, event, &evidence_id, "hint"),
            "TECHNOLOGY" => {
                add_named_asset(&mut assets, ReconAssetKind::Technology, event, &evidence_id)
            }
            "CERTIFICATE" => add_named_asset(
                &mut assets,
                ReconAssetKind::Certificate,
                event,
                &evidence_id,
            ),
            "FINDING" => add_candidate(&mut assets, event, &evidence_id),
            "ORG_STUB" => add_named_asset(
                &mut assets,
                ReconAssetKind::Organization,
                event,
                &evidence_id,
            ),
            _ => false,
        };
        if !mapped {
            *unmapped_event_types
                .entry(event.event_type.clone())
                .or_default() += 1;
        }

        if mapped
            && crate::dns_mutation_candidates::is_generated_dns_mutation_hint(event)
            && let Some(name) =
                event_value(event).and_then(|value| crate::host::canonical_domain(&value))
            && let Some(source_ids) = event
                .metadata
                .get("sourceEventIds")
                .and_then(Value::as_array)
        {
            let id = asset_id(ReconAssetKind::Domain, &name);
            if let Some(asset) = assets.get_mut(&id) {
                for source_id in source_ids.iter().filter_map(Value::as_str) {
                    if events_by_id.contains_key(source_id) {
                        let source_evidence_id = identity::id("chef-evidence", &[source_id]);
                        if !asset.evidence_ids.contains(&source_evidence_id) {
                            asset.evidence_ids.push(source_evidence_id);
                        }
                    }
                }
                asset.evidence_ids.sort();
            }
        }

        if mapped
            && crate::speculate::is_generated_dns_rdata_hint(event)
            && let Some(parent_id) = event.parent_id.as_deref()
            && events_by_id.contains_key(parent_id)
            && let Some(name) =
                event_value(event).and_then(|value| crate::host::canonical_domain(&value))
        {
            let asset_key = asset_id(ReconAssetKind::Domain, &name);
            let source_evidence_id = identity::id("chef-evidence", &[parent_id]);
            if let Some(asset) = assets.get_mut(&asset_key)
                && !asset.evidence_ids.contains(&source_evidence_id)
            {
                asset.evidence_ids.push(source_evidence_id);
                asset.evidence_ids.sort();
            }
        }

        if mapped
            && crate::speculate::is_generated_bimi_url_hint(event)
            && let Some(parent_id) = event.parent_id.as_deref()
            && events_by_id.contains_key(parent_id)
            && let Some(url) =
                event_value(event).and_then(|value| crate::url_tools::canonical_http_url(&value))
        {
            let asset_key = asset_id(ReconAssetKind::WebEndpoint, &url);
            let source_evidence_id = identity::id("chef-evidence", &[parent_id]);
            if let Some(asset) = assets.get_mut(&asset_key)
                && !asset.evidence_ids.contains(&source_evidence_id)
            {
                asset.evidence_ids.push(source_evidence_id);
                asset.evidence_ids.sort();
            }
        }

        if event
            .tags
            .iter()
            .any(|tag| tag == "chef:derived-parent-domain-hint")
            && let Some(parent_id) = event.parent_id.as_deref()
            && let Some(parent_event) = events_by_id.get(parent_id)
            && let (Some(child), Some(parent)) = (
                event_value(parent_event).and_then(|value| crate::host::canonical_domain(&value)),
                event_value(event).and_then(|value| crate::host::canonical_domain(&value)),
            )
        {
            add_relationship(
                &mut relationships,
                asset_id(ReconAssetKind::Domain, &child),
                asset_id(ReconAssetKind::Domain, &parent),
                ReconRelationshipKind::ParentDomainOf,
                "hint",
                &evidence_id,
            );
        }

        let Some(host_id) = add_observed_host(&mut assets, event, &evidence_id) else {
            continue;
        };
        match event.event_type.as_str() {
            "OPEN_TCP_PORT" | "OPEN_UDP_PORT" | "TCP_PORT_HINT" => {
                if let Some(service_id) = find_asset_for_event(&assets, event, &evidence_id) {
                    add_relationship(
                        &mut relationships,
                        host_id,
                        service_id,
                        ReconRelationshipKind::ExposesService,
                        if event.event_type == "TCP_PORT_HINT" {
                            "hint"
                        } else {
                            "observed"
                        },
                        &evidence_id,
                    );
                }
            }
            "URL" | "URL_UNVERIFIED" | "URL_HINT" | "HTTP_RESPONSE" => {
                if let Some(endpoint_id) = find_asset_for_event(&assets, event, &evidence_id) {
                    add_relationship(
                        &mut relationships,
                        host_id,
                        endpoint_id,
                        ReconRelationshipKind::ServesEndpoint,
                        qualification_for_url(&event.event_type),
                        &evidence_id,
                    );
                }
            }
            "TECHNOLOGY" => {
                if let Some(technology_id) = find_asset_for_event(&assets, event, &evidence_id) {
                    add_relationship(
                        &mut relationships,
                        host_id,
                        technology_id,
                        ReconRelationshipKind::UsesTechnology,
                        "observed",
                        &evidence_id,
                    );
                }
            }
            "CERTIFICATE" => {
                if let Some(certificate_id) = find_asset_for_event(&assets, event, &evidence_id) {
                    add_relationship(
                        &mut relationships,
                        host_id,
                        certificate_id,
                        ReconRelationshipKind::PresentsCertificate,
                        "observed",
                        &evidence_id,
                    );
                }
            }
            "FINDING" => {
                if let Some(finding_id) = find_asset_for_event(&assets, event, &evidence_id) {
                    add_relationship(
                        &mut relationships,
                        host_id,
                        finding_id,
                        ReconRelationshipKind::ReportsCandidate,
                        "candidate",
                        &evidence_id,
                    );
                }
            }
            _ => {}
        }

        if event.event_type == "DNS_NAME" || event.event_type == "DNS_NAME_UNRESOLVED" {
            add_explicit_dns_resolutions(&mut assets, &mut relationships, event, &evidence_id);
        }
    }

    let projected = ChefReconGraph {
        schema_version: "1.4.0".to_owned(),
        assets: assets.into_values().collect(),
        evidence,
        relationships: relationships.into_values().collect(),
        unmapped_event_types,
    };
    validate_chef_recon_graph(&projected)?;
    Ok(projected)
}

pub fn validate_chef_recon_graph(graph: &ChefReconGraph) -> ChefResult<()> {
    if !matches!(
        graph.schema_version.as_str(),
        "1.0.0" | "1.1.0" | "1.2.0" | "1.3.0" | "1.4.0"
    ) || graph.evidence.is_empty()
        || graph.assets.len() > MAX_ASSETS
        || graph.evidence.len() > MAX_EVIDENCE
        || graph.relationships.len() > MAX_RELATIONSHIPS
        || graph.unmapped_event_types.len() > 512
        || serde_json::to_vec(graph)
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Grafo CHEF inválido"))?
            .len()
            > 8 * 1024 * 1024
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Grafo de reconocimiento CHEF fuera de límites o versión no compatible",
        ));
    }
    let evidence_ids = graph
        .evidence
        .iter()
        .map(|item| item.id.as_str())
        .collect::<BTreeSet<_>>();
    let asset_ids = graph
        .assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<BTreeSet<_>>();
    let relationship_ids = graph
        .relationships
        .iter()
        .map(|relationship| relationship.id.as_str())
        .collect::<BTreeSet<_>>();
    if evidence_ids.len() != graph.evidence.len()
        || asset_ids.len() != graph.assets.len()
        || relationship_ids.len() != graph.relationships.len()
        || graph.evidence.iter().any(|item| {
            item.id != identity::id("chef-evidence", &[&item.source_event_id])
                || item.source_event_id.is_empty()
                || item
                    .parent_evidence_id
                    .as_ref()
                    .is_some_and(|id| !evidence_ids.contains(id.as_str()))
                || item.event_type.is_empty()
                || item.event_type.len() > 64
                || item.observed_at.is_empty()
                || item.locator.len() > 256
                || item
                    .module
                    .as_ref()
                    .is_some_and(|module| module.len() > 128)
        })
        || graph.assets.iter().any(|asset| {
            asset.id != identity::id("chef-recon-asset", &[kind_key(asset.kind), &asset.key])
                || asset.key.is_empty()
                || asset.key.len() > MAX_KEY_BYTES
                || !valid_qualification(asset.kind, asset.qualification.as_deref())
                || !qualification_allowed_for_version(graph, asset)
                || asset.evidence_ids.is_empty()
                || asset.evidence_ids.iter().collect::<BTreeSet<_>>().len()
                    != asset.evidence_ids.len()
                || asset
                    .evidence_ids
                    .iter()
                    .any(|id| !evidence_ids.contains(id.as_str()))
        })
        || graph.relationships.iter().any(|relationship| {
            !asset_ids.contains(relationship.from_asset_id.as_str())
                || !asset_ids.contains(relationship.to_asset_id.as_str())
                || relationship.evidence_ids.is_empty()
                || !valid_assertion(relationship.kind, &relationship.assertion)
                || relationship
                    .evidence_ids
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != relationship.evidence_ids.len()
                || relationship
                    .evidence_ids
                    .iter()
                    .any(|id| !evidence_ids.contains(id.as_str()))
                || relationship.id
                    != identity::id(
                        "chef-recon-relationship",
                        &[
                            kind_relation_key(relationship.kind),
                            &relationship.from_asset_id,
                            &relationship.to_asset_id,
                            &relationship.assertion,
                        ],
                    )
        })
        || graph
            .unmapped_event_types
            .iter()
            .any(|(event_type, count)| {
                event_type.is_empty() || event_type.len() > 64 || *count == 0
            })
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Activos, evidencia o relaciones CHEF no son coherentes",
        ));
    }
    Ok(())
}

fn add_named_asset(
    assets: &mut BTreeMap<String, ReconAsset>,
    kind: ReconAssetKind,
    event: &BbotEvent,
    evidence_id: &str,
) -> bool {
    let Some(key) = event_value(event) else {
        return false;
    };
    let key = match kind {
        ReconAssetKind::Domain => match crate::host::canonical_domain(&key) {
            Some(value) => value,
            None => return false,
        },
        ReconAssetKind::IpAddress => match key.parse::<std::net::IpAddr>() {
            Ok(value) => value.to_string(),
            Err(_) => return false,
        },
        ReconAssetKind::Organization => {
            let key = key.trim().to_lowercase();
            if key.chars().any(char::is_control) {
                return false;
            }
            key
        }
        _ => key.trim().to_owned(),
    };
    if key.is_empty() || key.len() > MAX_KEY_BYTES {
        return false;
    }
    let qualification = named_asset_qualification(kind, event);
    upsert_asset(assets, kind, key, qualification, event, evidence_id);
    true
}

fn named_asset_qualification(kind: ReconAssetKind, event: &BbotEvent) -> Option<String> {
    let qualification = match kind {
        ReconAssetKind::Domain
            if crate::dns_mutation_candidates::is_generated_dns_mutation_hint(event)
                || crate::speculate::is_generated_dns_rdata_hint(event) =>
        {
            Some("hint")
        }
        ReconAssetKind::Domain if event.event_type == "URL_HINT" => Some("hint"),
        ReconAssetKind::Domain if event.event_type == "DNS_NAME_UNRESOLVED" => Some(
            if event.module.as_deref() == Some("chef_dns_parent_hints")
                && event
                    .tags
                    .iter()
                    .any(|tag| tag == "chef:derived-parent-domain-hint")
            {
                "hint"
            } else {
                "unresolved"
            },
        ),
        ReconAssetKind::IpAddress
            if event.module.as_deref() == Some("chef_ip_range_expansion")
                && event
                    .tags
                    .iter()
                    .any(|tag| tag == "chef:derived-ip-range-member") =>
        {
            Some("hint")
        }
        ReconAssetKind::Organization if event.event_type == "ORG_STUB" => Some("hint"),
        _ => None,
    };
    qualification.map(str::to_owned)
}

fn add_service(
    assets: &mut BTreeMap<String, ReconAsset>,
    event: &BbotEvent,
    evidence_id: &str,
    protocol: &str,
    is_hint: bool,
) -> bool {
    let (Some(host), Some(port)) = (event.host.as_deref().and_then(canonical_host), event.port)
    else {
        return false;
    };
    let key = format!("{host}|{protocol}|{port}");
    upsert_asset(
        assets,
        ReconAssetKind::Service,
        key.clone(),
        Some(if is_hint { "hint" } else { "observed_open" }.to_owned()),
        event,
        evidence_id,
    );
    if !is_hint && let Some(asset) = assets.get_mut(&asset_id(ReconAssetKind::Service, &key)) {
        asset.qualification = Some("observed_open".to_owned());
    }
    true
}

fn add_url(
    assets: &mut BTreeMap<String, ReconAsset>,
    event: &BbotEvent,
    evidence_id: &str,
    qualification: &str,
) -> bool {
    let Some(value) = event_value(event) else {
        return false;
    };
    if value.len() > MAX_KEY_BYTES {
        return false;
    }
    let Some(url) = crate::url_tools::canonical_http_url(value.trim()) else {
        return false;
    };
    upsert_asset(
        assets,
        ReconAssetKind::WebEndpoint,
        url,
        Some(qualification.to_owned()),
        event,
        evidence_id,
    );
    true
}

fn add_candidate(
    assets: &mut BTreeMap<String, ReconAsset>,
    event: &BbotEvent,
    evidence_id: &str,
) -> bool {
    let Ok(data) = serde_json::to_vec(&event.data) else {
        return false;
    };
    let key = identity::digest(&data);
    upsert_asset(
        assets,
        ReconAssetKind::CandidateFinding,
        key,
        Some("candidate".to_owned()),
        event,
        evidence_id,
    );
    true
}

fn add_observed_host(
    assets: &mut BTreeMap<String, ReconAsset>,
    event: &BbotEvent,
    evidence_id: &str,
) -> Option<String> {
    let host = if event.event_type == "HTTP_RESPONSE" {
        let url = event_value(event)?;
        let parsed = url::Url::parse(&url).ok()?;
        canonical_host(parsed.host_str()?)?
    } else {
        event.host.as_deref().and_then(canonical_host)?
    };
    let kind = if host.parse::<std::net::IpAddr>().is_ok() {
        ReconAssetKind::IpAddress
    } else {
        ReconAssetKind::Domain
    };
    let id = asset_id(kind, &host);
    upsert_asset(
        assets,
        kind,
        host,
        named_asset_qualification(kind, event),
        event,
        evidence_id,
    );
    Some(id)
}

fn add_explicit_dns_resolutions(
    assets: &mut BTreeMap<String, ReconAsset>,
    relationships: &mut BTreeMap<String, ReconRelationship>,
    event: &BbotEvent,
    evidence_id: &str,
) {
    let Some(domain) = event_value(event).and_then(|value| crate::host::canonical_domain(&value))
    else {
        return;
    };
    let Some(hosts) = event
        .metadata
        .get("resolved_hosts")
        .and_then(Value::as_array)
    else {
        return;
    };
    let domain_id = asset_id(ReconAssetKind::Domain, &domain);
    upsert_asset(
        assets,
        ReconAssetKind::Domain,
        domain,
        None,
        event,
        evidence_id,
    );
    for value in hosts.iter().filter_map(Value::as_str) {
        let Some(address) = value
            .parse::<std::net::IpAddr>()
            .ok()
            .map(|address| address.to_string())
        else {
            continue;
        };
        let address_id = asset_id(ReconAssetKind::IpAddress, &address);
        upsert_asset(
            assets,
            ReconAssetKind::IpAddress,
            address,
            None,
            event,
            evidence_id,
        );
        add_relationship(
            relationships,
            domain_id.clone(),
            address_id,
            ReconRelationshipKind::ResolvesTo,
            "observed",
            evidence_id,
        );
    }
}

fn find_asset_for_event(
    assets: &BTreeMap<String, ReconAsset>,
    event: &BbotEvent,
    evidence_id: &str,
) -> Option<String> {
    assets.values().find_map(|asset| {
        asset
            .evidence_ids
            .contains(&evidence_id.to_owned())
            .then(|| asset.id.clone())
            .filter(|_| match event.event_type.as_str() {
                "OPEN_TCP_PORT" | "OPEN_UDP_PORT" | "TCP_PORT_HINT" => {
                    asset.kind == ReconAssetKind::Service
                }
                "URL" | "URL_UNVERIFIED" | "URL_HINT" => asset.kind == ReconAssetKind::WebEndpoint,
                "TECHNOLOGY" => asset.kind == ReconAssetKind::Technology,
                "CERTIFICATE" => asset.kind == ReconAssetKind::Certificate,
                "FINDING" => asset.kind == ReconAssetKind::CandidateFinding,
                _ => false,
            })
    })
}

fn event_value(event: &BbotEvent) -> Option<String> {
    match &event.data {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Object(object) => [
            "name",
            "domain",
            "hostname",
            "address",
            "ip",
            "url",
            "technology",
            "fingerprint",
            "sha256",
            "subject",
            "title",
        ]
        .iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str).map(str::to_owned)),
        _ => None,
    }
}

fn upsert_asset(
    assets: &mut BTreeMap<String, ReconAsset>,
    kind: ReconAssetKind,
    key: String,
    qualification: Option<String>,
    event: &BbotEvent,
    evidence_id: &str,
) {
    let id = asset_id(kind, &key);
    let asset = assets.entry(id.clone()).or_insert_with(|| ReconAsset {
        id,
        kind,
        key,
        qualification: qualification.clone(),
        first_seen: event.timestamp.clone(),
        last_seen: event.timestamp.clone(),
        evidence_ids: Vec::new(),
    });
    if qualification_rank(kind, qualification.as_deref())
        > qualification_rank(kind, asset.qualification.as_deref())
    {
        asset.qualification = qualification;
    }
    if event.timestamp < asset.first_seen {
        asset.first_seen = event.timestamp.clone();
    }
    if event.timestamp > asset.last_seen {
        asset.last_seen = event.timestamp.clone();
    }
    if !asset.evidence_ids.iter().any(|id| id == evidence_id) {
        asset.evidence_ids.push(evidence_id.to_owned());
    }
}

/// Fusiona calificaciones sin dejar que una pista rebaje una observación
/// directa, ni que el orden de los eventos cambie el resultado final.
fn qualification_rank(kind: ReconAssetKind, qualification: Option<&str>) -> u8 {
    match (kind, qualification) {
        (ReconAssetKind::Domain, None) => 3,
        (ReconAssetKind::Domain, Some("unresolved")) => 2,
        (ReconAssetKind::Domain, Some("hint")) => 1,
        (ReconAssetKind::IpAddress, None) => 2,
        (ReconAssetKind::IpAddress, Some("hint")) => 1,
        (ReconAssetKind::Service, Some("observed_open")) => 2,
        (ReconAssetKind::Service, Some("hint")) => 1,
        (ReconAssetKind::WebEndpoint, Some("observed")) => 3,
        (ReconAssetKind::WebEndpoint, Some("unverified")) => 2,
        (ReconAssetKind::WebEndpoint, Some("hint")) => 1,
        (ReconAssetKind::CandidateFinding, Some("candidate")) => 1,
        (ReconAssetKind::Organization, Some("hint")) => 1,
        _ => 0,
    }
}

fn add_relationship(
    relationships: &mut BTreeMap<String, ReconRelationship>,
    from: String,
    to: String,
    kind: ReconRelationshipKind,
    assertion: &str,
    evidence_id: &str,
) {
    let id = identity::id(
        "chef-recon-relationship",
        &[kind_relation_key(kind), &from, &to, assertion],
    );
    let relationship = relationships
        .entry(id.clone())
        .or_insert(ReconRelationship {
            id,
            from_asset_id: from,
            to_asset_id: to,
            kind,
            assertion: assertion.to_owned(),
            evidence_ids: Vec::new(),
        });
    if !relationship.evidence_ids.iter().any(|id| id == evidence_id) {
        relationship.evidence_ids.push(evidence_id.to_owned());
    }
}

fn asset_id(kind: ReconAssetKind, key: &str) -> String {
    identity::id("chef-recon-asset", &[kind_key(kind), key])
}

fn kind_key(kind: ReconAssetKind) -> &'static str {
    match kind {
        ReconAssetKind::Domain => "domain",
        ReconAssetKind::IpAddress => "ip-address",
        ReconAssetKind::Service => "service",
        ReconAssetKind::WebEndpoint => "web-endpoint",
        ReconAssetKind::Technology => "technology",
        ReconAssetKind::Certificate => "certificate",
        ReconAssetKind::CandidateFinding => "candidate-finding",
        ReconAssetKind::Organization => "organization",
    }
}

fn kind_relation_key(kind: ReconRelationshipKind) -> &'static str {
    match kind {
        ReconRelationshipKind::ResolvesTo => "resolves-to",
        ReconRelationshipKind::ParentDomainOf => "parent-domain-of",
        ReconRelationshipKind::ExposesService => "exposes-service",
        ReconRelationshipKind::ServesEndpoint => "serves-endpoint",
        ReconRelationshipKind::UsesTechnology => "uses-technology",
        ReconRelationshipKind::PresentsCertificate => "presents-certificate",
        ReconRelationshipKind::ReportsCandidate => "reports-candidate",
    }
}

fn valid_qualification(kind: ReconAssetKind, qualification: Option<&str>) -> bool {
    match kind {
        ReconAssetKind::Organization => qualification == Some("hint"),
        ReconAssetKind::Domain => matches!(qualification, None | Some("hint" | "unresolved")),
        ReconAssetKind::IpAddress => matches!(qualification, None | Some("hint")),
        ReconAssetKind::Service => matches!(qualification, Some("observed_open" | "hint")),
        ReconAssetKind::WebEndpoint => {
            matches!(qualification, Some("observed" | "unverified" | "hint"))
        }
        ReconAssetKind::CandidateFinding => qualification == Some("candidate"),
        _ => qualification.is_none(),
    }
}

fn qualification_allowed_for_version(graph: &ChefReconGraph, asset: &ReconAsset) -> bool {
    if graph.schema_version == "1.4.0" {
        return true;
    }
    !matches!(asset.kind, ReconAssetKind::Organization)
        && (graph.schema_version == "1.3.0"
            || !matches!(
                (asset.kind, asset.qualification.as_deref()),
                (ReconAssetKind::Domain, Some("hint" | "unresolved"))
                    | (ReconAssetKind::IpAddress, Some("hint"))
            ))
}

fn valid_assertion(kind: ReconRelationshipKind, assertion: &str) -> bool {
    match kind {
        ReconRelationshipKind::ReportsCandidate => assertion == "candidate",
        ReconRelationshipKind::ParentDomainOf => assertion == "hint",
        ReconRelationshipKind::ExposesService => matches!(assertion, "observed" | "hint"),
        ReconRelationshipKind::ServesEndpoint => {
            matches!(assertion, "observed" | "unverified" | "hint")
        }
        _ => assertion == "observed",
    }
}

fn canonical_host(value: &str) -> Option<String> {
    crate::host::canonical_host(value)
}

fn qualification_for_url(event_type: &str) -> &'static str {
    match event_type {
        "URL" | "HTTP_RESPONSE" => "observed",
        "URL_UNVERIFIED" => "unverified",
        _ => "hint",
    }
}
