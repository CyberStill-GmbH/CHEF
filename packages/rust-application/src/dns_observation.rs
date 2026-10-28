//! Proyección saneada de resultados DNS autorizados para almacenamiento local.
//!
//! Conserva autorización, tiempo y linaje; reemplaza RDATA crudo por un digest
//! y solo extrae objetivos tipados cuando la política permite seguimiento.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    DnsAnswerDisposition, DnsLookupOutcome, DnsQueryRequest, DnsResolutionError,
    DnsResolverFailure, DnsServicePlanExecution,
};

pub const DNS_OBSERVATION_SCHEMA_VERSION: &str = "1.0.0";
const MAX_DNS_OBSERVATIONS: usize = 256;
const MAX_TARGET_CANDIDATES_PER_ANSWER: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DnsObservationBatch {
    schema_version: String,
    batch_id: String,
    termination: String,
    observations: Vec<DnsQueryObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DnsQueryObservation {
    owner_name: String,
    scope_host: String,
    record_type: String,
    observed_at: String,
    resolution_hops: u8,
    authorization_id: String,
    scope_digest: String,
    source_evidence_id: Option<String>,
    outcome: DnsObservationOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum DnsObservationOutcome {
    Answers { answers: Vec<DnsObservationAnswer> },
    NoAnswers,
    Failed { failure: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct DnsObservationAnswer {
    record_type: String,
    ttl_seconds: u64,
    disposition: String,
    rdata_sha256: String,
    target_candidates: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsObservationError {
    InvalidBatch,
    LimitExceeded,
    InvalidAnswer,
}

impl DnsObservationBatch {
    pub fn from_service_execution(
        batch_id: String,
        execution: &DnsServicePlanExecution,
    ) -> Result<Self, DnsObservationError> {
        if !is_batch_id(&batch_id) || execution.results.len() > MAX_DNS_OBSERVATIONS {
            return Err(if execution.results.len() > MAX_DNS_OBSERVATIONS {
                DnsObservationError::LimitExceeded
            } else {
                DnsObservationError::InvalidBatch
            });
        }

        let observations = execution
            .results
            .iter()
            .map(|result| {
                let resolution = &result.resolution;
                let outcome = match &resolution.outcome {
                    DnsLookupOutcome::Answers(answers) => {
                        let answers = answers
                            .iter()
                            .map(|item| {
                                let answer = &item.answer;
                                let target_candidates =
                                    if item.disposition == DnsAnswerDisposition::FollowUpAllowed {
                                        chef_core::extract_dns_targets(
                                            answer.record_type().as_str(),
                                            answer.rdata(),
                                        )
                                        .map_err(|_| DnsObservationError::InvalidAnswer)?
                                        .into_iter()
                                        .map(|target| target.value)
                                        .collect::<Vec<_>>()
                                    } else {
                                        Vec::new()
                                    };
                                if target_candidates.len() > MAX_TARGET_CANDIDATES_PER_ANSWER {
                                    return Err(DnsObservationError::LimitExceeded);
                                }
                                Ok(DnsObservationAnswer {
                                    record_type: answer.record_type().as_str().to_owned(),
                                    ttl_seconds: answer.ttl().as_secs(),
                                    disposition: disposition_name(item.disposition).to_owned(),
                                    rdata_sha256: sha256(answer.rdata().as_bytes()),
                                    target_candidates,
                                })
                            })
                            .collect::<Result<Vec<_>, DnsObservationError>>()?;
                        DnsObservationOutcome::Answers { answers }
                    }
                    DnsLookupOutcome::NoAnswers => DnsObservationOutcome::NoAnswers,
                    DnsLookupOutcome::Failed(failure) => DnsObservationOutcome::Failed {
                        failure: resolver_failure_name(*failure).to_owned(),
                    },
                };
                Ok(DnsQueryObservation {
                    owner_name: result.owner_name.clone(),
                    scope_host: resolution.scope_host.clone(),
                    record_type: resolution.record_type.clone(),
                    observed_at: result.observed_at.clone(),
                    resolution_hops: resolution.resolution_hops,
                    authorization_id: resolution.authorization_id.clone(),
                    scope_digest: resolution.scope_digest.clone(),
                    source_evidence_id: result.source_evidence_id.clone(),
                    outcome,
                })
            })
            .collect::<Result<Vec<_>, DnsObservationError>>()?;

        let batch = Self {
            schema_version: DNS_OBSERVATION_SCHEMA_VERSION.to_owned(),
            batch_id,
            termination: termination_name(execution.termination).to_owned(),
            observations,
        };
        batch.validate()?;
        Ok(batch)
    }

    pub fn validate(&self) -> Result<(), DnsObservationError> {
        if self.schema_version != DNS_OBSERVATION_SCHEMA_VERSION
            || !is_batch_id(&self.batch_id)
            || !matches!(
                self.termination.as_str(),
                "completed"
                    | "invalid_query"
                    | "limit_exceeded"
                    | "not_authorized"
                    | "invalid_authorization"
                    | "authorization_expired"
                    | "cancelled"
            )
        {
            return Err(DnsObservationError::InvalidBatch);
        }
        if self.observations.is_empty() {
            return Err(DnsObservationError::InvalidBatch);
        }
        if self.observations.len() > MAX_DNS_OBSERVATIONS {
            return Err(DnsObservationError::LimitExceeded);
        }
        for observation in &self.observations {
            if observation.owner_name.is_empty()
                || observation.owner_name.len() > 253
                || observation.scope_host.is_empty()
                || observation.scope_host.len() > 253
                || observation.record_type != "SRV"
                || observation.observed_at.len() > 64
                || chrono::DateTime::parse_from_rfc3339(&observation.observed_at).is_err()
                || observation.authorization_id.is_empty()
                || observation.authorization_id.len() > 128
                || observation.scope_digest.len() != 64
                || !observation
                    .scope_digest
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit())
                || observation
                    .source_evidence_id
                    .as_deref()
                    .is_some_and(|id| !is_evidence_id(id))
                || DnsQueryRequest::for_srv_owner(
                    &observation.owner_name,
                    &observation.scope_host,
                    observation.resolution_hops,
                )
                .is_err()
            {
                return Err(DnsObservationError::InvalidBatch);
            }
            if let DnsObservationOutcome::Answers { answers } = &observation.outcome {
                if answers.len() > 4096 {
                    return Err(DnsObservationError::LimitExceeded);
                }
                for answer in answers {
                    if answer.record_type.is_empty()
                        || answer.record_type.len() > 16
                        || answer
                            .record_type
                            .bytes()
                            .any(|byte| !byte.is_ascii_alphanumeric() && byte != b'-')
                        || !matches!(
                            answer.disposition.as_str(),
                            "follow_up_allowed" | "evidence_only"
                        )
                        || !is_sha256(&answer.rdata_sha256)
                        || answer.target_candidates.len() > MAX_TARGET_CANDIDATES_PER_ANSWER
                        || (answer.disposition == "evidence_only"
                            && !answer.target_candidates.is_empty())
                    {
                        return Err(DnsObservationError::InvalidAnswer);
                    }
                }
            } else if let DnsObservationOutcome::Failed { failure } = &observation.outcome
                && !matches!(
                    failure.as_str(),
                    "name_not_found"
                        | "timeout"
                        | "server_failure"
                        | "refused"
                        | "network"
                        | "invalid_response"
                        | "other"
                )
            {
                return Err(DnsObservationError::InvalidAnswer);
            }
        }
        Ok(())
    }

    /// Validación adicional para persistencia: cada consulta debe tener linaje
    /// hacia evidencia importada, nunca solo hacia una entrada manual.
    pub fn validate_for_storage(&self) -> Result<(), DnsObservationError> {
        self.validate()?;
        if self
            .observations
            .iter()
            .any(|observation| observation.source_evidence_id.is_none())
        {
            return Err(DnsObservationError::InvalidBatch);
        }
        Ok(())
    }

    pub fn schema_version(&self) -> &str {
        &self.schema_version
    }

    pub fn batch_id(&self) -> &str {
        &self.batch_id
    }

    pub fn observations(&self) -> &[DnsQueryObservation] {
        &self.observations
    }

    pub fn source_evidence_ids(&self) -> impl Iterator<Item = &str> {
        self.observations
            .iter()
            .filter_map(|observation| observation.source_evidence_id.as_deref())
    }

    pub fn to_json_value(&self) -> Result<serde_json::Value, serde_json::Error> {
        serde_json::to_value(self)
    }

    pub fn from_json_value(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

fn is_batch_id(value: &str) -> bool {
    value.starts_with("dns-batch:")
        && (11..=128).contains(&value.len())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-_".contains(&byte)
        })
}

fn is_evidence_id(value: &str) -> bool {
    value.len() == 78
        && value.starts_with("chef-evidence:")
        && value[14..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256(value: &[u8]) -> String {
    Sha256::digest(value)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn disposition_name(disposition: DnsAnswerDisposition) -> &'static str {
    match disposition {
        DnsAnswerDisposition::FollowUpAllowed => "follow_up_allowed",
        DnsAnswerDisposition::EvidenceOnly => "evidence_only",
    }
}

fn termination_name(termination: crate::DnsServicePlanTermination) -> &'static str {
    match termination {
        crate::DnsServicePlanTermination::Completed => "completed",
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::InvalidQuery) => {
            "invalid_query"
        }
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::LimitExceeded) => {
            "limit_exceeded"
        }
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::NotAuthorized) => {
            "not_authorized"
        }
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::InvalidAuthorization) => {
            "invalid_authorization"
        }
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::AuthorizationExpired) => {
            "authorization_expired"
        }
        crate::DnsServicePlanTermination::Stopped(DnsResolutionError::Cancelled) => "cancelled",
    }
}

fn resolver_failure_name(failure: DnsResolverFailure) -> &'static str {
    match failure {
        DnsResolverFailure::NameNotFound => "name_not_found",
        DnsResolverFailure::Timeout => "timeout",
        DnsResolverFailure::ServerFailure => "server_failure",
        DnsResolverFailure::Refused => "refused",
        DnsResolverFailure::Network => "network",
        DnsResolverFailure::InvalidResponse => "invalid_response",
        DnsResolverFailure::Other => "other",
    }
}
