use std::path::PathBuf;

use chef_core::{
    Clock, ImportJobEvent, ScopePolicy, Snapshot, SystemClock, spawn_bbot_import_job_with_config,
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
) -> Result<Snapshot, String> {
    const EVENT_FIXTURE: &[u8] = include_bytes!("../../../../fixtures/event-import/events.ndjson");
    const SCOPE_FIXTURE: &[u8] = include_bytes!("../../../../fixtures/event-import/demo-scope.json");

    let scope: ScopePolicy = serde_json::from_slice(SCOPE_FIXTURE)
        .map_err(|_| "La política del fixture CHEF no es válida".to_owned())?;
    import_bbot_payload(
        app,
        state,
        config_state,
        EVENT_FIXTURE.to_vec(),
        scope,
        None,
    )
    .await
}

#[tauri::command]
async fn import_bbot_jsonl(
    app: AppHandle,
    state: State<'_, StoreSlot>,
    config_state: State<'_, ConfigSlot>,
    jsonl: String,
    scope: ScopePolicy,
    preset_id: Option<String>,
) -> Result<Snapshot, String> {
    let bytes = jsonl.into_bytes();
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("El archivo BBOT supera el máximo de 2 MiB CHEF".to_owned());
    }
    import_bbot_payload(app, state, config_state, bytes, scope, preset_id).await
}

async fn import_bbot_payload(
    app: AppHandle,
    state: State<'_, StoreSlot>,
    config_state: State<'_, ConfigSlot>,
    bytes: Vec<u8>,
    scope: ScopePolicy,
    preset_id: Option<String>,
) -> Result<Snapshot, String> {
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
    let mut sequence = 0_u64;
    let mut job_id: Option<String> = None;
    while let Some(event) = task.next_event().await {
        sequence += 1;
        let payload = match event {
            ImportJobEvent::Started { job_id: id } => {
                job_id = Some(id.clone());
                Some(ScanLogPayload {
                    schema_version: "1.0.0",
                    job_id: id,
                    sequence,
                    phase: "started",
                    message: "Importación offline CHEF iniciada.",
                    event_type: None,
                    observed_at: None,
                })
            }
            ImportJobEvent::BbotEvent { job_id: id, event } => Some(ScanLogPayload {
                schema_version: "1.0.0",
                job_id: id,
                sequence,
                phase: "event",
                message: "Evento de evidencia fuente o derivada procesado.",
                event_type: Some(event.event_type),
                observed_at: Some(event.timestamp),
            }),
            ImportJobEvent::Completed { job_id: id, .. } => Some(ScanLogPayload {
                schema_version: "1.0.0",
                job_id: id,
                sequence,
                phase: "validated",
                message: "JSONL validado; guardando resultado local.",
                event_type: None,
                observed_at: None,
            }),
            ImportJobEvent::Failed { job_id: id, .. } => Some(ScanLogPayload {
                schema_version: "1.0.0",
                job_id: id,
                sequence,
                phase: "failed",
                message: "La importación offline no pudo completarse.",
                event_type: None,
                observed_at: None,
            }),
        };
        if let Some(payload) = payload {
            app.emit("scan-log", payload)
                .map_err(|_| "No se pudo enviar el evento de actividad CHEF".to_owned())?;
        }
    }
    let imported = task
        .join()
        .await
        .map_err(|error| error.message.to_owned())?;

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
            schema_version: "1.0.0",
            job_id: job_id.clone(),
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
            schema_version: "1.0.0",
            job_id,
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
        chef_core::ChefTargetKind::IpAddress => "ipAddress",
        chef_core::ChefTargetKind::IpRange => "ipRange",
        chef_core::ChefTargetKind::DnsName => "dnsName",
        chef_core::ChefTargetKind::Url => "url",
        chef_core::ChefTargetKind::TcpEndpoint => "tcpEndpoint",
    };
    Ok(ChefTargetSeedDto {
        schema_version: "1.1.0",
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

fn validate_run_id(run_id: &str) -> Result<(), String> {
    if !run_id.starts_with("run:")
        || run_id.len() > 128
        || !run_id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b":-".contains(&byte))
    {
        return Err("El identificador de ejecución CHEF no es válido".to_owned());
    }
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_offline_demo_snapshot,
            import_bbot_jsonl,
            load_chef_recon_graph,
            load_chef_event_statistics,
            export_chef_snapshot,
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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("no se pudo iniciar la ventana de CHEF");
}
