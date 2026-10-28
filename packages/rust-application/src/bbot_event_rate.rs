//! Ventana móvil de procesamiento adaptada de `scanner.stats.SpeedCounter`.
//!
//! El llamador aporta tiempo monotónico; este contador no lee relojes ni mide
//! reconocimiento de red. Limita memoria y marca explícitamente conteos recortados.

use std::{collections::VecDeque, time::Duration};

const DEFAULT_WINDOW: Duration = Duration::from_secs(60);
const MAX_WINDOW: Duration = Duration::from_secs(60 * 60);
const MAX_SAMPLES: usize = 100_000;

/// Conteo de eventos procesados dentro de una ventana monotónica.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BbotEventRateSnapshot {
    events_in_window: usize,
    window_millis: u64,
    is_lower_bound: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbotEventRateError {
    InvalidWindow,
    WindowTooLong,
    ClockRegressed,
}

impl std::fmt::Display for BbotEventRateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidWindow => "La ventana de velocidad CHEF debe ser mayor que cero",
            Self::WindowTooLong => "La ventana de velocidad CHEF supera una hora",
            Self::ClockRegressed => "El reloj monotónico de velocidad CHEF retrocedió",
        })
    }
}

impl std::error::Error for BbotEventRateError {}

impl BbotEventRateSnapshot {
    pub fn events_in_window(&self) -> usize {
        self.events_in_window
    }

    pub fn window_millis(&self) -> u64 {
        self.window_millis
    }

    /// `true` indica que se descartaron muestras aún vigentes por el límite.
    pub fn is_lower_bound(&self) -> bool {
        self.is_lower_bound
    }
}

/// Contador acotado por ventana para procesamiento offline CHEF.
#[derive(Debug, Clone)]
pub struct BbotEventRateCounter {
    window: Duration,
    timestamps: VecDeque<Duration>,
    last_observed_at: Option<Duration>,
    latest_dropped_at: Option<Duration>,
}

impl Default for BbotEventRateCounter {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW).expect("la ventana CHEF por defecto es válida")
    }
}

impl BbotEventRateCounter {
    /// Crea un contador con una ventana mayor que cero y de hasta una hora.
    pub fn new(window: Duration) -> Result<Self, BbotEventRateError> {
        if window.is_zero() {
            return Err(BbotEventRateError::InvalidWindow);
        }
        if window > MAX_WINDOW {
            return Err(BbotEventRateError::WindowTooLong);
        }
        Ok(Self {
            window,
            timestamps: VecDeque::new(),
            last_observed_at: None,
            latest_dropped_at: None,
        })
    }

    /// Registra un evento ya procesado y devuelve el conteo de la ventana.
    ///
    /// `now` debe ser tiempo transcurrido desde un reloj monotónico de la tarea;
    /// nunca se interpreta un timestamp importado como tiempo de procesamiento.
    pub fn observe(&mut self, now: Duration) -> Result<BbotEventRateSnapshot, BbotEventRateError> {
        self.validate_time(now)?;
        self.timestamps.push_back(now);
        if self.timestamps.len() > MAX_SAMPLES
            && let Some(dropped) = self.timestamps.pop_front()
        {
            self.latest_dropped_at = Some(dropped);
        }
        self.prune(now);
        self.last_observed_at = Some(now);
        Ok(self.snapshot_at(now))
    }

    /// Consulta la ventana sin añadir una muestra.
    pub fn snapshot(&mut self, now: Duration) -> Result<BbotEventRateSnapshot, BbotEventRateError> {
        self.validate_time(now)?;
        self.prune(now);
        self.last_observed_at = Some(now);
        Ok(self.snapshot_at(now))
    }

    pub fn window(&self) -> Duration {
        self.window
    }

    fn validate_time(&self, now: Duration) -> Result<(), BbotEventRateError> {
        if self.last_observed_at.is_some_and(|last| now < last) {
            return Err(BbotEventRateError::ClockRegressed);
        }
        Ok(())
    }

    fn prune(&mut self, now: Duration) {
        while self
            .timestamps
            .front()
            .is_some_and(|event_at| now.saturating_sub(*event_at) > self.window)
        {
            self.timestamps.pop_front();
        }
        if self
            .latest_dropped_at
            .is_some_and(|dropped_at| now.saturating_sub(dropped_at) > self.window)
        {
            self.latest_dropped_at = None;
        }
    }

    fn snapshot_at(&self, now: Duration) -> BbotEventRateSnapshot {
        let window_millis = u64::try_from(self.window.as_millis()).unwrap_or(u64::MAX);
        BbotEventRateSnapshot {
            events_in_window: self.timestamps.len(),
            window_millis,
            is_lower_bound: self
                .latest_dropped_at
                .is_some_and(|dropped_at| now.saturating_sub(dropped_at) <= self.window),
        }
    }
}
