//! Registro efímero de autorizaciones DNS para consultas CHEF explícitas.
//!
//! La aplicación solo debe registrar una concesión después de verificar el
//! consentimiento de la persona y la política vigente del proyecto. El registro
//! limita objetivo, tipos de registro, duración y número de consultas; no concede
//! autorización recursiva a las respuestas recibidas.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::IpAddr,
    sync::Mutex,
    time::{Duration, Instant},
};

use chef_application::{
    DnsAnswer, DnsAnswerDisposition, DnsAuthorizationControl, DnsAuthorizationFailure,
    DnsAuthorizationPort, DnsAuthorizationReceipt, DnsQueryRequest, DnsRecordType,
};
use chef_core::{ChefTargetKind, parse_chef_target_seed};

const MAX_ACTIVE_GRANTS: usize = 256;
const MAX_QUERY_BUDGET: usize = 512;
const MAX_GRANT_DURATION: Duration = Duration::from_secs(60 * 60);
const SUPPORTED_RECORD_TYPES: [&str; 10] = [
    "A", "AAAA", "CNAME", "MX", "NS", "PTR", "SOA", "SRV", "TXT", "CAA",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsAuthorizationRegistryError {
    InvalidGrant,
    Unavailable,
}

/// Concesión temporal creada tras consentimiento y validación de scope.
#[derive(Debug, Clone)]
pub struct DnsAuthorizationGrant {
    scope_host: String,
    authorization_id: String,
    scope_digest: String,
    record_types: BTreeSet<String>,
    max_queries: usize,
    expires_at: Instant,
    control: DnsAuthorizationControl,
}

impl DnsAuthorizationGrant {
    pub fn new(
        scope_host: String,
        authorization_id: String,
        scope_digest: String,
        record_types: Vec<String>,
        max_queries: usize,
        expires_at: Instant,
    ) -> Result<Self, DnsAuthorizationRegistryError> {
        let (scope_host, is_ip) =
            canonical_scope_host(&scope_host).ok_or(DnsAuthorizationRegistryError::InvalidGrant)?;
        if !valid_authorization_id(&authorization_id)
            || scope_digest.len() != 64
            || !scope_digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            || max_queries == 0
            || max_queries > MAX_QUERY_BUDGET
        {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }
        let now = Instant::now();
        let duration = expires_at
            .checked_duration_since(now)
            .ok_or(DnsAuthorizationRegistryError::InvalidGrant)?;
        if duration.is_zero() || duration > MAX_GRANT_DURATION {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }

        let mut normalized_types = BTreeSet::new();
        for value in record_types {
            let parsed = DnsRecordType::parse(&value)
                .map_err(|_| DnsAuthorizationRegistryError::InvalidGrant)?;
            let record_type = parsed.as_str();
            if !SUPPORTED_RECORD_TYPES.contains(&record_type)
                || (is_ip && record_type != "PTR")
                || (!is_ip && record_type == "PTR")
                || !normalized_types.insert(record_type.to_owned())
            {
                return Err(DnsAuthorizationRegistryError::InvalidGrant);
            }
        }
        if normalized_types.is_empty() {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }

        Ok(Self {
            scope_host,
            authorization_id,
            scope_digest: scope_digest.to_ascii_lowercase(),
            record_types: normalized_types,
            max_queries,
            expires_at,
            control: DnsAuthorizationControl::new(),
        })
    }

    pub fn scope_host(&self) -> &str {
        &self.scope_host
    }

    pub fn authorization_id(&self) -> &str {
        &self.authorization_id
    }
}

#[derive(Debug, Clone)]
struct ActiveGrant {
    grant: DnsAuthorizationGrant,
    queries_used: usize,
}

/// Registro vacío al inicio; no lee configuración ni comienza consultas DNS.
#[derive(Debug, Default)]
pub struct DnsAuthorizationRegistry {
    grants: Mutex<BTreeMap<String, ActiveGrant>>,
}

impl DnsAuthorizationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        grant: DnsAuthorizationGrant,
    ) -> Result<(), DnsAuthorizationRegistryError> {
        if grant.expires_at <= Instant::now() || grant.control.is_revoked() {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| DnsAuthorizationRegistryError::Unavailable)?;
        prune_expired(&mut grants);
        if grants
            .get(grant.scope_host())
            .is_some_and(|current| current.grant.authorization_id == grant.authorization_id)
        {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }
        if !grants.contains_key(grant.scope_host()) && grants.len() >= MAX_ACTIVE_GRANTS {
            return Err(DnsAuthorizationRegistryError::InvalidGrant);
        }
        if let Some(previous) = grants.insert(
            grant.scope_host.clone(),
            ActiveGrant {
                grant,
                queries_used: 0,
            },
        ) {
            previous.grant.control.revoke();
        }
        Ok(())
    }

    pub fn revoke_scope(&self, scope_host: &str) -> Result<bool, DnsAuthorizationRegistryError> {
        let scope_host = canonical_scope_host(scope_host)
            .map(|(host, _)| host)
            .ok_or(DnsAuthorizationRegistryError::InvalidGrant)?;
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| DnsAuthorizationRegistryError::Unavailable)?;
        if let Some(grant) = grants.remove(&scope_host) {
            grant.grant.control.revoke();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn revoke_all(&self) -> Result<usize, DnsAuthorizationRegistryError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| DnsAuthorizationRegistryError::Unavailable)?;
        let revoked = grants.len();
        for grant in grants.values() {
            grant.grant.control.revoke();
        }
        grants.clear();
        Ok(revoked)
    }
}

