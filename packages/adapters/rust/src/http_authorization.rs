//! Registro local en memoria de concesiones HTTP explícitas.
//!
//! No resuelve DNS ni verifica por sí solo la procedencia del scope. La capa
//! que registra una concesión debe haber obtenido consentimiento explícito,
//! validado el scope vigente y resuelto el host bajo esa política. Cada
//! conexión queda fijada a las IP entregadas en esa concesión.

use std::{collections::BTreeMap, net::IpAddr, sync::Mutex, time::Instant};

use chef_application::{
    HttpAuthorizationControl, HttpAuthorizationFailure, HttpAuthorizationPort,
    HttpAuthorizationReceipt,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpAuthorizationRegistryError {
    InvalidGrant,
    Unavailable,
}

/// Concesión transitoria creada solo después de que la aplicación confirme
/// scope escrito y consentimiento. Las IP deben ser las aprobadas para este
/// origen en la validación DNS inmediatamente anterior a registrar el grant.
#[derive(Debug, Clone)]
pub struct HttpAuthorizationGrant {
    origin: String,
    authorization_id: String,
    scope_digest: String,
    allowed_ips: Vec<IpAddr>,
    expires_at: Instant,
    control: HttpAuthorizationControl,
}

impl HttpAuthorizationGrant {
    pub fn new(
        origin: String,
        authorization_id: String,
        scope_digest: String,
        allowed_ips: Vec<IpAddr>,
        expires_at: Instant,
    ) -> Result<Self, HttpAuthorizationRegistryError> {
        let control = HttpAuthorizationControl::new();
        let receipt = HttpAuthorizationReceipt::new_with_control(
            origin,
            authorization_id,
            scope_digest,
            allowed_ips,
            expires_at,
            control.clone(),
        )
        .map_err(|_| HttpAuthorizationRegistryError::InvalidGrant)?;
        Ok(Self {
            origin: receipt.origin().to_owned(),
            authorization_id: receipt.authorization_id().to_owned(),
            scope_digest: receipt.scope_digest().to_owned(),
            allowed_ips: receipt.allowed_ips().to_vec(),
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
}

/// Implementación fail-closed de la frontera de autorización HTTP.
///
/// Está vacía por defecto y no lee configuración que pudiera autorizar tráfico
/// implícitamente. Revocar un grant cancela las ejecuciones activas que aún
/// conservan su recibo. El registro no persiste concesiones entre sesiones.
#[derive(Debug, Default)]
pub struct HttpAuthorizationRegistry {
    grants: Mutex<BTreeMap<String, HttpAuthorizationGrant>>,
}

impl HttpAuthorizationRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &self,
        grant: HttpAuthorizationGrant,
    ) -> Result<(), HttpAuthorizationRegistryError> {
        if grant.expires_at <= Instant::now() || grant.control.is_revoked() {
            return Err(HttpAuthorizationRegistryError::InvalidGrant);
        }
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| HttpAuthorizationRegistryError::Unavailable)?;
        if grants
            .get(grant.origin())
            .is_some_and(|current| current.authorization_id == grant.authorization_id)
        {
            return Err(HttpAuthorizationRegistryError::InvalidGrant);
        }
        if let Some(previous) = grants.insert(grant.origin.clone(), grant.clone()) {
            previous.control.revoke();
        }
        Ok(())
    }

    /// Revoca el origen exacto y también los recibos emitidos previamente.
    pub fn revoke_origin(
        &self,
        canonical_origin: &str,
    ) -> Result<bool, HttpAuthorizationRegistryError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| HttpAuthorizationRegistryError::Unavailable)?;
        if let Some(grant) = grants.remove(canonical_origin) {
            grant.control.revoke();
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn revoke_all(&self) -> Result<usize, HttpAuthorizationRegistryError> {
        let mut grants = self
            .grants
            .lock()
            .map_err(|_| HttpAuthorizationRegistryError::Unavailable)?;
        let revoked = grants.len();
        for grant in grants.values() {
            grant.control.revoke();
        }
        grants.clear();
        Ok(revoked)
    }
}

impl HttpAuthorizationPort for HttpAuthorizationRegistry {
    async fn authorize_origin(
        &self,
        origin: &str,
    ) -> Result<HttpAuthorizationReceipt, HttpAuthorizationFailure> {
        let grant = {
            let mut grants = self
                .grants
                .lock()
                .map_err(|_| HttpAuthorizationFailure::NotAuthorized)?;
            match grants.get(origin) {
                Some(grant) if grant.expires_at > Instant::now() && !grant.control.is_revoked() => {
                    Some(grant.clone())
                }
                Some(_) => {
                    if let Some(expired) = grants.remove(origin) {
                        expired.control.revoke();
                    }
                    None
                }
                None => None,
            }
        }
        .ok_or(HttpAuthorizationFailure::NotAuthorized)?;

        HttpAuthorizationReceipt::new_with_control(
            grant.origin,
            grant.authorization_id,
            grant.scope_digest,
            grant.allowed_ips,
            grant.expires_at,
            grant.control,
        )
    }
}
