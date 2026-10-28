//! Consulta de importaciones locales; no crea un modelo CRUD de proyectos.

use std::future::Future;

pub const IMPORT_HISTORY_MAX_PAGE_SIZE: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRunSummary {
    pub run_id: String,
    pub schema_version: String,
    pub state: String,
    pub started_at: String,
    pub finished_at: String,
    pub evidence_count: usize,
    pub observation_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportHistoryError {
    InvalidLimit,
    Unavailable,
    InvalidRecord,
}

pub trait ImportHistoryPort: Send + Sync {
    fn list_recent(
        &self,
        limit: usize,
    ) -> impl Future<Output = Result<Vec<ImportRunSummary>, ImportHistoryError>> + Send;
}

pub struct ListImportRuns<'a, R> {
    repository: &'a R,
}

impl<'a, R> ListImportRuns<'a, R>
where
    R: ImportHistoryPort,
{
    pub fn new(repository: &'a R) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, limit: usize) -> Result<Vec<ImportRunSummary>, ImportHistoryError> {
        if limit == 0 || limit > IMPORT_HISTORY_MAX_PAGE_SIZE {
            return Err(ImportHistoryError::InvalidLimit);
        }
        self.repository.list_recent(limit).await
    }
}
