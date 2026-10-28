use std::sync::atomic::{AtomicBool, Ordering};

use chrono::{SecondsFormat, Utc};

use crate::{
    BbotEventEngine,
    config::ChefConfig,
    contracts::{
        Asset, Evidence, ExposurePath, Observation, Relationship, SCHEMA_VERSION, ScanRun,
        ScopePolicy, Snapshot,
    },
    domain::{ChefError, ChefResult, ErrorCode, ServiceInput},
    event_import, identity, nmap,
};

const INPUT_LIMIT: usize = 2 * 1024 * 1024;
const OBSERVATION_LIMIT: usize = 10_000;
const EVENT_GRAPH_BYTE_LIMIT: usize = 8 * 1024 * 1024;

pub trait Clock {
    fn now(&self) -> String;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
    }
}

pub struct FixedClock(pub String);

impl Clock for FixedClock {
    fn now(&self) -> String {
        self.0.clone()
    }
}

pub struct ImportEvidence;

impl ImportEvidence {
    /// Parse and publish one complete Nmap snapshot. Validation happens before graph creation.
    pub fn execute(
        bytes: &[u8],
        policy: &ScopePolicy,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<Snapshot> {
        check_cancelled(cancelled)?;
        let policy = normalize_policy(policy.clone())?;
        if bytes.len() > policy.max_input_bytes {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Input exceeds scope byte limit",
            ));
        }

        let parsed = nmap::parse(bytes, policy.max_observations)?;
        ImportServices::execute(
            bytes,
            parsed,
            &policy,
            clock,
            cancelled,
            EvidenceSource::NmapXml,
        )
    }
}

/// Creates a CHEF snapshot from records parsed by a source adapter.
/// Source adapters must preserve the original bytes and stable locators.
pub(crate) struct ImportServices;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EvidenceSource {
    NmapXml,
    BbotNdjson,
}

impl EvidenceSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::NmapXml => "nmap-xml",
            Self::BbotNdjson => "bbot-ndjson",
        }
    }

    fn parser_version(self) -> &'static str {
        match self {
            Self::NmapXml => "1.0.0",
            Self::BbotNdjson => "1.1.0",
        }
    }

    fn schema_version(self) -> &'static str {
        match self {
            Self::NmapXml => "1.0.0",
            Self::BbotNdjson => "1.1.0",
        }
    }

    fn locator(self) -> &'static str {
        match self {
            Self::NmapXml => "/nmaprun",
            Self::BbotNdjson => "/events",
        }
    }
}

impl ImportServices {
    pub(crate) fn execute(
        bytes: &[u8],
        parsed: Vec<ServiceInput>,
        policy: &ScopePolicy,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
        source: EvidenceSource,
    ) -> ChefResult<Snapshot> {
        check_cancelled(cancelled)?;
        let policy = normalize_policy(policy.clone())?;
        if bytes.len() > policy.max_input_bytes {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Input exceeds scope byte limit",
            ));
        }
        if parsed.len() > policy.max_observations {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Too many service observations",
            ));
        }
        let started_at = clock.now();
        let sha256 = identity::digest(bytes);
        let scope_json = serde_json::to_vec(&policy)
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Invalid scope policy"))?;
        let scope_digest = identity::digest(&scope_json);
        let run_id = identity::id("run", &[&sha256, &scope_digest, source.parser_version()]);
        let evidence_id = identity::id("evidence", &[&run_id, &sha256]);

        for item in &parsed {
            check_cancelled(cancelled)?;
            check_scope(item, &policy)?;
        }

        let mut observations = parsed
            .into_iter()
            .map(|item| Observation {
                id: identity::id("observation", &[&evidence_id, &item.locator]),
                evidence_id: evidence_id.clone(),
                run_id: run_id.clone(),
                source: source.as_str().to_owned(),
                fact: "open-port".to_owned(),
                address: item.address,
                protocol: item.protocol,
                port: item.port,
                observed_at: item.observed_at,
                locator: item.locator,
                confidence: 1.0,
            })
            .collect::<Vec<_>>();
        observations.sort_by(|left, right| left.id.cmp(&right.id));
        let (assets, relationships, paths) = correlate(&observations);
        check_cancelled(cancelled)?;

        let snapshot = Snapshot {
            schema_version: source.schema_version().to_owned(),
            scope_policy: policy.clone(),
            run: ScanRun {
                id: run_id.clone(),
                scope_policy_id: policy.id.clone(),
                scope_digest,
                state: "completed".to_owned(),
                started_at: started_at.clone(),
                finished_at: clock.now(),
            },
            evidence: vec![Evidence {
                id: evidence_id,
                sha256,
                source: source.as_str().to_owned(),
                parser_version: source.parser_version().to_owned(),
                run_id,
                locator: source.locator().to_owned(),
                imported_at: started_at,
            }],
            observations,
            assets,
            relationships,
            paths,
        };
        validate_snapshot(&snapshot)?;
        Ok(snapshot)
    }
}