impl DnsAuthorizationPort for DnsAuthorizationRegistry {
    fn authorize_query(
        &self,
        request: &DnsQueryRequest,
    ) -> Result<DnsAuthorizationReceipt, DnsAuthorizationFailure> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| DnsAuthorizationFailure::NotAuthorized)?;
        prune_expired(&mut grants);
        let active = grants
            .get_mut(request.scope_host())
            .ok_or(DnsAuthorizationFailure::NotAuthorized)?;
        if active.grant.expires_at <= Instant::now()
            || active.grant.control.is_revoked()
            || !active
                .grant
                .record_types
                .contains(request.record_type().as_str())
            || active.queries_used >= active.grant.max_queries
        {
            return Err(DnsAuthorizationFailure::NotAuthorized);
        }
        active.queries_used += 1;
        DnsAuthorizationReceipt::new_with_control(
            active.grant.authorization_id.clone(),
            active.grant.scope_digest.clone(),
            active.grant.expires_at,
            active.grant.control.clone(),
        )
    }

    fn classify_answer(
        &self,
        _receipt: &DnsAuthorizationReceipt,
        _request: &DnsQueryRequest,
        _answer: &DnsAnswer,
    ) -> DnsAnswerDisposition {
        // Los resultados DNS nunca amplían el grant ni autorizan recursión.
        DnsAnswerDisposition::EvidenceOnly
    }
}

fn canonical_scope_host(value: &str) -> Option<(String, bool)> {
    let seed = parse_chef_target_seed(value).ok()?;
    let is_ip = matches!(seed.kind, ChefTargetKind::IpAddress);
    if !is_ip && seed.kind != ChefTargetKind::DnsName {
        return None;
    }
    let host = seed.host?;
    if is_ip && host.parse::<IpAddr>().is_err() {
        return None;
    }
    Some((host, is_ip))
}

fn valid_authorization_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':'))
}

fn prune_expired(grants: &mut BTreeMap<String, ActiveGrant>) {
    grants.retain(|_, active| {
        let valid = active.grant.expires_at > Instant::now() && !active.grant.control.is_revoked();
        if !valid {
            active.grant.control.revoke();
        }
        valid
    });
}
