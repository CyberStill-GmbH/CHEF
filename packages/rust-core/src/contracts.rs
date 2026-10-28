use serde::{Deserialize, Serialize};

pub const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScopePolicy {
    pub schema_version: String,
    pub id: String,
    pub allowed_addresses: Vec<String>,
    pub excluded_addresses: Vec<String>,
    pub allowed_ports: Vec<u16>,
    pub allowed_protocols: Vec<String>,
    pub max_input_bytes: usize,
    pub max_observations: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Snapshot {
    pub schema_version: String,
    pub scope_policy: ScopePolicy,
    pub run: ScanRun,
    pub evidence: Vec<Evidence>,
    pub observations: Vec<Observation>,
    pub assets: Vec<Asset>,
    pub relationships: Vec<Relationship>,
    pub paths: Vec<ExposurePath>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScanRun {
    pub id: String,
    pub scope_policy_id: String,
    pub scope_digest: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Evidence {
    pub id: String,
    pub sha256: String,
    pub source: String,
    pub parser_version: String,
    pub run_id: String,
    pub locator: String,
    pub imported_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Observation {
    pub id: String,
    pub evidence_id: String,
    pub run_id: String,
    pub source: String,
    pub fact: String,
    pub address: String,
    pub protocol: String,
    pub port: u16,
    pub observed_at: String,
    pub locator: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub kind: String,
    pub key: String,
    pub first_seen: String,
    pub last_seen: String,
    pub observation_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Relationship {
    pub id: String,
    pub from: String,
    pub to: String,
    pub kind: String,
    pub assertion: String,
    pub observation_ids: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExposurePath {
    pub id: String,
    pub asset_ids: Vec<String>,
    pub relationship_ids: Vec<String>,
    pub assertion: String,
    pub rule_id: String,
    pub explanation: String,
    pub confidence: f64,
    pub is_confirmed_vulnerability: bool,
}