/// Result of importing an offline BBOT JSONL export into CHEF's canonical snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct BbotImportResult {
    pub snapshot: Snapshot,
    pub skipped_event_types: std::collections::BTreeMap<String, usize>,
    pub event_graph: crate::BbotEventGraph,
    pub recon_graph: crate::ChefReconGraph,
}

pub struct ImportBbotEvidence;

/// Parsea un export JSONL BBOT completo para el motor CHEF de eventos.
/// El parseo conserva hechos y linaje; no concede autorización para red.
pub fn parse_bbot_event_graph(
    bytes: &[u8],
    max_events: usize,
) -> ChefResult<crate::BbotEventGraph> {
    if bytes.len() > INPUT_LIMIT {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "BBOT JSONL supera el máximo de bytes CHEF",
        ));
    }
    event_import::parse_event_graph(bytes, max_events)
}

pub fn validate_bbot_event_graph(graph: &crate::BbotEventGraph) -> ChefResult<()> {
    crate::event_engine::validate_event_graph(graph)
}

impl ImportBbotEvidence {
    pub fn execute(
        bytes: &[u8],
        policy: &ScopePolicy,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<BbotImportResult> {
        Self::execute_with_config(bytes, policy, &ChefConfig::default(), clock, cancelled)
    }

    /// Importa eventos BBOT y ejecuta únicamente los módulos CHEF que el
    /// usuario habilitó en su configuración local. El motor solo contiene
    /// transformaciones offline y no autoriza tráfico.
    pub fn execute_with_config(
        bytes: &[u8],
        policy: &ScopePolicy,
        config: &ChefConfig,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
    ) -> ChefResult<BbotImportResult> {
        Self::execute_with_config_and_observer(bytes, policy, config, clock, cancelled, &mut |_| {
            Ok(())
        })
    }

    /// Variante de importación que notifica eventos saneados conforme el motor
    /// offline los valida y expande. El parseo NDJSON inicial sigue siendo
    /// atómico: ante una línea inválida no se publica un resultado parcial.
    pub fn execute_with_config_and_observer(
        bytes: &[u8],
        policy: &ScopePolicy,
        config: &ChefConfig,
        clock: &impl Clock,
        cancelled: Option<&AtomicBool>,
        observer: &mut dyn FnMut(&crate::BbotEvent) -> ChefResult<()>,
    ) -> ChefResult<BbotImportResult> {
        check_cancelled(cancelled)?;
        config.validate()?;
        if bytes.len() > policy.max_input_bytes {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Input exceeds scope byte limit",
            ));
        }
        let parsed = event_import::parse(bytes, policy.max_observations.min(OBSERVATION_LIMIT))?;
        let mut event_graph = parsed.event_graph;
        if config.modules.values().any(|enabled| *enabled) {
            let engine = BbotEventEngine::with_chef_modules(
                config.runtime.max_events,
                config.runtime.max_depth,
            )?;
            event_graph = engine.run_offline_with_observer(
                event_graph.events.clone(),
                config,
                clock,
                cancelled,
                observer,
            )?;
        } else {
            for event in &event_graph.events {
                check_cancelled(cancelled)?;
                observer(event)?;
            }
        }
        let event_count_before_mutations = event_graph.events.len();
        append_dns_mutation_hints(&mut event_graph, config, clock, cancelled)?;
        for event in &event_graph.events[event_count_before_mutations..] {
            check_cancelled(cancelled)?;
            observer(event)?;
        }
        validate_bbot_event_graph(&event_graph)?;
        let recon_graph = crate::adapt_bbot_to_chef(&event_graph)?;
        let snapshot = ImportServices::execute(
            bytes,
            parsed.services,
            policy,
            clock,
            cancelled,
            EvidenceSource::BbotNdjson,
        )?;
        Ok(BbotImportResult {
            snapshot,
            skipped_event_types: recon_graph.unmapped_event_types.clone(),
            event_graph,
            recon_graph,
        })
    }
}

