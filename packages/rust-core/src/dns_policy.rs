//! Reglas locales de omisión y circuit breaker adaptadas de `DNSHelper` BBOT.
//!
//! El adaptador aporta el resultado de conectividad; este módulo no consulta red,
//! no lee configuración y mantiene acotado el estado por scan.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_OMIT_ENTRIES: usize = 1024;
const MAX_ERROR_KEYS: usize = 10_000;
const MAX_WARNED_KEYS: usize = 10_000;
const DEFAULT_ABORT_THRESHOLD: u32 = 50;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DnsErrorKey {
    parent: String,
    record_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ErrorEntry {
    consecutive: u32,
    sequence: u64,
}

/// Resultado del gate al superar el número de errores por zona y tipo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsZoneDecision {
    Continue,
    Abort { first_warning: bool },
}

/// Política por trabajo para DNS omitido y fallos consecutivos.
#[derive(Debug, Clone)]
pub struct DnsQueryPolicy {
    omitted: BTreeMap<String, BTreeSet<String>>,
    abort_threshold: u32,
    errors: BTreeMap<DnsErrorKey, ErrorEntry>,
    warned: BTreeSet<DnsErrorKey>,
    next_sequence: u64,
}

impl Default for DnsQueryPolicy {
    fn default() -> Self {
        Self {
            omitted: BTreeMap::new(),
            abort_threshold: DEFAULT_ABORT_THRESHOLD,
            errors: BTreeMap::new(),
            warned: BTreeSet::new(),
            next_sequence: 1,
        }
    }
}

impl DnsQueryPolicy {
    /// Valida y canonicaliza entradas `TIPO:dominio` de `dns.omit_queries`.
    pub fn new(omit_queries: &[String]) -> ChefResult<Self> {
        Self::with_abort_threshold(omit_queries, DEFAULT_ABORT_THRESHOLD)
    }

    pub fn with_abort_threshold(omit_queries: &[String], abort_threshold: u32) -> ChefResult<Self> {
        if omit_queries.len() > MAX_OMIT_ENTRIES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La lista de consultas DNS omitidas supera el máximo CHEF",
            ));
        }
        if abort_threshold == 0 {
            return Err(invalid_dns_policy());
        }
        let mut policy = Self {
            abort_threshold,
            ..Self::default()
        };
        for entry in omit_queries {
            let (record_type, host) = entry.split_once(':').ok_or_else(invalid_dns_policy)?;
            let record_type = normalize_record_type(record_type).ok_or_else(invalid_dns_policy)?;
            let host = crate::host::canonical_host(host).ok_or_else(invalid_dns_policy)?;
            policy.omitted.entry(record_type).or_default().insert(host);
        }
        Ok(policy)
    }

    /// Coincide con el host omitido o cualquiera de sus subdominios.
    pub fn is_omitted(&self, query: &str, record_type: &str) -> bool {
        let Some(query) = crate::host::canonical_host(query) else {
            return false;
        };
        let Some(record_type) = normalize_record_type(record_type) else {
            return false;
        };
        self.omitted.get(&record_type).is_some_and(|hosts| {
            hosts.iter().any(|host| {
                query == *host
                    || (query.parse::<std::net::IpAddr>().is_err()
                        && host.parse::<std::net::IpAddr>().is_err()
                        && query.ends_with(&format!(".{host}")))
            })
        })
    }

    /// Registra un error para la misma clave de zona/type que usa BBOT.
    pub fn record_dns_error(&mut self, query: &str, record_type: &str) -> ChefResult<u32> {
        let key = error_key(query, record_type).ok_or_else(invalid_dns_policy)?;
        if let Some(entry) = self.errors.get_mut(&key) {
            entry.consecutive = entry.consecutive.saturating_add(1);
            entry.sequence = self.next_sequence;
        } else {
            if self.errors.len() >= MAX_ERROR_KEYS
                && let Some(oldest) = self
                    .errors
                    .iter()
                    .min_by_key(|(candidate, entry)| (entry.sequence, *candidate))
                    .map(|(candidate, _)| candidate.clone())
            {
                self.errors.remove(&oldest);
                self.warned.remove(&oldest);
            }
            self.errors.insert(
                key.clone(),
                ErrorEntry {
                    consecutive: 1,
                    sequence: self.next_sequence,
                },
            );
        }
        self.next_sequence = self.next_sequence.saturating_add(1);
        Ok(self.errors[&key].consecutive)
    }

    /// Reinicia errores solo si hubo respuesta DNS con al menos una respuesta,
    /// igual que `resolve_multi_full` en BBOT.
    pub fn record_answer(&mut self, query: &str, record_type: &str) -> ChefResult<()> {
        let key = error_key(query, record_type).ok_or_else(invalid_dns_policy)?;
        if let Some(entry) = self.errors.get_mut(&key) {
            entry.consecutive = 0;
            entry.sequence = self.next_sequence;
            self.next_sequence = self.next_sequence.saturating_add(1);
        }
        Ok(())
    }

    /// Decide si se pausa una zona al superar el umbral.
    ///
    /// La app entrega `resolver_reachable`, obtenido por el adaptador. Si no
    /// hay conectividad, se limpian los errores globales y no se culpa a la zona.
    pub fn zone_decision(
        &mut self,
        query: &str,
        record_type: &str,
        resolver_reachable: bool,
    ) -> ChefResult<DnsZoneDecision> {
        let key = error_key(query, record_type).ok_or_else(invalid_dns_policy)?;
        if self
            .errors
            .get(&key)
            .is_none_or(|entry| entry.consecutive < self.abort_threshold)
        {
            return Ok(DnsZoneDecision::Continue);
        }
        if !resolver_reachable {
            self.errors.clear();
            return Ok(DnsZoneDecision::Continue);
        }
        if self.warned.len() >= MAX_WARNED_KEYS && !self.warned.contains(&key) {
            self.warned.pop_first();
        }
        let first_warning = self.warned.insert(key);
        Ok(DnsZoneDecision::Abort { first_warning })
    }

    pub fn abort_threshold(&self) -> u32 {
        self.abort_threshold
    }
}

fn error_key(query: &str, record_type: &str) -> Option<DnsErrorKey> {
    let query = crate::host::canonical_host(query)?;
    let record_type = normalize_record_type(record_type)?;
    Some(DnsErrorKey {
        parent: parent_domain(&query),
        record_type,
    })
}

fn parent_domain(host: &str) -> String {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return host.to_owned();
    }
    let Some(registrable) = psl::domain(host.as_bytes()) else {
        return remove_first_label_when_nested(host);
    };
    if registrable.suffix().typ() != Some(psl::Type::Icann) {
        return remove_first_label_when_nested(host);
    }
    let registrable = String::from_utf8_lossy(registrable.as_bytes());
    if host == registrable {
        host.to_owned()
    } else if host.ends_with(&format!(".{registrable}")) {
        remove_first_label_when_nested(host)
    } else {
        host.to_owned()
    }
}

fn remove_first_label_when_nested(host: &str) -> String {
    if host.matches('.').count() > 1 {
        host.split_once('.')
            .map_or_else(|| host.to_owned(), |(_, tail)| tail.to_owned())
    } else {
        host.to_owned()
    }
}

fn normalize_record_type(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 16
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }
    Some(value.to_ascii_uppercase())
}

fn invalid_dns_policy() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "La política local de consultas DNS no es válida",
    )
}
