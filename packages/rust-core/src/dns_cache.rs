//! Caché local y acotada para respuestas DNS entregadas por un adaptador.
//!
//! El adaptador aporta el TTL junto con el resultado y el llamador aporta
//! tiempo monotónico acumulado. Este módulo no consulta DNS ni interpreta una
//! respuesta vacía como un tipo particular de error.

use std::collections::BTreeMap;
use std::time::Duration;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_CACHE_CAPACITY: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DnsCacheKey {
    host: String,
    record_type: String,
}

#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    expires_at: Duration,
    last_used: u64,
}

/// Caché TTL por host y tipo de registro, bounded y sin reloj de pared.
///
/// El límite máximo replica el tamaño de las cachés DNS auxiliares locales.
/// Las entradas se expulsan por LRU; los empates se resuelven por clave para
/// mantener resultados reproducibles.
#[derive(Debug, Clone)]
pub struct DnsCache<V> {
    capacity: usize,
    entries: BTreeMap<DnsCacheKey, CacheEntry<V>>,
    sequence: u64,
}

impl<V> DnsCache<V> {
    /// Crea la caché con capacidad entre 1 y 10 000 entradas.
    pub fn new(capacity: usize) -> ChefResult<Self> {
        if capacity == 0 {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La capacidad de la caché DNS debe ser mayor que cero",
            ));
        }
        if capacity > MAX_CACHE_CAPACITY {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La capacidad de la caché DNS supera el máximo CHEF",
            ));
        }
        Ok(Self {
            capacity,
            entries: BTreeMap::new(),
            sequence: 1,
        })
    }

    /// Busca una respuesta no expirada y actualiza su orden de uso.
    pub fn get(&mut self, host: &str, record_type: &str, now: Duration) -> Option<&V> {
        let key = cache_key(host, record_type)?;
        let entry = self.entries.get(&key)?;
        if now >= entry.expires_at {
            self.entries.remove(&key);
            return None;
        }
        let sequence = self.take_sequence();
        self.entries.get_mut(&key)?.last_used = sequence;
        self.entries.get(&key).map(|entry| &entry.value)
    }

    /// Guarda una respuesta provista por el adaptador hasta `now + ttl`.
    ///
    /// TTL cero elimina cualquier valor previo para la clave y no se cachea.
    /// El llamador decide si resultados vacíos o negativos son cacheables.
    pub fn insert(
        &mut self,
        host: &str,
        record_type: &str,
        value: V,
        ttl: Duration,
        now: Duration,
    ) -> ChefResult<()> {
        let key = cache_key(host, record_type).ok_or_else(invalid_key)?;
        if ttl.is_zero() {
            self.entries.remove(&key);
            return Ok(());
        }
        let expires_at = now.checked_add(ttl).ok_or_else(|| {
            ChefError::new(
                ErrorCode::LimitExceeded,
                "La expiración de la caché DNS supera el rango monotónico",
            )
        })?;
        let sequence = self.take_sequence();
        self.entries.retain(|_, entry| now < entry.expires_at);
        if !self.entries.contains_key(&key)
            && self.entries.len() >= self.capacity
            && let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(candidate, entry)| (entry.last_used, *candidate))
                .map(|(candidate, _)| candidate.clone())
        {
            self.entries.remove(&oldest);
        }
        self.entries.insert(
            key,
            CacheEntry {
                value,
                expires_at,
                last_used: sequence,
            },
        );
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn take_sequence(&mut self) -> u64 {
        if self.sequence == u64::MAX {
            let mut ordered = self
                .entries
                .iter()
                .map(|(key, entry)| (entry.last_used, key.clone()))
                .collect::<Vec<_>>();
            ordered.sort();
            for (sequence, (_, key)) in ordered.into_iter().enumerate() {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.last_used = sequence as u64 + 1;
                }
            }
            self.sequence = self.entries.len() as u64 + 1;
        }
        let sequence = self.sequence;
        self.sequence += 1;
        sequence
    }
}

fn cache_key(host: &str, record_type: &str) -> Option<DnsCacheKey> {
    let host = crate::host::canonical_host(host)?;
    let record_type = record_type.trim();
    if record_type.is_empty()
        || record_type.len() > 16
        || !record_type
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return None;
    }
    Some(DnsCacheKey {
        host,
        record_type: record_type.to_ascii_uppercase(),
    })
}

fn invalid_key() -> ChefError {
    ChefError::new(
        ErrorCode::InvalidInput,
        "La clave de caché DNS debe contener host y tipo de registro válidos",
    )
}