fn append_dns_mutation_hints(
    graph: &mut crate::BbotEventGraph,
    config: &ChefConfig,
    clock: &impl Clock,
    cancelled: Option<&AtomicBool>,
) -> ChefResult<()> {
    if !config
        .modules
        .get("chef_dns_mutation_hints")
        .copied()
        .unwrap_or(false)
    {
        return Ok(());
    }

    let remaining = config.runtime.max_events.saturating_sub(graph.events.len());
    if remaining == 0 {
        return Ok(());
    }
    let batch = crate::generate_dns_mutation_candidates(
        &graph.events,
        crate::DnsMutationCandidateOptions {
            max_candidates: remaining,
            max_candidates_per_root: 100,
        },
    )?;
    let events_by_id = graph
        .events
        .iter()
        .map(|event| (event.id.clone(), event.clone()))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut existing_event_ids = graph
        .events
        .iter()
        .map(|event| event.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let mut graph_bytes = serde_json::to_vec(graph)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Grafo de eventos inválido"))?
        .len();

    graph.modules_run = graph.modules_run.saturating_add(1);
    graph.duplicates_suppressed = graph
        .duplicates_suppressed
        .saturating_add(batch.duplicates_suppressed);
    for candidate in batch.candidates {
        check_cancelled(cancelled)?;
        let parent = candidate
            .source_event_ids
            .iter()
            .filter_map(|source_id| events_by_id.get(source_id).cloned())
            .min_by(|left, right| {
                left.depth
                    .cmp(&right.depth)
                    .then_with(|| left.id.cmp(&right.id))
            });
        let Some(parent) = parent else {
            continue;
        };
        let Some(depth) = parent.depth.checked_add(1) else {
            continue;
        };
        if depth > config.runtime.max_depth || parent.scope_distance >= 32 {
            continue;
        }

        let reasons = candidate
            .reasons
            .iter()
            .map(|reason| match reason {
                crate::DnsMutationReason::CrossRootLabel => "cross_root_label",
                crate::DnsMutationReason::ExtractedWord => "extracted_word",
                crate::DnsMutationReason::NumericSuffix => "numeric_suffix",
                crate::DnsMutationReason::NumberAdjustment => "number_adjustment",
                crate::DnsMutationReason::WordCloudModifier => "wordcloud_modifier",
                crate::DnsMutationReason::WordCloudBase => "wordcloud_base",
                crate::DnsMutationReason::StaticWordlist => "static_wordlist",
            })
            .collect::<Vec<_>>();
        let mut tags = vec!["chef:derived-dns-mutation-hint".to_owned()];
        tags.extend(
            reasons
                .iter()
                .map(|reason| format!("chef:dns-mutation:{reason}")),
        );
        let candidate_event_id = crate::identity::id("chef-dns-mutation-event", &[&candidate.id]);
        if !existing_event_ids.insert(candidate_event_id.clone()) {
            continue;
        }
        let mut event = crate::BbotEvent {
            id: candidate_event_id,
            source_id: None,
            source_uuid: None,
            event_type: "DNS_NAME_HINT".to_owned(),
            data: serde_json::Value::String(candidate.name.clone()),
            data_redacted: false,
            metadata: serde_json::json!({
                "candidateId": candidate.id,
                "root": candidate.root,
                "ruleId": candidate.rule_id,
                "ruleVersion": candidate.rule_version,
                "reasons": reasons,
                "sourceEventIds": candidate.source_event_ids,
            }),
            metadata_redacted: false,
            host: Some(candidate.name),
            port: None,
            parent_id: Some(parent.id.clone()),
            parent_source_id: None,
            parent_source_uuid: None,
            timestamp: clock.now(),
            tags,
            module: Some("chef_dns_mutation_hints".to_owned()),
            locator: format!(
                "rule:{}@{};reason={}",
                candidate.rule_id,
                candidate.rule_version,
                reasons.join(",")
            ),
            depth,
            scope_distance: parent.scope_distance.saturating_add(1),
        };
        let (data, redacted) =
            crate::event_engine::sanitize_event_data(&event.event_type, event.data);
        event.data = data;
        event.data_redacted = redacted;
        let event_bytes = serde_json::to_vec(&event)
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Pista DNS inválida"))?
            .len();
        if graph_bytes.saturating_add(event_bytes).saturating_add(1) > EVENT_GRAPH_BYTE_LIMIT {
            break;
        }
        graph_bytes += event_bytes + 1;
        graph.events.push(event);
    }
    Ok(())
}

fn check_cancelled(cancelled: Option<&AtomicBool>) -> ChefResult<()> {
    if cancelled.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
        Err(ChefError::new(ErrorCode::Cancelled, "Import cancelled"))
    } else {
        Ok(())
    }
}

