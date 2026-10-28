//! Ejecución HTTP detrás de autorización y transporte inyectados.
//!
//! Este caso de uso no resuelve DNS ni abre sockets. La autorización debe
//! entregar las IP aprobadas para el destino exacto y el adaptador de
//! transporte debe fijar la conexión a ellas, sin seguir redirects.

use std::{
    collections::BTreeSet,
    future::Future,
    net::IpAddr,
    sync::Arc,
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, Notify, Semaphore, SemaphorePermit};
use url::Url;

use crate::{HttpRequestPlan, PlannedHttpRequest};

const MAX_AUTHORIZATION_ID_BYTES: usize = 128;
const MAX_AUTHORIZED_IPS: usize = 256;
const MIN_REQUEST_INTERVAL: Duration = Duration::from_millis(500);
const MAX_REDIRECT_LOCATION_BYTES: usize = 8 * 1024;
const MAX_CONCURRENT_HTTP_REQUESTS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpAuthorizationFailure {
    NotAuthorized,
    InvalidReceipt,
}

#[derive(Debug)]
struct HttpAuthorizationState {
    revoked: tokio::sync::watch::Sender<bool>,
}

/// Control local de una concesión explícita. Revocarlo bloquea nuevas
/// autorizaciones y cancela la ejecución que ya use su recibo.
#[derive(Debug, Clone)]
pub struct HttpAuthorizationControl(Arc<HttpAuthorizationState>);

impl Default for HttpAuthorizationControl {
    fn default() -> Self {
        let (revoked, _) = tokio::sync::watch::channel(false);
        Self(Arc::new(HttpAuthorizationState { revoked }))
    }
}

impl HttpAuthorizationControl {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn revoke(&self) {
        self.0.revoked.send_replace(true);
    }

    pub fn is_revoked(&self) -> bool {
        *self.0.revoked.borrow()
    }

    async fn revoked(&self) {
        let mut receiver = self.0.revoked.subscribe();
        while !*receiver.borrow() {
            if receiver.changed().await.is_err() {
                return;
            }
        }
    }
}

/// Autorización vinculada a un origen concreto y a las IP que el adaptador
/// validó contra el scope vigente. Solo el adaptador de autorización debe
/// construirla tras confirmar permiso explícito.
#[derive(Debug, Clone)]
pub struct HttpAuthorizationReceipt {
    origin: String,
    authorization_id: String,
    scope_digest: String,
    allowed_ips: Vec<IpAddr>,
    expires_at: Instant,
    control: HttpAuthorizationControl,
}

impl HttpAuthorizationReceipt {
    pub fn new(
        origin: String,
        authorization_id: String,
        scope_digest: String,
        allowed_ips: Vec<IpAddr>,
        expires_at: Instant,
    ) -> Result<Self, HttpAuthorizationFailure> {
        Self::new_with_control(
            origin,
            authorization_id,
            scope_digest,
            allowed_ips,
            expires_at,
            HttpAuthorizationControl::new(),
        )
    }

    pub fn new_with_control(
        origin: String,
        authorization_id: String,
        scope_digest: String,
        mut allowed_ips: Vec<IpAddr>,
        expires_at: Instant,
        control: HttpAuthorizationControl,
    ) -> Result<Self, HttpAuthorizationFailure> {
        let Some(canonical_origin) = canonical_http_origin(&origin) else {
            return Err(HttpAuthorizationFailure::InvalidReceipt);
        };
        if canonical_origin != origin
            || authorization_id.is_empty()
            || authorization_id.len() > MAX_AUTHORIZATION_ID_BYTES
            || authorization_id.chars().any(char::is_control)
            || scope_digest.len() != 64
            || !scope_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || allowed_ips.is_empty()
            || allowed_ips.len() > MAX_AUTHORIZED_IPS
            || expires_at <= Instant::now()
            || control.is_revoked()
        {
            return Err(HttpAuthorizationFailure::InvalidReceipt);
        }
        allowed_ips.sort_unstable();
        allowed_ips.dedup();
        Ok(Self {
            origin: canonical_origin,
            authorization_id,
            scope_digest: scope_digest.to_ascii_lowercase(),
            allowed_ips,
            expires_at,
            control,
        })
    }

    pub fn origin(&self) -> &str {
        &self.origin
    }

