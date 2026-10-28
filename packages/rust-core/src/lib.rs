//! Rust port of CHEF's offline evidence core.
//!
//! This crate deliberately has no filesystem, network, Tauri, or database access.
//! The caller supplies bounded bytes, scope, and timestamps through explicit inputs.

mod application;
mod asn;
mod config;
mod config_merge;
mod contracts;
mod dns_cache;
mod dns_format;
mod dns_mutation_candidates;
mod dns_policy;
mod dns_records;
mod dns_service_candidates;
mod dns_word_segmentation;
mod domain;
mod event_engine;
mod event_import;
mod host;
mod identity;
mod jobs;
mod keyed_lock;
mod nmap;
mod port_spec;
mod presets;
mod rate_limit;
mod recon;
mod speculate;
mod statistics;
mod target_seed;
mod text_policy;
mod url_fingerprint;
mod url_policy;
mod url_tools;
mod wordcloud;

pub use application::{
    BbotImportResult, Clock, FixedClock, ImportBbotEvidence, ImportEvidence, SystemClock,
    parse_bbot_event_graph, validate_bbot_event_graph,
};
pub use asn::{
    AsnEnrichmentCircuitBreaker, AsnRecord, AsnTargetRetryPolicy, normalize_asn_response,
};
pub use config::{ChefConfig, DefectDojoSettings, RuntimeSettings, ScopeSettings, SourceSettings};
pub use config_merge::{
    deep_merge_toml, dotted_get, dotted_set, iter_dotted_paths, parse_layered_toml,
};
pub use contracts::{ScopePolicy, Snapshot};
pub use dns_cache::DnsCache;
pub use dns_format::format_dns_rdata;
pub use dns_mutation_candidates::{
    DnsMutationCandidate, DnsMutationCandidateBatch, DnsMutationCandidateOptions,
    DnsMutationReason, generate_dns_mutation_candidates,
};
pub use dns_policy::{DnsQueryPolicy, DnsZoneDecision};
pub use dns_records::{DNS_RECORD_TYPES, DnsTarget, extract_dns_targets, is_dns_service_record};
pub use dns_service_candidates::common_dns_service_candidates;
pub use domain::{ChefError, ChefResult, ErrorCode, ServiceInput};
pub use event_engine::{
    BbotEvent, BbotEventDraft, BbotEventEngine, BbotEventGraph, BbotModule, EventScopeGate,
    ModuleDescriptor,
};
pub use jobs::{
    ImportCancellation, ImportFormat, ImportJobEvent, ImportOutput, ImportTask,
    spawn_bbot_import_job, spawn_bbot_import_job_with_config, spawn_import_job,
};
pub use keyed_lock::{KeyedLock, KeyedLockGuard};
pub use port_spec::parse_port_string;
pub use presets::{ChefPreset, PresetCatalog, ResolvedPreset};
pub use rate_limit::RateLimiter;
pub use recon::{
    ChefReconGraph, ReconAsset, ReconAssetKind, ReconEvidence, ReconRelationship,
    ReconRelationshipKind, adapt_bbot_to_chef, validate_chef_recon_graph,
};
pub use statistics::{
    BbotEventStatistics, BbotRuntimeStatistics, EventTypeCount, ModuleProducedCount,
    ModuleRuntimeStatistics, summarize_bbot_events,
};
pub use target_seed::{ChefTargetKind, ChefTargetSeed, parse_chef_target_seed};
pub use text_policy::{escape_terminal_controls, is_printable_text, truncate_text};
pub use url_fingerprint::url_shape_fingerprint;
pub use url_policy::{canonical_fqdn_or_ip, url_path_depth};

#[cfg(test)]
mod tests;
