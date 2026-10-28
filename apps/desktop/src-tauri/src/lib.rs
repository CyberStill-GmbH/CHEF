use std::{collections::HashMap, path::PathBuf};

use chef_core::{
    Clock, ImportCancellation, ImportJobEvent, ScopePolicy, Snapshot, SystemClock,
    spawn_bbot_import_job_with_config,
};
use chef_storage::LocalSnapshotStore;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

mod config_store;

struct StoreSlot {
    database_path: PathBuf,
    store: Mutex<Option<LocalSnapshotStore>>,
}

struct ConfigSlot {
    config_path: PathBuf,
}

#[derive(Default)]
struct ImportJobs {
    active: Mutex<HashMap<String, ImportCancellation>>,
    admission: Mutex<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChefConfigDto {
    schema_version: &'static str,
    config: chef_core::ChefConfig,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChefTargetSeedDto {
    schema_version: &'static str,
    kind: &'static str,
    data: String,
    host: Option<String>,
    port: Option<u16>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanLogPayload {
    schema_version: &'static str,
    job_id: String,
    request_id: String,
    sequence: u64,
    phase: &'static str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    observed_at: Option<String>,
}

#[tauri::command]
async fn load_offline_demo_snapshot(
    app: AppHandle,
    state: State<'_, StoreSlot>,
    config_state: State<'_, ConfigSlot>,
    jobs: State<'_, ImportJobs>,
) -> Result<Snapshot, String> {
    const EVENT_FIXTURE: &[u8] = include_bytes!("../../../../fixtures/event-import/events.ndjson");
    const SCOPE_FIXTURE: &[u8] =
        include_bytes!("../../../../fixtures/event-import/demo-scope.json");

    let scope: ScopePolicy = serde_json::from_slice(SCOPE_FIXTURE)
        .map_err(|_| "La política del fixture CHEF no es válida".to_owned())?;
    import_bbot_payload(
        app,
        state,
        config_state,
        EVENT_FIXTURE.to_vec(),
        scope,
        None,
        &jobs,
        "00000000-0000-0000-0000-000000000000".to_owned(),
    )
    .await
}

#[tauri::command]
async fn import_bbot_jsonl(
    app: AppHandle,
    state: State<'_, StoreSlot>,
    config_state: State<'_, ConfigSlot>,
    jobs: State<'_, ImportJobs>,
    request_id: String,
    jsonl: String,
    scope: ScopePolicy,
    preset_id: Option<String>,
) -> Result<Snapshot, String> {
    validate_request_id(&request_id)?;
    let bytes = jsonl.into_bytes();
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("El archivo BBOT supera el máximo de 2 MiB CHEF".to_owned());
    }
    import_bbot_payload(
        app,
        state,
        config_state,
        bytes,
        scope,
        preset_id,
        &jobs,
        request_id,
    )
    .await
}

async fn import_bbot_payload(
    app: AppHandle,
    state: State<'_, StoreSlot>,
    config_state: State<'_, ConfigSlot>,
    bytes: Vec<u8>,
    scope: ScopePolicy,
    preset_id: Option<String>,
    jobs: &ImportJobs,
    request_id: String,
) -> Result<Snapshot, String> {
    let _admission = jobs
        .admission
        .try_lock()
        .map_err(|_| "Ya hay una importación CHEF en curso".to_owned())?;
    let mut config = load_runtime_config(config_state.config_path.clone()).await?;
    if let Some(preset_id) = preset_id {
        let preset = chef_core::PresetCatalog::built_in()
            .resolve(&preset_id)
            .map_err(|error| error.message.to_owned())?;
        preset
            .apply(&mut config)
            .map_err(|error| error.message.to_owned())?;
    }
    let task = spawn_bbot_import_job_with_config(bytes, scope, SystemClock.now(), config)
        .await
        .map_err(|error| error.message.to_owned())?;
    let mut task = task;
    let cancellation = task.cancellation();
    let mut sequence = 0_u64;
    let mut job_id: Option<String> = None;
    let mut log_delivery_failed = false;
    let mut cancelled = false;
    while let Some(event) = task.next_event().await {
        sequence += 1;
        if let ImportJobEvent::Started { job_id: id } = &event {
            job_id = Some(id.clone());
            jobs.active
                .lock()
                .await
                .insert(id.clone(), cancellation.clone());
        }
        match &event {
            ImportJobEvent::Completed { job_id: id, .. } => {
                cancelled = jobs
                    .active
                    .lock()
                    .await
                    .remove(id)
                    .is_some_and(|handle| handle.is_cancelled());
            }
            ImportJobEvent::Failed { job_id: id, code } => {
                jobs.active.lock().await.remove(id);
                cancelled = *code == chef_core::ErrorCode::Cancelled;
            }
            _ => {}
        }
        let payload = match event {
            ImportJobEvent::Started { job_id: id } => Some(ScanLogPayload {
                schema_version: "1.2.0",
                job_id: id,
                request_id: request_id.clone(),
                sequence,
                phase: "started",
                message: "Importación offline CHEF iniciada.",
                event_type: None,
                observed_at: None,
            }),
            ImportJobEvent::BbotEvent { job_id: id, event } => Some(ScanLogPayload {
                schema_version: "1.2.0",
                job_id: id,
                request_id: request_id.clone(),
                sequence,
                phase: "event",
                message: "Evento de evidencia fuente o derivada procesado.",
                event_type: Some(event.event_type),
                observed_at: Some(event.timestamp),
            }),
            ImportJobEvent::Completed { job_id: id, .. } => Some(ScanLogPayload {
                schema_version: "1.2.0",
                job_id: id,
                request_id: request_id.clone(),
                sequence,
                phase: if cancelled { "cancelled" } else { "validated" },
                message: if cancelled {
                    "La importación offline fue cancelada."
                } else {
                    "JSONL validado; guardando resultado local."
                },
                event_type: None,
                observed_at: None,
            }),
            ImportJobEvent::Failed { job_id: id, code } => Some(ScanLogPayload {
                schema_version: "1.2.0",
                job_id: id,
                request_id: request_id.clone(),
                sequence,
                phase: if code == chef_core::ErrorCode::Cancelled {
                    "cancelled"
                } else {
                    "failed"
                },
                message: if code == chef_core::ErrorCode::Cancelled {
                    "La importación offline fue cancelada."
                } else {
                    "La importación offline no pudo completarse."
                },
                event_type: None,
                observed_at: None,
            }),
        };
        if let Some(payload) = payload {
            log_delivery_failed |= app.emit("scan-log", payload).is_err();
        }
    }
    let import_result = task.join().await;
    if let Some(job_id) = &job_id {
        jobs.active.lock().await.remove(job_id);
    }
    if log_delivery_failed {
        return Err("No se pudo enviar el evento de actividad CHEF".to_owned());
    }
    if cancelled {
        return Err("La importación offline fue cancelada.".to_owned());
    }
    let imported = import_result.map_err(|error| error.message.to_owned())?;

    let mut slot = state.store.lock().await;
    if slot.is_none() {
        *slot = Some(
            LocalSnapshotStore::open(&state.database_path)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    slot.as_ref()
        .expect("el almacén se abrió arriba")
        .save_import_output(&imported)
        .await
        .map_err(|error| error.to_string())?;

    let job_id = job_id.ok_or_else(|| "La tarea CHEF no emitió su identificador".to_owned())?;
    sequence += 1;
    app.emit(
        "scan-log",
        ScanLogPayload {
            schema_version: "1.2.0",
            job_id: job_id.clone(),
            request_id: request_id.clone(),
            sequence,
            phase: "stored",
            message: "Snapshot y grafo guardados localmente.",
            event_type: None,
            observed_at: None,
        },
    )
    .map_err(|_| "No se pudo enviar el evento de actividad CHEF".to_owned())?;
    sequence += 1;
    app.emit(
        "scan-log",
        ScanLogPayload {
            schema_version: "1.2.0",
            job_id,
            request_id,
            sequence,
            phase: "completed",
            message: "Importación CHEF finalizada.",
            event_type: None,
            observed_at: None,
        },
    )
    .map_err(|_| "No se pudo enviar el evento de actividad CHEF".to_owned())?;

    Ok(imported.snapshot)
}

#[tauri::command]
async fn cancel_import_job(job_id: String, jobs: State<'_, ImportJobs>) -> Result<bool, String> {
    validate_job_id(&job_id)?;
    let active = jobs.active.lock().await;
    if let Some(cancellation) = active.get(&job_id) {
        cancellation.cancel();
        Ok(true)
    } else {
        Ok(false)
    }
}

#[tauri::command]
async fn load_chef_config(state: State<'_, ConfigSlot>) -> Result<ChefConfigDto, String> {
    let config = load_runtime_config(state.config_path.clone()).await?;
    Ok(ChefConfigDto {
        schema_version: "1.0.0",
        config,
    })
}

#[tauri::command]
fn parse_chef_target_seed(input: String) -> Result<ChefTargetSeedDto, String> {
    if input.len() > 8 * 1024 {
        return Err("El objetivo supera el máximo CHEF de 8 KiB".to_owned());
    }
    let seed =
        chef_core::parse_chef_target_seed(&input).map_err(|error| error.message.to_owned())?;
    let kind = match seed.kind {
        chef_core::ChefTargetKind::Asn => "asn",
        chef_core::ChefTargetKind::Organization => "organization",
        chef_core::ChefTargetKind::EmailAddress => "emailAddress",
        chef_core::ChefTargetKind::Username => "username",
        chef_core::ChefTargetKind::Filesystem => "filesystem",
        chef_core::ChefTargetKind::MobileApp => "mobileApp",
        chef_core::ChefTargetKind::BlacklistRegex => "blacklistRegex",
        chef_core::ChefTargetKind::IpAddress => "ipAddress",
        chef_core::ChefTargetKind::IpRange => "ipRange",
        chef_core::ChefTargetKind::DnsName => "dnsName",
        chef_core::ChefTargetKind::Url => "url",
        chef_core::ChefTargetKind::TcpEndpoint => "tcpEndpoint",
    };
    Ok(ChefTargetSeedDto {
        schema_version: "1.3.0",
        kind,
        data: seed.data,
        host: seed.host,
        port: seed.port,
    })
}

async fn load_runtime_config(config_path: PathBuf) -> Result<chef_core::ChefConfig, String> {
    tokio::task::spawn_blocking(move || config_store::load_config(&config_path))
        .await
        .map_err(|_| "No se pudo completar la carga de configuración CHEF".to_owned())?
}

#[tauri::command]
async fn load_chef_recon_graph(
    run_id: String,
    state: State<'_, StoreSlot>,
) -> Result<chef_core::ChefReconGraph, String> {
    validate_run_id(&run_id)?;

    let mut slot = state.store.lock().await;
    if slot.is_none() {
        *slot = Some(
            LocalSnapshotStore::open(&state.database_path)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    slot.as_ref()
        .expect("el almacén se abrió arriba")
        .load_chef_recon_graph(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No existe un grafo CHEF para esa importación".to_owned())
}

#[tauri::command]
async fn load_chef_event_statistics(
    run_id: String,
    state: State<'_, StoreSlot>,
) -> Result<EventStatisticsDto, String> {
    validate_run_id(&run_id)?;

    let mut slot = state.store.lock().await;
    if slot.is_none() {
        *slot = Some(
            LocalSnapshotStore::open(&state.database_path)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    let graph = slot
        .as_ref()
        .expect("el almacén se abrió arriba")
        .load_bbot_event_graph(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No existen eventos de origen para esa importación".to_owned())?;
    let statistics =
        chef_core::summarize_bbot_events(&graph).map_err(|error| error.message.to_owned())?;
    Ok(EventStatisticsDto {
        schema_version: "1.0.0",
        statistics,
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EventStatisticsDto {
    schema_version: &'static str,
    statistics: chef_core::BbotEventStatistics,
}

#[tauri::command]
async fn export_chef_snapshot(
    run_id: String,
    state: State<'_, StoreSlot>,
) -> Result<Snapshot, String> {
    validate_run_id(&run_id)?;
    let mut slot = state.store.lock().await;
    if slot.is_none() {
        *slot = Some(
            LocalSnapshotStore::open(&state.database_path)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    slot.as_ref()
        .expect("el almacén se abrió arriba")
        .load(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No existe un snapshot CHEF para esa ejecución".to_owned())
}

#[tauri::command]
async fn export_chef_observed_domains(
    run_id: String,
    state: State<'_, StoreSlot>,
) -> Result<chef_core::ChefObservedDomainExport, String> {
    validate_run_id(&run_id)?;
    let mut slot = state.store.lock().await;
    if slot.is_none() {
        *slot = Some(
            LocalSnapshotStore::open(&state.database_path)
                .await
                .map_err(|error| error.to_string())?,
        );
    }
    let graph = slot
        .as_ref()
        .expect("el almacén se abrió arriba")
        .load_chef_recon_graph(&run_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "No existe un grafo CHEF para esa importación".to_owned())?;
    chef_core::export_observed_domain_list(&run_id, &graph)
        .map_err(|error| error.message.to_owned())
}

fn validate_run_id(run_id: &str) -> Result<(), String> {
    if !run_id.starts_with("run:")
        || run_id.len() == "run:".len()
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err("El identificador de ejecución CHEF no es válido".to_owned());
    }
    Ok(())
}

fn validate_job_id(job_id: &str) -> Result<(), String> {
    if !job_id.starts_with("job:")
        || job_id.len() > 128
        || !job_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err("El identificador de trabajo CHEF no es válido".to_owned());
    }
    Ok(())
}

fn validate_request_id(request_id: &str) -> Result<(), String> {
    let bytes = request_id.as_bytes();
    if bytes.len() != 36
        || bytes.iter().enumerate().any(|(index, byte)| match index {
            8 | 13 | 18 | 23 => *byte != b'-',
            _ => !byte.is_ascii_hexdigit(),
        })
    {
        return Err("El identificador de solicitud CHEF no es válido".to_owned());
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_offline_demo_snapshot,
            import_bbot_jsonl,
            cancel_import_job,
            load_chef_recon_graph,
            load_chef_event_statistics,
            export_chef_snapshot,
            export_chef_observed_domains,
            load_chef_config,
            parse_chef_target_seed
        ])
        .setup(|app| {
            let database_path = app.path().app_local_data_dir()?.join("chef/rocksdb");
            let config_path = app.path().app_config_dir()?.join("config.toml");
            app.manage(StoreSlot {
                database_path,
                store: Mutex::new(None),
            });
            app.manage(ConfigSlot { config_path });
            app.manage(ImportJobs::default());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la ventana de CHEF");
}
