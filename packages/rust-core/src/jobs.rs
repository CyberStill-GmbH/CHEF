//! Coordinación asíncrona de importaciones locales para la futura capa Tauri.
//! No inicia módulos ni tráfico de reconocimiento.

use std::collections::BTreeMap;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
};

use tokio::{
    sync::{Notify, mpsc},
    task::{JoinError, JoinHandle},
};

use crate::{
    BbotEventGraph, BbotImportResult, ChefConfig, ChefError, ChefReconGraph, ChefResult, ErrorCode,
    FixedClock, ImportBbotEvidence, ImportEvidence, ScopePolicy, Snapshot, identity,
};

const DEFAULT_EVENT_CAPACITY: usize = 16;
static NEXT_JOB_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Cuenta unidades de trabajo activas sin conservar nombres, payloads ni handles
/// de tareas. Cada permiso devuelve su peso al contador al salir, incluso si el
/// worker termina por cancelación o pánico.
#[derive(Debug, Clone, Default)]
struct TaskCounter {
    active: Arc<AtomicUsize>,
}

#[derive(Debug)]
struct TaskPermit {
    active: Arc<AtomicUsize>,
    weight: usize,
}

impl TaskCounter {
    fn enter(&self, weight: usize) -> ChefResult<TaskPermit> {
        self.active
            .try_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                active.checked_add(weight)
            })
            .map_err(|_| {
                ChefError::new(
                    ErrorCode::LimitExceeded,
                    "El contador de trabajos CHEF superó su capacidad",
                )
            })?;
        Ok(TaskPermit {
            active: Arc::clone(&self.active),
            weight,
        })
    }

    fn value(&self) -> usize {
        self.active.load(Ordering::Acquire)
    }
}