    pub fn authorization_id(&self) -> &str {
        &self.authorization_id
    }

    pub fn scope_digest(&self) -> &str {
        &self.scope_digest
    }

    pub fn allowed_ips(&self) -> &[IpAddr] {
        &self.allowed_ips
    }

    pub fn is_valid_at(&self, now: Instant) -> bool {
        now < self.expires_at && !self.control.is_revoked()
    }

    fn expires_at(&self) -> Instant {
        self.expires_at
    }
}

pub trait HttpAuthorizationPort: Send + Sync {
    /// Valida scope vigente y resuelve el host; rechaza si alguna IP de destino
    /// no está permitida. El recibo debe quedar ligado al origen solicitado.
    fn authorize_origin<'a>(
        &'a self,
        origin: &'a str,
    ) -> impl Future<Output = Result<HttpAuthorizationReceipt, HttpAuthorizationFailure>> + Send + 'a;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpTransportFailure {
    Dns,
    Connect,
    Tls,
    Timeout,
    InvalidResponse,
    Other,
}

/// Solicitud efímera para un transporte que conecta únicamente a `allowed_ips`
/// y devuelve el peer IP real. El transporte no sigue redirects ni conserva el
/// cuerpo; debe detener la lectura al llegar a `max_response_bytes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpTransportRequest {
    url: String,
    allowed_ips: Vec<IpAddr>,
    max_response_bytes: usize,
    timeout: Duration,
}

impl HttpTransportRequest {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn method(&self) -> &'static str {
        "GET"
    }

    pub fn allowed_ips(&self) -> &[IpAddr] {
        &self.allowed_ips
    }

    pub fn max_response_bytes(&self) -> usize {
        self.max_response_bytes
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpTransportResponse {
    pub status: u16,
    pub peer_ip: IpAddr,
    /// Bytes de cuerpo leídos de forma transitoria; el contenido no se retiene.
    pub response_bytes: usize,
    pub location: Option<String>,
}

pub trait HttpTransportPort: Send + Sync {
    /// Debe ejecutar exactamente una petición y respetar IP, timeout y límite
    /// entregados. No debe seguir redirects ni exponer cuerpo o cabeceras.
    fn get_once(
        &self,
        request: HttpTransportRequest,
    ) -> impl Future<Output = Result<HttpTransportResponse, HttpTransportFailure>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpExecutionError {
    NotAuthorized,
    InvalidAuthorization,
    AuthorizationRevoked,
    AuthorizationExpired,
    AuthorizationTimeout,
    Cancelled,
    JobTimeout,
    RequestLimit,
    InvalidRedirect,
    RedirectLimit,
    UnauthorizedPeer,
    ResponseLimit,
}

impl std::fmt::Display for HttpExecutionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::NotAuthorized => "El origen HTTP no está autorizado para este scope",
            Self::InvalidAuthorization => "La autorización HTTP no es válida",
            Self::AuthorizationRevoked => "La autorización HTTP fue revocada",
            Self::AuthorizationExpired => "La autorización HTTP expiró",
            Self::AuthorizationTimeout => "La autorización HTTP superó su tiempo máximo",
            Self::Cancelled => "La ejecución HTTP fue cancelada",
            Self::JobTimeout => "La ejecución HTTP superó su tiempo máximo",
            Self::RequestLimit => "La ejecución HTTP superó el máximo de solicitudes",
            Self::InvalidRedirect => "La redirección HTTP no cumple la política CHEF",
            Self::RedirectLimit => "La cadena HTTP superó el máximo de redirecciones",
            Self::UnauthorizedPeer => "La conexión HTTP alcanzó una IP no autorizada",
            Self::ResponseLimit => "La respuesta HTTP superó el máximo de bytes",
        })
    }
}

impl std::error::Error for HttpExecutionError {}

#[derive(Debug, Default)]
struct CancellationState {
    cancelled: std::sync::atomic::AtomicBool,
    notify: Notify,
}

/// Señal clonable de cancelación cooperativa para un trabajo HTTP.
#[derive(Debug, Clone, Default)]
pub struct HttpCancellation(Arc<CancellationState>);