fn validate_policy(policy: &ScopePolicy) -> ChefResult<()> {
    if policy.schema_version != SCHEMA_VERSION
        || policy.id.is_empty()
        || policy.id.len() > 2048
        || policy.allowed_addresses.is_empty()
        || policy.allowed_addresses.len() > 1000
        || policy.excluded_addresses.len() > 1000
        || policy
            .allowed_addresses
            .iter()
            .chain(policy.excluded_addresses.iter())
            .any(|address| canonical_scope_address(address).is_err())
        || policy.allowed_ports.is_empty()
        || policy.allowed_protocols.is_empty()
        || policy.allowed_protocols.len() > 2
        || policy.max_input_bytes == 0
        || policy.max_input_bytes > INPUT_LIMIT
        || policy.max_observations == 0
        || policy.max_observations > OBSERVATION_LIMIT
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Scope policy fails schema validation",
        ));
    }

    if policy
        .allowed_addresses
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != policy.allowed_addresses.len()
        || policy
            .excluded_addresses
            .iter()
            .any(|address| canonical_scope_address(address).is_err())
        || policy.allowed_ports.contains(&0)
        || policy
            .allowed_ports
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != policy.allowed_ports.len()
        || policy
            .allowed_protocols
            .iter()
            .any(|protocol| protocol != "tcp" && protocol != "udp")
        || policy
            .allowed_protocols
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            != policy.allowed_protocols.len()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Scope policy fails schema validation",
        ));
    }
    Ok(())
}

fn normalize_policy(mut policy: ScopePolicy) -> ChefResult<ScopePolicy> {
    if policy.allowed_addresses.len() > 1000
        || policy.excluded_addresses.len() > 1000
        || policy.allowed_ports.len() > 65_535
        || policy.allowed_protocols.len() > 2
        || policy.id.len() > 2048
        || policy
            .allowed_addresses
            .iter()
            .chain(policy.excluded_addresses.iter())
            .any(|address| address.len() > 64)
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Scope policy fails schema validation",
        ));
    }
    for address in &mut policy.allowed_addresses {
        *address = canonical_scope_address(address)?;
    }
    for address in &mut policy.excluded_addresses {
        *address = canonical_scope_address(address)?;
    }
    policy.allowed_addresses.sort();
    policy.allowed_addresses.dedup();
    policy.excluded_addresses.sort();
    policy.excluded_addresses.dedup();
    policy.allowed_ports.sort_unstable();
    policy.allowed_ports.dedup();
    policy.allowed_protocols.sort();
    policy.allowed_protocols.dedup();
    validate_policy(&policy)?;
    Ok(policy)
}

fn canonical_address(value: &str) -> ChefResult<String> {
    if value.contains('%') {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Expected a literal IP address without zone ID",
        ));
    }
    value
        .parse::<std::net::IpAddr>()
        .map(|address| address.to_string())
        .map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "Expected a literal IP address without zone ID",
            )
        })
}

fn canonical_scope_address(value: &str) -> ChefResult<String> {
    if value.contains('/') {
        return crate::target_seed::canonical_network(value).ok_or_else(|| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "Scope address must be a literal IP address or canonicalizable CIDR",
            )
        });
    }
    canonical_address(value)
}

fn scope_contains(scope_address: &str, address: std::net::IpAddr) -> bool {
    if scope_address.contains('/') {
        return crate::target_seed::network_contains(scope_address, address);
    }
    scope_address.parse::<std::net::IpAddr>().ok() == Some(address)
}

fn check_scope(input: &ServiceInput, policy: &ScopePolicy) -> ChefResult<()> {
    let address = canonical_address(&input.address)?;
    let address = address
        .parse::<std::net::IpAddr>()
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Expected a literal IP address"))?;
    let excluded = policy
        .excluded_addresses
        .iter()
        .any(|value| scope_contains(value, address));
    let allowed = policy
        .allowed_addresses
        .iter()
        .any(|value| scope_contains(value, address));
    if excluded
        || !allowed
        || !policy.allowed_ports.contains(&input.port)
        || !policy.allowed_protocols.contains(&input.protocol)
    {
        return Err(ChefError::new(
            ErrorCode::OutOfScope,
            "An imported open service is outside the explicit scope policy",
        ));
    }
    Ok(())
}

