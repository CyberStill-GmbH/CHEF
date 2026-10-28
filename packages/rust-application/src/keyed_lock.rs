//! Exclusión asíncrona por clave para adaptadores y trabajos CHEF.
//!
//! El registro tiene capacidad acotada, nunca mantiene su mutex de coordinación
//! durante un `.await` y elimina entradas inactivas cuando deja de usarse una
//! clave. No realiza operaciones de E/S.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, Weak},
};

use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};

use chef_core::{ChefError, ChefResult, ErrorCode};

const DEFAULT_MAX_ENTRIES: usize = 10_000;
const MAX_ALLOWED_ENTRIES: usize = 100_000;
const MAX_KEY_BYTES: usize = 4_096;

#[derive(Debug)]
struct Inner {
    entries: Mutex<BTreeMap<String, Arc<AsyncMutex<()>>>>,
    max_entries: usize,
}

/// Proporciona exclusión mutua asíncrona independiente para cada clave.
///
/// Las claves son opacas y distinguen mayúsculas. La capacidad se aplica al
/// número de claves residentes; si está completa y todas están ocupadas, una
/// nueva clave falla con `LimitExceeded` en lugar de evictar un lock en uso.
#[derive(Debug, Clone)]
pub struct KeyedLock {
    inner: Arc<Inner>,
}

impl Default for KeyedLock {
    fn default() -> Self {
        Self {
            inner: Arc::new(Inner {
                entries: Mutex::new(BTreeMap::new()),
                max_entries: DEFAULT_MAX_ENTRIES,
            }),
        }
    }
}

impl KeyedLock {
    /// Crea un registro de locks con capacidad explícita y acotada.
    pub fn with_max_entries(max_entries: usize) -> ChefResult<Self> {
        if max_entries == 0 || max_entries > MAX_ALLOWED_ENTRIES {
            return Err(ChefError::new(
                if max_entries == 0 {
                    ErrorCode::InvalidInput
                } else {
                    ErrorCode::LimitExceeded
                },
                "La capacidad de locks CHEF debe estar entre 1 y 100000",
            ));
        }
        Ok(Self {
            inner: Arc::new(Inner {
                entries: Mutex::new(BTreeMap::new()),
                max_entries,
            }),
        })
    }

    /// Adquiere el lock exclusivo asociado a una clave.
    ///
    /// El guard libera la entrada inactiva al salir si no hay otros usuarios o
    /// esperas para esa clave. Si la tabla está llena, se retira primero una
    /// entrada inactiva de menor orden lexicográfico; no se rompe exclusión.
    pub async fn lock(&self, key: impl Into<String>) -> ChefResult<KeyedLockGuard> {
        let key = key.into();
        validate_key(&key)?;

        let lock = {
            let mut entries = self.inner.entries.lock().map_err(|_| poisoned_registry())?;

            if let Some(existing) = entries.get(&key) {
                Arc::clone(existing)
            } else {
                if entries.len() >= self.inner.max_entries {
                    let idle_key = entries.iter().find_map(|(candidate, entry)| {
                        if Arc::strong_count(entry) != 1 {
                            return None;
                        }
                        entry.try_lock().ok().map(|guard| {
                            drop(guard);
                            candidate.clone()
                        })
                    });
                    if let Some(idle_key) = idle_key {
                        entries.remove(&idle_key);
                    } else {
                        return Err(ChefError::new(
                            ErrorCode::LimitExceeded,
                            "El registro de locks CHEF alcanzó su capacidad",
                        ));
                    }
                }

                let lock = Arc::new(AsyncMutex::new(()));
                entries.insert(key.clone(), Arc::clone(&lock));
                lock
            }
        };

        let guard = Arc::clone(&lock).lock_owned().await;
        Ok(KeyedLockGuard {
            key,
            inner: Arc::downgrade(&self.inner),
            lock,
            guard: Some(guard),
        })
    }

    /// Número de claves actualmente residentes, útil para telemetría local.
    pub fn len(&self) -> ChefResult<usize> {
        self.inner
            .entries
            .lock()
            .map(|entries| entries.len())
            .map_err(|_| poisoned_registry())
    }

    pub fn is_empty(&self) -> ChefResult<bool> {
        self.len().map(|len| len == 0)
    }
}

/// Guard propietario que libera el lock y limpia su clave cuando queda ociosa.
#[derive(Debug)]
pub struct KeyedLockGuard {
    key: String,
    inner: Weak<Inner>,
    lock: Arc<AsyncMutex<()>>,
    guard: Option<OwnedMutexGuard<()>>,
}

impl Drop for KeyedLockGuard {
    fn drop(&mut self) {
        drop(self.guard.take());
        let Some(inner) = self.inner.upgrade() else {
            return;
        };
        let Ok(mut entries) = inner.entries.lock() else {
            return;
        };

        let is_current = entries
            .get(&self.key)
            .is_some_and(|entry| Arc::ptr_eq(entry, &self.lock));
        if is_current && Arc::strong_count(&self.lock) == 2 {
            entries.remove(&self.key);
        }
    }
}

fn validate_key(key: &str) -> ChefResult<()> {
    if key.is_empty() || key.len() > MAX_KEY_BYTES || key.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La clave de lock CHEF está vacía, es demasiado larga o contiene controles",
        ));
    }
    Ok(())
}

fn poisoned_registry() -> ChefError {
    ChefError::new(
        ErrorCode::WorkerFailed,
        "El registro local de locks CHEF no está disponible",
    )
}
