//! Caso de uso DNS con dos puertos obligatorios: resolver y autorización.
//!
//! El módulo no elige resolvers, consulta red ni acepta `ScopePolicy` importado
//! como autorización. La app debe entregar un autorizador que emita un recibo
//! de consentimiento vigente y un adaptador de red por separado.

use std::{
    fmt,
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use chef_core::{ChefTargetKind, parse_chef_target_seed};
use tokio::sync::Notify;

const MAX_RECORD_TYPE_BYTES: usize = 16;
const MAX_ANSWER_BYTES: usize = 64 * 1024;
const MAX_ANSWERS_PER_LOOKUP: usize = 4096;
const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const MAX_AUTHORIZATION_ID_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsRecordType(String);

impl DnsRecordType {
    pub fn parse(value: &str) -> Result<Self, DnsResolutionError> {
        let value = value.trim();
        if value.is_empty()
            || value.len() > MAX_RECORD_TYPE_BYTES
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        {
            return Err(DnsResolutionError::InvalidQuery);
        }
        Ok(Self(value.to_ascii_uppercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQueryRequest {
    host: String,
    record_type: DnsRecordType,
    resolution_hops: u8,
}

impl DnsQueryRequest {
    pub fn new(
        host: &str,
        record_type: &str,
        resolution_hops: u8,
    ) -> Result<Self, DnsResolutionError> {
        if resolution_hops > 32 {
            return Err(DnsResolutionError::LimitExceeded);
        }
        let seed = parse_chef_target_seed(host).map_err(|_| DnsResolutionError::InvalidQuery)?;
        if !matches!(
            seed.kind,
            ChefTargetKind::DnsName | ChefTargetKind::IpAddress
        ) {
            return Err(DnsResolutionError::InvalidQuery);
        }
        let host = seed.host.ok_or(DnsResolutionError::InvalidQuery)?;
        Ok(Self {
            host,
            record_type: DnsRecordType::parse(record_type)?,
            resolution_hops,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn record_type(&self) -> &DnsRecordType {
        &self.record_type
    }

    pub fn resolution_hops(&self) -> u8 {
        self.resolution_hops
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsAnswer {
    record_type: DnsRecordType,
    rdata: String,
    ttl: Duration,
}

impl DnsAnswer {
    pub fn new(
        record_type: &str,
        rdata: String,
        ttl: Duration,
    ) -> Result<Self, DnsResolutionError> {
        if rdata.is_empty() || rdata.len() > MAX_ANSWER_BYTES {
            return Err(DnsResolutionError::LimitExceeded);
        }
        Ok(Self {
            record_type: DnsRecordType::parse(record_type)?,
            rdata,
            ttl,
        })
    }

    pub fn record_type(&self) -> &DnsRecordType {
        &self.record_type
    }

    pub fn rdata(&self) -> &str {
        &self.rdata
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

/// Fallos tipados del adaptador, sin incluir hosts, respuestas ni texto externo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResolverFailure {
    NameNotFound,
    Timeout,
    ServerFailure,
    Refused,
    Network,
    InvalidResponse,
    Other,
}

pub trait DnsResolverPort: Send + Sync {
    fn lookup(
        &self,
        request: &DnsQueryRequest,
    ) -> impl Future<Output = Result<Vec<DnsAnswer>, DnsResolverFailure>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsAuthorizationFailure {
    NotAuthorized,
    InvalidReceipt,
}

#[derive(Debug, Clone)]
pub struct DnsAuthorizationReceipt {
    authorization_id: String,
    scope_digest: String,
    expires_at: Instant,
}

impl DnsAuthorizationReceipt {
    /// Lo construye únicamente el adaptador de autorización tras confirmación explícita.
    pub fn new(
        authorization_id: String,
        scope_digest: String,
        expires_at: Instant,
    ) -> Result<Self, DnsAuthorizationFailure> {
        if authorization_id.is_empty()
            || authorization_id.len() > MAX_AUTHORIZATION_ID_BYTES
            || scope_digest.len() != 64
            || !scope_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || expires_at <= Instant::now()
        {
            return Err(DnsAuthorizationFailure::InvalidReceipt);
        }
        Ok(Self {
            authorization_id,
            scope_digest: scope_digest.to_ascii_lowercase(),
            expires_at,
        })
    }

    pub fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub fn scope_digest(&self) -> &str {
        &self.scope_digest
    }

    pub fn is_valid_at(&self, now: Instant) -> bool {
        now < self.expires_at
    }

    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsAnswerDisposition {
    /// La política autorizada permite considerar el resultado en otra consulta.
    FollowUpAllowed,
    /// Se conserva como evidencia, pero no habilita recursión ni scope nuevo.
    EvidenceOnly,
}

pub trait DnsAuthorizationPort: Send + Sync {
    fn authorize_query(
        &self,
        request: &DnsQueryRequest,
    ) -> Result<DnsAuthorizationReceipt, DnsAuthorizationFailure>;

    fn classify_answer(
        &self,
        receipt: &DnsAuthorizationReceipt,
        request: &DnsQueryRequest,
        answer: &DnsAnswer,
    ) -> DnsAnswerDisposition;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsAnswerEvidence {
    pub answer: DnsAnswer,
    pub disposition: DnsAnswerDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsLookupOutcome {
    Answers(Vec<DnsAnswerEvidence>),
    NoAnswers,
    Failed(DnsResolverFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsResolutionResult {
    pub host: String,
    pub record_type: String,
    pub resolution_hops: u8,
    pub authorization_id: String,
    pub scope_digest: String,
    pub outcome: DnsLookupOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsResolutionError {
    InvalidQuery,
    LimitExceeded,
    NotAuthorized,
    InvalidAuthorization,
    AuthorizationExpired,
    Cancelled,
}

impl fmt::Display for DnsResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidQuery => "La consulta DNS no es válida",
            Self::LimitExceeded => "La consulta DNS supera los límites CHEF",
            Self::NotAuthorized => "La consulta DNS no está autorizada para este scope",
            Self::InvalidAuthorization => "La autorización DNS no es válida",
            Self::AuthorizationExpired => {
                "La autorización DNS expiró antes de completar la consulta"
            }
            Self::Cancelled => "La consulta DNS fue cancelada",
        })
    }
}

impl std::error::Error for DnsResolutionError {}

#[derive(Debug, Default)]
struct CancellationState {
    cancelled: AtomicBool,
    notify: Notify,
}

/// Señal clonable para detener una consulta en curso o antes de iniciarla.
#[derive(Debug, Clone, Default)]
pub struct DnsCancellation(Arc<CancellationState>);

impl DnsCancellation {
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        self.0.notify.notify_one();
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }

    async fn cancelled(&self) {
        loop {
            let notified = self.0.notify.notified();
            if self.is_cancelled() {
                return;
            }
            notified.await;
        }
    }
}

pub struct ResolveDns<'a, R, A> {
    resolver: &'a R,
    authorization: &'a A,
}

impl<'a, R, A> ResolveDns<'a, R, A>
where
    R: DnsResolverPort,
    A: DnsAuthorizationPort,
{
    pub fn new(resolver: &'a R, authorization: &'a A) -> Self {
        Self {
            resolver,
            authorization,
        }
    }

    /// Verifica autorización antes del puerto de red y conserva respuestas fuera
    /// de alcance como evidencia sin habilitar seguimiento recursivo.
    pub async fn execute(
        &self,
        request: DnsQueryRequest,
        cancellation: &DnsCancellation,
    ) -> Result<DnsResolutionResult, DnsResolutionError> {
        if cancellation.is_cancelled() {
            return Err(DnsResolutionError::Cancelled);
        }
        let receipt = self
            .authorization
            .authorize_query(&request)
            .map_err(|_| DnsResolutionError::NotAuthorized)?;
        if !receipt.is_valid_at(Instant::now()) {
            return Err(DnsResolutionError::InvalidAuthorization);
        }

        let expires_at = tokio::time::Instant::from_std(receipt.expires_at());
        let lookup = tokio::select! {
            _ = cancellation.cancelled() => return Err(DnsResolutionError::Cancelled),
            _ = tokio::time::sleep_until(expires_at) => {
                return Err(DnsResolutionError::AuthorizationExpired);
            },
            result = self.resolver.lookup(&request) => result,
        };
        if cancellation.is_cancelled() {
            return Err(DnsResolutionError::Cancelled);
        }
        if !receipt.is_valid_at(Instant::now()) {
            return Err(DnsResolutionError::AuthorizationExpired);
        }

        let outcome = match lookup {
            Err(error) => DnsLookupOutcome::Failed(error),
            Ok(answers) if answers.is_empty() => DnsLookupOutcome::NoAnswers,
            Ok(answers) => {
                if answers.len() > MAX_ANSWERS_PER_LOOKUP
                    || answers
                        .iter()
                        .any(|answer| answer.rdata.len() > MAX_ANSWER_BYTES)
                    || answers
                        .iter()
                        .try_fold(0_usize, |size, answer| size.checked_add(answer.rdata.len()))
                        .is_none_or(|size| size > MAX_RESPONSE_BYTES)
                {
                    return Err(DnsResolutionError::LimitExceeded);
                }
                DnsLookupOutcome::Answers(
                    answers
                        .into_iter()
                        .map(|answer| {
                            let disposition = self
                                .authorization
                                .classify_answer(&receipt, &request, &answer);
                            DnsAnswerEvidence {
                                answer,
                                disposition,
                            }
                        })
                        .collect(),
                )
            }
        };

        if cancellation.is_cancelled() {
            return Err(DnsResolutionError::Cancelled);
        }
        if !receipt.is_valid_at(Instant::now()) {
            return Err(DnsResolutionError::AuthorizationExpired);
        }

        Ok(DnsResolutionResult {
            host: request.host,
            record_type: request.record_type.0,
            resolution_hops: request.resolution_hops,
            authorization_id: receipt.authorization_id,
            scope_digest: receipt.scope_digest,
            outcome,
        })
    }
}
