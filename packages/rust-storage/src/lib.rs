//! Adaptador de persistencia local CHEF con SurrealDB embebida y RocksDB.
//! El core no conoce archivos ni almacenamiento.

use std::path::Path;

use chef_application::{
    IMPORT_HISTORY_MAX_PAGE_SIZE, ImportHistoryError, ImportHistoryPort, ImportOutput,
    ImportRunSummary,
};
use chef_core::{
    BbotEventGraph, BbotImportResult, CensysIpImportResult, ChefReconGraph, Snapshot,
    deterministic_id,
};
use serde_json::{Value, json};
use surrealdb::{
    Surreal,
    engine::local::{Db, RocksDb},
};
use thiserror::Error;

const SUPPORTED_SNAPSHOT_VERSIONS: &[&str] = &["1.0.0", "1.1.0"];
const NAMESPACE: &str = "chef";
const DATABASE: &str = "local";
const TABLE: &str = "snapshots";
const PROVIDER_IMPORT_TABLE: &str = "provider_imports";
const PROVIDER_IMPORT_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("no se pudo abrir o actualizar SurrealDB local: {0}")]
    Database(#[from] surrealdb::Error),
    #[error("no se pudo crear el directorio local de datos: {0}")]
    FileSystem(#[from] std::io::Error),
    #[error("no se pudo serializar la importación CHEF: {0}")]
    Serialize(#[from] serde_json::Error),
    #[error("la versión de esquema o snapshot local no está soportada")]
    UnsupportedVersion,
    #[error("el snapshot y su clave de persistencia no coinciden")]
    InvalidRecord,
}

/// Almacén local por perfil. Mantener una sola instancia abierta por directorio.
pub struct LocalSnapshotStore {
    db: Surreal<Db>,
}

impl LocalSnapshotStore {
    /// Abre o crea el directorio de RocksDB. La ruta debe venir de la capa Tauri.
    pub async fn open(path: impl AsRef<Path>) -> Result<Self, StorageError> {
        if let Some(parent) = path.as_ref().parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent)?;
        }
        let path = path.as_ref().to_string_lossy().into_owned();
        let db = Surreal::new::<RocksDb>(path).await?;
        db.use_ns(NAMESPACE).use_db(DATABASE).await?;
        db.query(
            "DEFINE TABLE IF NOT EXISTS snapshots SCHEMAFULL;
             DEFINE FIELD IF NOT EXISTS run_id ON snapshots TYPE string;
             DEFINE FIELD IF NOT EXISTS schema_version ON snapshots TYPE string;
             DEFINE FIELD OVERWRITE snapshot ON snapshots TYPE object FLEXIBLE;
             DEFINE FIELD IF NOT EXISTS event_graph ON snapshots TYPE any;
             DEFINE FIELD IF NOT EXISTS chef_recon_graph ON snapshots TYPE any;
             DEFINE TABLE IF NOT EXISTS provider_imports SCHEMAFULL;
             DEFINE FIELD IF NOT EXISTS import_id ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS schema_version ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS provider ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS target_ip ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS response_sha256 ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS imported_at ON provider_imports TYPE string;
             DEFINE FIELD IF NOT EXISTS event_graph ON provider_imports TYPE any;
             DEFINE FIELD IF NOT EXISTS chef_recon_graph ON provider_imports TYPE any;",
        )
        .await?
        .check()?;
        Ok(Self { db })
    }

    /// Reemplaza el snapshot completo bajo su runId determinista en un único upsert.
    pub async fn save(&self, snapshot: &Snapshot) -> Result<(), StorageError> {
        self.save_with_event_graph(snapshot, None).await
    }

    /// Guarda snapshot y grafo BBOT bajo el mismo documento y upsert atómico.
    pub async fn save_bbot_import(&self, result: &BbotImportResult) -> Result<(), StorageError> {
        self.save_with_graphs(
            &result.snapshot,
            Some(&result.event_graph),
            Some(&result.recon_graph),
        )
        .await
    }

    /// Persiste el resultado de un trabajo Tokio y, si es BBOT, su grafo completo.
    pub async fn save_import_output(&self, result: &ImportOutput) -> Result<(), StorageError> {
        self.save_with_graphs(
            &result.snapshot,
            result.event_graph.as_ref(),
            result.recon_graph.as_ref(),
        )
        .await
    }

    async fn save_with_event_graph(
        &self,
        snapshot: &Snapshot,
        event_graph: Option<&BbotEventGraph>,
    ) -> Result<(), StorageError> {
        self.save_with_graphs(snapshot, event_graph, None).await
    }

    async fn save_with_graphs(
        &self,
        snapshot: &Snapshot,
        event_graph: Option<&BbotEventGraph>,
        recon_graph: Option<&ChefReconGraph>,
    ) -> Result<(), StorageError> {
        validate_snapshot_key(snapshot, &snapshot.run.id)?;
        if let Some(graph) = event_graph {
            if snapshot.schema_version != "1.1.0" {
                return Err(StorageError::InvalidRecord);
            }
            chef_core::validate_bbot_event_graph(graph).map_err(|_| StorageError::InvalidRecord)?;
        }
        if let Some(graph) = recon_graph {
            if snapshot.schema_version != "1.1.0" || event_graph.is_none() {
                return Err(StorageError::InvalidRecord);
            }
            chef_core::validate_chef_recon_graph(graph).map_err(|_| StorageError::InvalidRecord)?;
        }
        let derived_graph = if recon_graph.is_none() {
            event_graph
                .map(chef_core::adapt_bbot_to_chef)
                .transpose()
                .map_err(|_| StorageError::InvalidRecord)?
        } else {
            None
        };
        let recon_graph = recon_graph.or(derived_graph.as_ref());
        let payload = json!({
            "run_id": snapshot.run.id,
            "schema_version": snapshot.schema_version,
            "snapshot": snapshot,
            "event_graph": event_graph,
            "chef_recon_graph": recon_graph,
        });
        let _: Option<Value> = self
            .db
            .upsert((TABLE, snapshot.run.id.as_str()))
            .content(payload)
            .await?;
        Ok(())
    }

    /// Guarda una importación Censys archivada sin forzarla al contrato Snapshot,
    /// que representa observaciones de puertos abiertos y no pistas históricas.
    pub async fn save_censys_ip_import(
        &self,
        result: &CensysIpImportResult,
    ) -> Result<String, StorageError> {
        validate_censys_ip_import(result)?;
        let import_id = censys_import_id(result);
        let imported_at = result
            .event_graph
            .events
            .first()
            .map(|event| event.timestamp.as_str())
            .ok_or(StorageError::InvalidRecord)?;
        let payload = json!({
            "import_id": import_id,
            "schema_version": PROVIDER_IMPORT_SCHEMA_VERSION,
            "provider": "censys-hosts-ip",
            "target_ip": result.target_ip,
            "response_sha256": result.response_sha256,
            "imported_at": imported_at,
            "event_graph": result.event_graph,
            "chef_recon_graph": result.recon_graph,
        });
        let _: Option<Value> = self
            .db
            .upsert((PROVIDER_IMPORT_TABLE, import_id.as_str()))
            .content(payload)
            .await?;
        Ok(import_id)
    }

    /// Recupera la importación y vuelve a validar su identidad y grafos.
    pub async fn load_censys_ip_import(
        &self,
        import_id: &str,
    ) -> Result<Option<CensysIpImportResult>, StorageError> {
        if !valid_provider_import_id(import_id) {
            return Err(StorageError::InvalidRecord);
        }
        let Some(record) = self
            .db
            .select::<Option<Value>>((PROVIDER_IMPORT_TABLE, import_id))
            .await?
        else {
            return Ok(None);
        };
        if record.get("import_id").and_then(Value::as_str) != Some(import_id)
            || record.get("schema_version").and_then(Value::as_str)
                != Some(PROVIDER_IMPORT_SCHEMA_VERSION)
            || record.get("provider").and_then(Value::as_str) != Some("censys-hosts-ip")
        {
            return Err(StorageError::InvalidRecord);
        }
        let result: CensysIpImportResult = serde_json::from_value(json!({
            "target_ip": record.get("target_ip").cloned().ok_or(StorageError::InvalidRecord)?,
            "response_sha256": record.get("response_sha256").cloned().ok_or(StorageError::InvalidRecord)?,
            "event_graph": record.get("event_graph").cloned().ok_or(StorageError::InvalidRecord)?,
            "recon_graph": record.get("chef_recon_graph").cloned().ok_or(StorageError::InvalidRecord)?,
        }))?;
        validate_censys_ip_import(&result)?;
        if censys_import_id(&result) != import_id
            || record.get("imported_at").and_then(Value::as_str)
                != result
                    .event_graph
                    .events
                    .first()
                    .map(|event| event.timestamp.as_str())
        {
            return Err(StorageError::InvalidRecord);
        }
        Ok(Some(result))
    }

    /// Recupera el grafo tipado conservado con una importación BBOT.
    pub async fn load_bbot_event_graph(
        &self,
        run_id: &str,
    ) -> Result<Option<BbotEventGraph>, StorageError> {
        let Some(record) = self.db.select::<Option<Value>>((TABLE, run_id)).await? else {
            return Ok(None);
        };
        if record.get("run_id").and_then(Value::as_str) != Some(run_id)
            || record.get("schema_version").and_then(Value::as_str) != Some("1.1.0")
        {
            return Err(StorageError::InvalidRecord);
        }
        let Some(value) = record.get("event_graph") else {
            return Ok(None);
        };
        if value.is_null() {
            return Ok(None);
        }
        let graph: BbotEventGraph = serde_json::from_value(value.clone())?;
        chef_core::validate_bbot_event_graph(&graph).map_err(|_| StorageError::InvalidRecord)?;
        Ok(Some(graph))
    }

    /// Recupera activos, evidencias y relaciones adaptados al reconocimiento CHEF.
    pub async fn load_chef_recon_graph(
        &self,
        run_id: &str,
    ) -> Result<Option<ChefReconGraph>, StorageError> {
        let Some(record) = self.db.select::<Option<Value>>((TABLE, run_id)).await? else {
            return Ok(None);
        };
        if record.get("run_id").and_then(Value::as_str) != Some(run_id)
            || record.get("schema_version").and_then(Value::as_str) != Some("1.1.0")
        {
            return Err(StorageError::InvalidRecord);
        }
        let Some(value) = record.get("chef_recon_graph") else {
            return Ok(None);
        };
        if value.is_null() {
            return Ok(None);
        }
        let graph: ChefReconGraph = serde_json::from_value(value.clone())?;
        chef_core::validate_chef_recon_graph(&graph).map_err(|_| StorageError::InvalidRecord)?;
        Ok(Some(graph))
    }

    /// Recupera el documento completo y vuelve a comprobar versión e identidad.
    pub async fn load(&self, run_id: &str) -> Result<Option<Snapshot>, StorageError> {
        let Some(record) = self.db.select::<Option<Value>>((TABLE, run_id)).await? else {
            return Ok(None);
        };
        let stored_run_id = record
            .get("run_id")
            .and_then(Value::as_str)
            .ok_or(StorageError::InvalidRecord)?;
        let schema_version = record
            .get("schema_version")
            .and_then(Value::as_str)
            .ok_or(StorageError::InvalidRecord)?;
        if stored_run_id != run_id {
            return Err(StorageError::InvalidRecord);
        }
        if !SUPPORTED_SNAPSHOT_VERSIONS.contains(&schema_version) {
            return Err(StorageError::UnsupportedVersion);
        }
        let payload = record
            .get("snapshot")
            .cloned()
            .ok_or(StorageError::InvalidRecord)?;
        let snapshot: Snapshot = serde_json::from_value(payload)?;
        validate_snapshot_key(&snapshot, run_id)?;
        if snapshot.schema_version != schema_version {
            return Err(StorageError::InvalidRecord);
        }
        Ok(Some(snapshot))
    }
}

