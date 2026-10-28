//! Adaptador de persistencia local CHEF con SurrealDB embebida y RocksDB.
//! El core no conoce archivos ni almacenamiento.

use std::path::Path;

use chef_application::{
    IMPORT_HISTORY_MAX_PAGE_SIZE, ImportHistoryError, ImportHistoryPort, ImportOutput,
    ImportRunSummary,
};
use chef_core::{BbotEventGraph, BbotImportResult, ChefReconGraph, Snapshot};
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
             DEFINE FIELD IF NOT EXISTS snapshot ON snapshots TYPE object;
             DEFINE FIELD IF NOT EXISTS event_graph ON snapshots TYPE any;
             DEFINE FIELD IF NOT EXISTS chef_recon_graph ON snapshots TYPE any;",
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
        let path = directory.path().join("nested").join("chef-surreal");
        let snapshot = fixture_snapshot();
        let run_id = snapshot.run.id.clone();

        {
            let store = LocalSnapshotStore::open(&path).await.unwrap();
            store.save(&snapshot).await.unwrap();
            assert_eq!(store.load(&run_id).await.unwrap(), Some(snapshot.clone()));
        }

        let reopened = LocalSnapshotStore::open(&path).await.unwrap();
        assert_eq!(reopened.load(&run_id).await.unwrap(), Some(snapshot));
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
}
