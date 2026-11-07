//! Política pura para espaciar operaciones que consumen una cuota.
//!
//! La capa de aplicación aporta tiempo monotónico y espera el retraso devuelto.
//! Este módulo no duerme, no accede a red y no autoriza reconocimiento.

use std::num::NonZeroU32;
use std::time::Duration;

/// Reserva turnos a una tasa estable, sin ráfagas dentro de una ventana fija.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    interval: Duration,
    next_slot: Option<Duration>,
}

impl RateLimiter {
    /// `operations_per_second` debe ser positivo por construcción.
    pub fn new(operations_per_second: NonZeroU32) -> Self {
        let rate = u64::from(operations_per_second.get());
        let interval_nanos = 1_000_000_000_u64.div_ceil(rate).max(1);
        Self {
            interval: Duration::from_nanos(interval_nanos),
            next_slot: None,
        }
    }

    /// Reserva un turno a partir de un instante monotónico.
    ///
    /// Devuelve cuánto debe esperar el caller antes de ejecutar la operación.
    /// Cada reserva ocupa un turno, incluso si el caller espera para consumirlo;
    /// el caller debe serializar reservas del mismo limitador.
    pub fn reserve(&mut self, now: Duration) -> Duration {
        let slot = self.next_slot.map_or(now, |next| next.max(now));
        self.next_slot = Some(slot.saturating_add(self.interval));
        slot.saturating_sub(now)
    }
}
