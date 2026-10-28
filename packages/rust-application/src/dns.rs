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
    scope_host: String,
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
            scope_host: host.clone(),
            host,
            record_type: DnsRecordType::parse(record_type)?,
            resolution_hops,
        })
    }

    /// Construye una consulta SRV para un propietario del catálogo CHEF.
    ///
    /// `scope_host` conserva el dominio raíz que debe autorizar la aplicación;
    /// el propietario con etiquetas `_servicio._protocolo` no se usa como scope.
    pub fn for_srv_owner(
        owner_name: &str,
        scope_host: &str,
        resolution_hops: u8,
    ) -> Result<Self, DnsResolutionError> {
        if resolution_hops > 32 {
            return Err(DnsResolutionError::LimitExceeded);
        }
        let seed =
            parse_chef_target_seed(scope_host).map_err(|_| DnsResolutionError::InvalidQuery)?;
        if seed.kind != ChefTargetKind::DnsName {
            return Err(DnsResolutionError::InvalidQuery);
        }
        let scope_host = seed.host.ok_or(DnsResolutionError::InvalidQuery)?;
        if !chef_core::common_dns_service_candidates(&scope_host)
            .map_err(|_| DnsResolutionError::InvalidQuery)?
            .iter()
            .any(|candidate| candidate == owner_name)
        {
            return Err(DnsResolutionError::InvalidQuery);
        }
        Ok(Self {
            host: owner_name.to_owned(),
            scope_host,
            record_type: DnsRecordType::parse("SRV")?,
            resolution_hops,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    /// Dominio de alcance al que pertenece la solicitud DNS.
    pub fn scope_host(&self) -> &str {
        &self.scope_host
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

#[derive(Debug, Default)]
struct DnsAuthorizationState {
    revoked: AtomicBool,
    notify: Notify,
}

/// Control compartido para revocar autorizaciones y detener consultas activas.
#[derive(Debug, Clone, Default)]
pub struct DnsAuthorizationControl(Arc<DnsAuthorizationState>);

impl DnsAuthorizationControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revoke(&self) {
        self.0.revoked.store(true, Ordering::Release);
        self.0.notify.notify_waiters();
    }

    pub fn is_revoked(&self) -> bool {
        self.0.revoked.load(Ordering::Acquire)
    }

    async fn revoked(&self) {
        loop {
            let notified = self.0.notify.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            if self.is_revoked() {
                return;
            }
            notified.await;
        }
    }
}

#[derive(Debug, Clone)]
pub struct DnsAuthorizationReceipt {
    authorization_id: String,
    scope_digest: String,
    expires_at: Instant,
    control: DnsAuthorizationControl,
}

impl DnsAuthorizationReceipt {
    /// Lo construye únicamente el adaptador de autorización tras confirmación explícita.
    pub fn new(
        authorization_id: String,
        scope_digest: String,
        expires_at: Instant,
    ) -> Result<Self, DnsAuthorizationFailure> {
        Self::new_with_control(
            authorization_id,
            scope_digest,
            expires_at,
            DnsAuthorizationControl::new(),
        )
    }

    pub fn new_with_control(
        authorization_id: String,
        scope_digest: String,
        expires_at: Instant,
        control: DnsAuthorizationControl,
    ) -> Result<Self, DnsAuthorizationFailure> {
        if authorization_id.is_empty()
            || authorization_id.len() > MAX_AUTHORIZATION_ID_BYTES
            || scope_digest.len() != 64
            || !scope_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || expires_at <= Instant::now()
            || control.is_revoked()
        {
            return Err(DnsAuthorizationFailure::InvalidReceipt);
        }
        Ok(Self {
            authorization_id,
            scope_digest: scope_digest.to_ascii_lowercase(),
            expires_at,
            control,
        })
    }

    pub fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub fn scope_digest(&self) -> &str {
        &self.scope_digest
    }

    pub fn is_valid_at(&self, now: Instant) -> bool {
        now < self.expires_at && !self.control.is_revoked()
    }

    fn expires_at(&self) -> Instant {
        self.expires_at
    }

    async fn revoked(&self) {
        self.control.revoked().await;
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
    /// Autoriza el dominio de `request.scope_host()` en la política vigente.
    ///
    /// Para propietarios SRV CHEF, `request.host()` es un nombre de consulta
    /// con etiquetas de servicio; no debe reemplazar al dominio de scope.
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
    pub scope_host: String,
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
    AuthorizationRevoked,
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
            Self::AuthorizationRevoked => "La autorización DNS se revocó durante la consulta",
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
    /// de alcance como evidencia sin habilitar seguimiento recursivo. El adaptador
    /// debe autorizar `request.scope_host()` y validar allí cualquier subconsulta.
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
        if receipt.control.is_revoked() {
            return Err(DnsResolutionError::AuthorizationRevoked);
        }
        if !receipt.is_valid_at(Instant::now()) {
            return Err(DnsResolutionError::InvalidAuthorization);
        }

        let expires_at = tokio::time::Instant::from_std(receipt.expires_at());
        let lookup = tokio::select! {
            _ = cancellation.cancelled() => return Err(DnsResolutionError::Cancelled),
            _ = receipt.revoked() => return Err(DnsResolutionError::AuthorizationRevoked),
            _ = tokio::time::sleep_until(expires_at) => {
                return Err(DnsResolutionError::AuthorizationExpired);
            },
            result = self.resolver.lookup(&request) => result,
        };
        if cancellation.is_cancelled() {
            return Err(DnsResolutionError::Cancelled);
        }
        if receipt.control.is_revoked() {
            return Err(DnsResolutionError::AuthorizationRevoked);
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
        if receipt.control.is_revoked() {
            return Err(DnsResolutionError::AuthorizationRevoked);
        }
        if !receipt.is_valid_at(Instant::now()) {
            return Err(DnsResolutionError::AuthorizationExpired);
        }

        Ok(DnsResolutionResult {
            host: request.host,
            scope_host: request.scope_host,
            record_type: request.record_type.0,
            resolution_hops: request.resolution_hops,
            authorization_id: receipt.authorization_id,
            scope_digest: receipt.scope_digest,
            outcome,
        })
    }
}