impl Drop for TaskPermit {
    fn drop(&mut self) {
        if self.weight > 0 {
            self.active.fetch_sub(self.weight, Ordering::AcqRel);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    NmapXml,
    BbotJsonl,
}

impl ImportFormat {
    fn as_str(self) -> &'static str {
        match self {
            Self::NmapXml => "nmap-xml",
            Self::BbotJsonl => "bbot-ndjson",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportJobEvent {
    Started {
        job_id: String,
    },
    /// Evento BBOT fuente saneado; solo se emite al usar el flujo con eventos.
    BbotEvent {
        job_id: String,
        event: Box<crate::BbotEvent>,
    },
    Completed {
        job_id: String,
        observation_count: usize,
    },
    Failed {
        job_id: String,
        code: ErrorCode,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportOutput {
    pub snapshot: Snapshot,
    pub skipped_event_types: BTreeMap<String, usize>,
    pub event_graph: Option<BbotEventGraph>,
    pub recon_graph: Option<ChefReconGraph>,
}

impl From<Snapshot> for ImportOutput {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            snapshot,
            skipped_event_types: BTreeMap::new(),
            event_graph: None,
            recon_graph: None,
        }
    }
}

impl From<BbotImportResult> for ImportOutput {
    fn from(result: BbotImportResult) -> Self {
        Self {
            snapshot: result.snapshot,
            skipped_event_types: result.skipped_event_types,
            event_graph: Some(result.event_graph),
            recon_graph: Some(result.recon_graph),
        }
    }
}

/// Handle for a bounded background import and its status stream.
pub struct ImportTask {
    cancel: Arc<AtomicBool>,
    cancel_notify: Arc<Notify>,
    task_counter: TaskCounter,
    events: mpsc::Receiver<ImportJobEvent>,
    worker: JoinHandle<ChefResult<ImportOutput>>,
}

impl ImportTask {
    /// Request cooperative cancellation. The importer checks between records and
    /// while publishing events, including when a bounded channel applies backpressure.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
        self.cancel_notify.notify_one();
    }

    pub async fn next_event(&mut self) -> Option<ImportJobEvent> {
        self.events.recv().await
    }

    /// Unidades de trabajo activas asociadas a esta importación.
    /// El valor vuelve a cero al terminar o cancelarse el worker.
    pub fn active_task_count(&self) -> usize {
        self.task_counter.value()
    }

    pub async fn join(self) -> ChefResult<ImportOutput> {
        self.worker.await.map_err(|_: JoinError| {
            ChefError::new(
                ErrorCode::WorkerFailed,
                "Import worker terminated unexpectedly",
            )
        })?
    }
}

/// Parse Nmap XML or BBOT JSONL in a blocking worker, outside Tokio's async
/// executor threads. Lifecycle events use a bounded channel.
pub async fn spawn_import_job(
    format: ImportFormat,
    bytes: Vec<u8>,
    policy: ScopePolicy,
    imported_at: String,
) -> ChefResult<ImportTask> {
    spawn_import_job_inner(
        format,
        bytes,
        policy,
        imported_at,
        ChefConfig::default(),
        false,
    )
    .await
}

/// Import BBOT JSONL and stream each sanitized source or derived event through the same
/// bounded channel before `Completed`. The caller must drain events while the
/// task runs so backpressure stays bounded.
pub async fn spawn_bbot_import_job(
    bytes: Vec<u8>,
    policy: ScopePolicy,
    imported_at: String,
) -> ChefResult<ImportTask> {
    spawn_bbot_import_job_with_config(bytes, policy, imported_at, ChefConfig::default()).await
}

/// Variante de importación que habilita los módulos offline solicitados en la
/// configuración local validada.
pub async fn spawn_bbot_import_job_with_config(
    bytes: Vec<u8>,
    policy: ScopePolicy,
    imported_at: String,
    config: ChefConfig,
) -> ChefResult<ImportTask> {
    spawn_import_job_inner(
        ImportFormat::BbotJsonl,
        bytes,
        policy,
        imported_at,
        config,
        true,
    )
    .await
}

async fn spawn_import_job_inner(
    format: ImportFormat,
    bytes: Vec<u8>,
    policy: ScopePolicy,
    imported_at: String,
    config: ChefConfig,
    stream_bbot_events: bool,
) -> ChefResult<ImportTask> {
    let sequence = NEXT_JOB_SEQUENCE
        .fetch_add(1, Ordering::Relaxed)
        .to_string();
    let job_id = identity::id(
        "job",
        &[format.as_str(), &identity::digest(&bytes), &sequence],
    );
    let cancel = Arc::new(AtomicBool::new(false));
    let cancel_notify = Arc::new(Notify::new());
    let task_counter = TaskCounter::default();
    let task_permit = task_counter.enter(1)?;
    let worker_cancel = Arc::clone(&cancel);
    let stream_cancel = Arc::clone(&cancel);
    let stream_cancel_notify = Arc::clone(&cancel_notify);
    let (tx, events) = mpsc::channel(DEFAULT_EVENT_CAPACITY);
    tx.send(ImportJobEvent::Started {
        job_id: job_id.clone(),
    })
    .await
    .map_err(|_| ChefError::new(ErrorCode::WorkerFailed, "Import event receiver unavailable"))?;

    let worker = tokio::spawn(async move {
        let _task_permit = task_permit;
        let blocking_cancel = Arc::clone(&worker_cancel);
        let result = tokio::task::spawn_blocking(move || {
            let clock = FixedClock(imported_at);
            match format {
                ImportFormat::NmapXml => {
                    ImportEvidence::execute(&bytes, &policy, &clock, Some(blocking_cancel.as_ref()))
                        .map(ImportOutput::from)
                }
                ImportFormat::BbotJsonl => ImportBbotEvidence::execute_with_config(
                    &bytes,
                    &policy,
                    &config,
                    &clock,
                    Some(blocking_cancel.as_ref()),
                )
                .map(ImportOutput::from),
            }
        })
        .await
        .map_err(|_: JoinError| {
            ChefError::new(
                ErrorCode::WorkerFailed,
                "Import worker terminated unexpectedly",
            )
        })?;
        let result = match result {
            Ok(output) if stream_bbot_events => {
                let stream_result = async {
                    if let Some(graph) = output.event_graph.as_ref() {
                        for event in &graph.events {
                            if stream_cancel.load(Ordering::SeqCst) {
                                return Err(ChefError::new(
                                    ErrorCode::Cancelled,
                                    "Importación CHEF cancelada durante el flujo de eventos",
                                ));
                            }
                            tokio::select! {
                                _ = stream_cancel_notify.notified() => {
                                    return Err(ChefError::new(
                                        ErrorCode::Cancelled,
                                        "Importación CHEF cancelada durante el flujo de eventos",
                                    ));
                                }
                                result = tx.send(ImportJobEvent::BbotEvent {
                                    job_id: job_id.clone(),
                                    event: Box::new(event.clone()),
                                }) => {
                                    result.map_err(|_| ChefError::new(
                                        ErrorCode::WorkerFailed,
                                        "Import event receiver unavailable",
                                    ))?;
                                }
                            }
                        }
                    }
                    Ok::<(), ChefError>(())
                }
                .await;
                match stream_result {
                    Ok(()) => Ok(output),
                    Err(error) => Err(error),
                }
            }
            other => other,
        };
        match &result {
            Ok(output) => {
                let _ = tx
                    .send(ImportJobEvent::Completed {
                        job_id: job_id.clone(),
                        observation_count: output.snapshot.observations.len(),
                    })
                    .await;
            }
            Err(error) => {
                let _ = tx
                    .send(ImportJobEvent::Failed {
                        job_id: job_id.clone(),
                        code: error.code,
                    })
                    .await;
            }
        }
        result
    });

    Ok(ImportTask {
        cancel,
        cancel_notify,
        task_counter,
        events,
        worker,
    })
}