fn correlate(observations: &[Observation]) -> (Vec<Asset>, Vec<Relationship>, Vec<ExposurePath>) {
    let mut assets = std::collections::BTreeMap::<String, Asset>::new();
    let mut relations = std::collections::BTreeMap::<String, Relationship>::new();
    for observation in observations {
        let address_id = identity::id("asset", &["address", &observation.address]);
        let service_key = format!(
            "{}|{}|{}",
            observation.address, observation.protocol, observation.port
        );
        let service_id = identity::id("asset", &["service", &service_key]);
        upsert_asset(
            &mut assets,
            &address_id,
            "address",
            &observation.address,
            observation,
        );
        upsert_asset(
            &mut assets,
            &service_id,
            "service",
            &service_key,
            observation,
        );

        let relation_id = identity::id("relationship", &["exposes", &address_id, &service_id]);
        let relation = relations
            .entry(relation_id.clone())
            .or_insert_with(|| Relationship {
                id: relation_id,
                from: address_id,
                to: service_id,
                kind: "exposes".to_owned(),
                assertion: "observed".to_owned(),
                observation_ids: Vec::new(),
                confidence: observation.confidence,
            });
        relation.confidence = relation.confidence.max(observation.confidence);
        if !relation.observation_ids.contains(&observation.id) {
            relation.observation_ids.push(observation.id.clone());
        }
    }
    let relationships = relations.into_values().collect::<Vec<_>>();
    let paths = relationships
        .iter()
        .map(|relation| ExposurePath {
            id: identity::id("path", &["open-service/v1", &relation.id]),
            asset_ids: vec![relation.from.clone(), relation.to.clone()],
            relationship_ids: vec![relation.id.clone()],
            assertion: "inferred".to_owned(),
            rule_id: "open-service/v1".to_owned(),
            explanation: "An imported observation reports an open service. Reachability now and exploitability have not been confirmed.".to_owned(),
            confidence: relation.confidence,
            is_confirmed_vulnerability: false,
        })
        .collect();
    (assets.into_values().collect(), relationships, paths)
}

fn upsert_asset(
    assets: &mut std::collections::BTreeMap<String, Asset>,
    id: &str,
    kind: &str,
    key: &str,
    observation: &Observation,
) {
    let asset = assets.entry(id.to_owned()).or_insert_with(|| Asset {
        id: id.to_owned(),
        kind: kind.to_owned(),
        key: key.to_owned(),
        first_seen: observation.observed_at.clone(),
        last_seen: observation.observed_at.clone(),
        observation_ids: Vec::new(),
    });
    if observation.observed_at < asset.first_seen {
        asset.first_seen = observation.observed_at.clone();
    }
    if observation.observed_at > asset.last_seen {
        asset.last_seen = observation.observed_at.clone();
    }
    if !asset.observation_ids.contains(&observation.id) {
        asset.observation_ids.push(observation.id.clone());
    }
}

fn validate_snapshot(snapshot: &Snapshot) -> ChefResult<()> {
    if !matches!(
        (
            snapshot.schema_version.as_str(),
            snapshot.evidence.first().map(|e| e.source.as_str())
        ),
        ("1.0.0", Some("nmap-xml")) | ("1.1.0", Some("bbot-ndjson"))
    ) || snapshot.observations.len() > OBSERVATION_LIMIT
        || snapshot
            .assets
            .iter()
            .any(|asset| asset.observation_ids.is_empty())
        || snapshot.observations.iter().any(|item| {
            item.source
                != snapshot
                    .evidence
                    .first()
                    .map(|e| e.source.as_str())
                    .unwrap_or_default()
                || item.fact != "open-port"
                || !(0.0..=1.0).contains(&item.confidence)
        })
        || snapshot
            .paths
            .iter()
            .any(|path| path.is_confirmed_vulnerability || path.assertion != "inferred")
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Snapshot fails contract validation",
        ));
    }
    let observation_ids = snapshot
        .observations
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let asset_ids = snapshot
        .assets
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let relation_ids = snapshot
        .relationships
        .iter()
        .map(|item| item.id.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    if snapshot.assets.iter().any(|item| {
        item.observation_ids
            .iter()
            .any(|id| !observation_ids.contains(id.as_str()))
    }) || snapshot.relationships.iter().any(|item| {
        !asset_ids.contains(item.from.as_str())
            || !asset_ids.contains(item.to.as_str())
            || item
                .observation_ids
                .iter()
                .any(|id| !observation_ids.contains(id.as_str()))
    }) || snapshot.paths.iter().any(|path| {
        path.asset_ids
            .iter()
            .any(|id| !asset_ids.contains(id.as_str()))
            || path
                .relationship_ids
                .iter()
                .any(|id| !relation_ids.contains(id.as_str()))
    }) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Snapshot contains unresolved references",
        ));
    }
    Ok(())
}