impl ImportHistoryPort for LocalSnapshotStore {
    async fn list_recent(&self, limit: usize) -> Result<Vec<ImportRunSummary>, ImportHistoryError> {
        if limit == 0 || limit > IMPORT_HISTORY_MAX_PAGE_SIZE {
            return Err(ImportHistoryError::InvalidLimit);
        }

        let mut response = self
            .db
            .query(
                r#"SELECT run_id, schema_version,
                          snapshot.run.id AS snapshot_run_id,
                          snapshot.run.state AS state,
                          snapshot.run.startedAt AS started_at,
                          snapshot.run.finishedAt AS finished_at,
                          array::len(snapshot.evidence) AS evidence_count,
                          array::len(snapshot.observations) AS observation_count,
                          array::len(snapshot.assets) AS asset_count
                   FROM snapshots
                   ORDER BY snapshot.run.startedAt DESC, run_id ASC
                   LIMIT $limit"#,
            )
            .bind(("limit", limit as u64))
            .await
            .map_err(|_| ImportHistoryError::Unavailable)?;
        let rows = response
            .take::<Vec<Value>>(0)
            .map_err(|_| ImportHistoryError::Unavailable)?;
        rows.into_iter().map(parse_import_history_row).collect()
    }
}

fn validate_censys_ip_import(result: &CensysIpImportResult) -> Result<(), StorageError> {
    let target_ip = result
        .target_ip
        .parse::<std::net::IpAddr>()
        .map_err(|_| StorageError::InvalidRecord)?
        .to_string();
    let root = result
        .event_graph
        .events
        .first()
        .ok_or(StorageError::InvalidRecord)?;
    if target_ip != result.target_ip
        || result.response_sha256.len() != 64
        || !result
            .response_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        || root.event_type != "CENSYS_HOST_RESPONSE"
        || root.data.get("targetIp").and_then(Value::as_str) != Some(result.target_ip.as_str())
        || root.data.get("responseSha256").and_then(Value::as_str)
            != Some(result.response_sha256.as_str())
        || root.metadata.get("responseSha256").and_then(Value::as_str)
            != Some(result.response_sha256.as_str())
        || root.timestamp.is_empty()
        || result
            .event_graph
            .events
            .iter()
            .any(|event| event.timestamp != root.timestamp)
    {
        return Err(StorageError::InvalidRecord);
    }
    chef_core::validate_bbot_event_graph(&result.event_graph)
        .map_err(|_| StorageError::InvalidRecord)?;
    chef_core::validate_chef_recon_graph(&result.recon_graph)
        .map_err(|_| StorageError::InvalidRecord)?;
    let expected = chef_core::adapt_bbot_to_chef(&result.event_graph)
        .map_err(|_| StorageError::InvalidRecord)?;
    if expected != result.recon_graph {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

fn censys_import_id(result: &CensysIpImportResult) -> String {
    deterministic_id(
        "censys-import",
        &[&result.target_ip, &result.response_sha256],
    )
}

fn valid_provider_import_id(import_id: &str) -> bool {
    import_id
        .strip_prefix("censys-import:")
        .is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

fn parse_import_history_row(value: Value) -> Result<ImportRunSummary, ImportHistoryError> {
    let run_id = value
        .get("run_id")
        .and_then(Value::as_str)
        .ok_or(ImportHistoryError::InvalidRecord)?;
    let schema_version = value
        .get("schema_version")
        .and_then(Value::as_str)
        .ok_or(ImportHistoryError::InvalidRecord)?;
    if !SUPPORTED_SNAPSHOT_VERSIONS.contains(&schema_version)
        || value.get("snapshot_run_id").and_then(Value::as_str) != Some(run_id)
    {
        return Err(ImportHistoryError::InvalidRecord);
    }
    let string_field = |name| {
        value
            .get(name)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or(ImportHistoryError::InvalidRecord)
    };
    let count_field = |name| {
        value
            .get(name)
            .and_then(Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .ok_or(ImportHistoryError::InvalidRecord)
    };

    Ok(ImportRunSummary {
        run_id: run_id.to_owned(),
        schema_version: schema_version.to_owned(),
        state: string_field("state")?,
        started_at: string_field("started_at")?,
        finished_at: string_field("finished_at")?,
        evidence_count: count_field("evidence_count")?,
        observation_count: count_field("observation_count")?,
        asset_count: count_field("asset_count")?,
    })
}

fn validate_snapshot_key(snapshot: &Snapshot, run_id: &str) -> Result<(), StorageError> {
    if !SUPPORTED_SNAPSHOT_VERSIONS.contains(&snapshot.schema_version.as_str()) {
        return Err(StorageError::UnsupportedVersion);
    }
    if snapshot.run.id.is_empty() || snapshot.run.id != run_id {
        return Err(StorageError::InvalidRecord);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chef_core::{FixedClock, ImportBbotEvidence, ImportEvidence, ScopePolicy};
    use std::process::Command;

    fn fixture_snapshot() -> Snapshot {
        let xml = include_bytes!("../../../fixtures/nmap/lab.xml");
        let scope: ScopePolicy =
            serde_json::from_slice(include_bytes!("../../../fixtures/scope/lab.json")).unwrap();
        ImportEvidence::execute(
            xml,
            &scope,
            &FixedClock("2026-10-18T00:00:00.000Z".into()),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn persists_and_recovers_the_same_snapshot_across_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = std::env::var_os("CHEF_STORAGE_REOPEN_PATH")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| directory.path().join("nested").join("chef-surreal"));
        let phase = std::env::var("CHEF_STORAGE_REOPEN_PHASE").ok();
        if let Some(phase) = phase {
            let snapshot = fixture_snapshot();
            let store = LocalSnapshotStore::open(&path).await.unwrap();
            match phase.as_str() {
                "write" => {
                    store.save(&snapshot).await.unwrap();
                    assert_eq!(store.load(&snapshot.run.id).await.unwrap(), Some(snapshot));
                }
                "read" => {
                    assert_eq!(store.load(&snapshot.run.id).await.unwrap(), Some(snapshot));
                }
                _ => panic!("fase de persistencia de prueba no válida"),
            }
            return;
        }

        let test_binary = std::env::current_exe().unwrap();
        for phase in ["write", "read"] {
            let output = Command::new(&test_binary)
                .arg("--exact")
                .arg("tests::persists_and_recovers_the_same_snapshot_across_reopen")
                .env("CHEF_STORAGE_REOPEN_PHASE", phase)
                .env("CHEF_STORAGE_REOPEN_PATH", &path)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "fase {phase} de persistencia falló: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    #[tokio::test]
    async fn rejects_unknown_versions_and_missing_runs() {
        let directory = tempfile::tempdir().unwrap();
        let store = LocalSnapshotStore::open(directory.path().join("chef-surreal"))
            .await
            .unwrap();
        let mut snapshot = fixture_snapshot();
        snapshot.schema_version = "9.0.0".into();
        assert!(matches!(
            store.save(&snapshot).await,
            Err(StorageError::UnsupportedVersion)
        ));
        assert!(store.load(&snapshot.run.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn persists_bbot_snapshot_version_1_1() {
        let directory = tempfile::tempdir().unwrap();
        let events = include_bytes!("../../../fixtures/event-import/events.ndjson");
        let mut scope: ScopePolicy =
            serde_json::from_slice(include_bytes!("../../../fixtures/scope/lab.json")).unwrap();
        scope.allowed_ports.push(53);
        let imported = ImportBbotEvidence::execute(
            events,
            &scope,
            &FixedClock("2026-10-18T00:00:00.000Z".into()),
            None,
        )
        .unwrap();
        let run_id = imported.snapshot.run.id.clone();
        let store = LocalSnapshotStore::open(directory.path().join("chef-surreal"))
            .await
            .unwrap();
        store.save(&imported.snapshot).await.unwrap();
        assert_eq!(store.load(&run_id).await.unwrap(), Some(imported.snapshot));
    }

    #[tokio::test]
    async fn persists_archived_censys_import_separately_from_snapshot() {
        let directory = tempfile::tempdir().unwrap();
        let body = include_bytes!("../../../fixtures/censys-ip-response/positive.json");
        let imported = chef_core::import_censys_ip_snapshot(
            "1.2.3.4",
            body,
            &FixedClock("2026-10-28T00:00:00.000Z".into()),
        )
        .unwrap();
        let store = LocalSnapshotStore::open(directory.path().join("chef-surreal"))
            .await
            .unwrap();

        let import_id = store.save_censys_ip_import(&imported).await.unwrap();
        assert_eq!(
            store.load_censys_ip_import(&import_id).await.unwrap(),
            Some(imported)
        );
        assert!(matches!(
            store.load_censys_ip_import("censys-import:invalid").await,
            Err(StorageError::InvalidRecord)
        ));
    }
}