impl HttpCancellation {
    pub fn cancel(&self) {
        self.0
            .cancelled
            .store(true, std::sync::atomic::Ordering::Release);
        self.0.notify.notify_one();
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(std::sync::atomic::Ordering::Acquire)
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HttpRequestOutcome {
    Response {
        status: u16,
        final_origin: String,
        redirects_followed: u8,
        authorization_id: String,
        scope_digest: String,
    },
    TransportFailed(HttpTransportFailure),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestResult {
    pub planned_origin: String,
    pub source_event_ids: Vec<String>,
    pub outcome: HttpRequestOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpExecutionTermination {
    Completed,
    Stopped(HttpExecutionError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpPlanExecution {
    pub results: Vec<HttpRequestResult>,
    /// Trazabilidad de autorización incluso si el trabajo se detiene a mitad
    /// de una cadena de redirects.
    pub authorization_uses: Vec<HttpAuthorizationUse>,
    pub termination: HttpExecutionTermination,
}

struct HttpExecutionState {
    job_deadline: tokio::time::Instant,
    attempts: usize,
    max_attempts: usize,
    authorization_uses: Vec<HttpAuthorizationUse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpAuthorizationUse {
    pub source_event_ids: Vec<String>,
    pub hop: u8,
    pub origin: String,
    pub authorization_id: String,
    pub scope_digest: String,
    pub allowed_ips: Vec<IpAddr>,
    pub dispatched: bool,
    pub response_status: Option<u16>,
}

/// Gate compartido entre trabajos: máximo cuatro solicitudes concurrentes y
/// separación global mínima de 500 ms entre despachos (2 solicitudes/s).
/// La composición debe compartir una instancia por proceso.
#[derive(Debug, Clone)]
pub struct HttpExecutionGate {
    permits: Arc<Semaphore>,
    last_dispatch: Arc<Mutex<Option<tokio::time::Instant>>>,
}

impl Default for HttpExecutionGate {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_HTTP_REQUESTS)),
            last_dispatch: Arc::new(Mutex::new(None)),
        }
    }
}

impl HttpExecutionGate {
    async fn acquire(
        &self,
        cancellation: &HttpCancellation,
        job_deadline: tokio::time::Instant,
    ) -> Result<SemaphorePermit<'_>, HttpExecutionError> {
        tokio::select! {
            _ = cancellation.cancelled() => Err(HttpExecutionError::Cancelled),
            _ = tokio::time::sleep_until(job_deadline) => Err(HttpExecutionError::JobTimeout),
            result = self.permits.acquire() => result.map_err(|_| HttpExecutionError::Cancelled),
        }
    }

    async fn wait_for_turn(
        &self,
        cancellation: &HttpCancellation,
        authorization: &HttpAuthorizationControl,
        job_deadline: tokio::time::Instant,
    ) -> Result<(), HttpExecutionError> {
        let mut last_dispatch = tokio::select! {
            _ = cancellation.cancelled() => return Err(HttpExecutionError::Cancelled),
            _ = authorization.revoked() => return Err(HttpExecutionError::AuthorizationRevoked),
            _ = tokio::time::sleep_until(job_deadline) => return Err(HttpExecutionError::JobTimeout),
            guard = self.last_dispatch.lock() => guard,
        };
        if let Some(last) = *last_dispatch {
            let ready_at = last + MIN_REQUEST_INTERVAL;
            if tokio::time::Instant::now() < ready_at {
                tokio::select! {
                    _ = cancellation.cancelled() => return Err(HttpExecutionError::Cancelled),
                    _ = authorization.revoked() => return Err(HttpExecutionError::AuthorizationRevoked),
                    _ = tokio::time::sleep_until(job_deadline) => return Err(HttpExecutionError::JobTimeout),
                    _ = tokio::time::sleep_until(ready_at) => {},
                }
            }
        }
        *last_dispatch = Some(tokio::time::Instant::now());
        Ok(())
    }
}

pub struct ExecuteHttpPlan<'a, T, A> {
    transport: &'a T,
    authorization: &'a A,
    gate: &'a HttpExecutionGate,
}

impl<'a, T, A> ExecuteHttpPlan<'a, T, A>
where
    T: HttpTransportPort,
    A: HttpAuthorizationPort,
{
    pub fn new(transport: &'a T, authorization: &'a A, gate: &'a HttpExecutionGate) -> Self {
        Self {
            transport,
            authorization,
            gate,
        }
    }

    /// Ejecuta secuencialmente el plan; cada origen y redirect requiere un
    /// recibo nuevo. El caso de uso no está compuesto en Tauri.
    pub async fn execute(
        &self,
        plan: &HttpRequestPlan,
        cancellation: &HttpCancellation,
    ) -> HttpPlanExecution {
        let start = tokio::time::Instant::now();
        let mut results = Vec::with_capacity(plan.requests().len());
        let max_attempts = plan
            .settings()
            .max_requests()
            .saturating_mul(usize::from(plan.settings().max_redirects()) + 1);
        let mut state = HttpExecutionState {
            job_deadline: start + plan.settings().job_timeout(),
            attempts: 0,
            max_attempts,
            authorization_uses: Vec::new(),
        };

        for planned in plan.requests() {
            match self
                .execute_request(planned, plan, cancellation, &mut state)
                .await
            {
                Ok(result) => results.push(result),
                Err(error) => {
                    return HttpPlanExecution {
                        results,
                        authorization_uses: state.authorization_uses,
                        termination: HttpExecutionTermination::Stopped(error),
                    };
                }
            }
        }
        HttpPlanExecution {
            results,
            authorization_uses: state.authorization_uses,
            termination: HttpExecutionTermination::Completed,
        }
    }

    async fn execute_request(
        &self,
        planned: &PlannedHttpRequest,
        plan: &HttpRequestPlan,
        cancellation: &HttpCancellation,
        state: &mut HttpExecutionState,
    ) -> Result<HttpRequestResult, HttpExecutionError> {
        let mut url =
            Url::parse(planned.origin()).map_err(|_| HttpExecutionError::InvalidRedirect)?;
        let mut redirects = 0_u8;
        let mut visited = BTreeSet::from([url.as_str().to_owned()]);
        loop {
            if cancellation.is_cancelled() {
                return Err(HttpExecutionError::Cancelled);
            }
            if tokio::time::Instant::now() >= state.job_deadline {
                return Err(HttpExecutionError::JobTimeout);
            }
            if state.attempts >= state.max_attempts {
                return Err(HttpExecutionError::RequestLimit);
            }
            state.attempts += 1;
            // Acotar también una eventual resolución DNS dentro del puerto de
            // autorización; no permitir trabajo externo sin cupo concurrente.
            let permit = self.gate.acquire(cancellation, state.job_deadline).await?;

            let origin = url.origin().ascii_serialization();
            let authorization_deadline =
                tokio::time::Instant::now() + plan.settings().request_timeout();
            let authorization = tokio::select! {
                _ = cancellation.cancelled() => return Err(HttpExecutionError::Cancelled),
                _ = tokio::time::sleep_until(state.job_deadline) => return Err(HttpExecutionError::JobTimeout),
                _ = tokio::time::sleep_until(authorization_deadline) => return Err(HttpExecutionError::AuthorizationTimeout),
                result = self.authorization.authorize_origin(&origin) => result,
            };
            let receipt = authorization.map_err(|_| HttpExecutionError::NotAuthorized)?;
            if receipt.control.is_revoked() {
                return Err(HttpExecutionError::AuthorizationRevoked);
            }
            if receipt.origin() != origin || receipt.allowed_ips().is_empty() {
                return Err(HttpExecutionError::InvalidAuthorization);
            }
            if !receipt.is_valid_at(Instant::now()) {
                return Err(if receipt.control.is_revoked() {
                    HttpExecutionError::AuthorizationRevoked
                } else {
                    HttpExecutionError::InvalidAuthorization
                });
            }
            let authorization_use_index = state.authorization_uses.len();
            state.authorization_uses.push(HttpAuthorizationUse {
                source_event_ids: planned.source_event_ids().to_vec(),
                hop: redirects,
                origin: origin.clone(),
                authorization_id: receipt.authorization_id().to_owned(),
                scope_digest: receipt.scope_digest().to_owned(),
                allowed_ips: receipt.allowed_ips().to_vec(),
                dispatched: false,
                response_status: None,
            });

            self.gate
                .wait_for_turn(cancellation, &receipt.control, state.job_deadline)
                .await?;
            if cancellation.is_cancelled() {
                return Err(HttpExecutionError::Cancelled);
            }
            if tokio::time::Instant::now() >= state.job_deadline {
                return Err(HttpExecutionError::JobTimeout);
            }
            if !receipt.is_valid_at(Instant::now()) {
                return Err(if receipt.control.is_revoked() {
                    HttpExecutionError::AuthorizationRevoked
                } else {
                    HttpExecutionError::AuthorizationExpired
                });
            }
            state.authorization_uses[authorization_use_index].dispatched = true;

            let transport_request = HttpTransportRequest {
                url: url.to_string(),
                allowed_ips: receipt.allowed_ips().to_vec(),
                max_response_bytes: plan.settings().max_response_bytes(),
                timeout: plan.settings().request_timeout(),
            };
            let request_deadline = tokio::time::Instant::now() + plan.settings().request_timeout();
            let auth_deadline = tokio::time::Instant::from_std(receipt.expires_at());
            let response = tokio::select! {
                _ = cancellation.cancelled() => return Err(HttpExecutionError::Cancelled),
                _ = receipt.control.revoked() => return Err(HttpExecutionError::AuthorizationRevoked),
                _ = tokio::time::sleep_until(state.job_deadline) => return Err(HttpExecutionError::JobTimeout),
                _ = tokio::time::sleep_until(auth_deadline) => return Err(HttpExecutionError::AuthorizationExpired),
                _ = tokio::time::sleep_until(request_deadline) => Err(HttpTransportFailure::Timeout),
                result = self.transport.get_once(transport_request) => result,
            };
            drop(permit);

            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    return Ok(HttpRequestResult {
                        planned_origin: planned.origin().to_owned(),
                        source_event_ids: planned.source_event_ids().to_vec(),
                        outcome: HttpRequestOutcome::TransportFailed(error),
                    });
                }
            };
            if cancellation.is_cancelled() {
                return Err(HttpExecutionError::Cancelled);
            }
            if !receipt.is_valid_at(Instant::now()) {
                return Err(if receipt.control.is_revoked() {
                    HttpExecutionError::AuthorizationRevoked
                } else {
                    HttpExecutionError::AuthorizationExpired
                });
            }
            state.authorization_uses[authorization_use_index].response_status =
                Some(response.status);
            if !receipt.allowed_ips().contains(&response.peer_ip) {
                return Err(HttpExecutionError::UnauthorizedPeer);
            }
            if response.response_bytes > plan.settings().max_response_bytes() {
                return Err(HttpExecutionError::ResponseLimit);
            }

            if is_redirect(response.status)
                && let Some(location) = response.location.as_deref()
            {
                if redirects >= plan.settings().max_redirects() {
                    return Err(HttpExecutionError::RedirectLimit);
                }
                let next = validated_redirect(&url, location)?;
                if url.scheme() == "https" && next.scheme() == "http" {
                    return Err(HttpExecutionError::InvalidRedirect);
                }
                if !visited.insert(next.as_str().to_owned()) {
                    return Err(HttpExecutionError::InvalidRedirect);
                }
                url = next;
                redirects = redirects.saturating_add(1);
                continue;
            }

            return Ok(HttpRequestResult {
                planned_origin: planned.origin().to_owned(),
                source_event_ids: planned.source_event_ids().to_vec(),
                outcome: HttpRequestOutcome::Response {
                    status: response.status,
                    final_origin: url.origin().ascii_serialization(),
                    redirects_followed: redirects,
                    authorization_id: receipt.authorization_id().to_owned(),
                    scope_digest: receipt.scope_digest().to_owned(),
                },
            });
        }
    }
}

fn canonical_http_origin(value: &str) -> Option<String> {
    let url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host().is_none()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return None;
    }
    Some(url.origin().ascii_serialization())
}

fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

fn validated_redirect(current: &Url, location: &str) -> Result<Url, HttpExecutionError> {
    if location.is_empty()
        || location.len() > MAX_REDIRECT_LOCATION_BYTES
        || location.contains('\\')
        || location.chars().any(char::is_control)
    {
        return Err(HttpExecutionError::InvalidRedirect);
    }
    let mut next = current
        .join(location)
        .map_err(|_| HttpExecutionError::InvalidRedirect)?;
    if !matches!(next.scheme(), "http" | "https")
        || !next.username().is_empty()
        || next.password().is_some()
        || next.host().is_none()
        || next.query().is_some()
        || next.port_or_known_default() == Some(0)
    {
        return Err(HttpExecutionError::InvalidRedirect);
    }
    next.set_fragment(None);
    Ok(next)
}
