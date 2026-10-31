//! Caso de uso para normalizar localmente respuestas urlscan archivadas.
//! No realiza HTTP ni consultas DNS.

use chef_core::{ChefError, ChefResult, ErrorCode, FixedClock, UrlScanImportResult};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub async fn import_urlscan_snapshot(
    query_domain: String,
    response: Vec<u8>,
    imported_at: String,
) -> ChefResult<UrlScanImportResult> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta urlscan supera el máximo de 2 MiB CHEF",
        ));
    }

    tokio::task::spawn_blocking(move || {
        chef_core::import_urlscan_snapshot(&query_domain, &response, &FixedClock(imported_at))
    })
    .await
    .map_err(|_| {
        ChefError::new(
            ErrorCode::WorkerFailed,
            "No se pudo completar la importación local urlscan",
        )
    })?
}
