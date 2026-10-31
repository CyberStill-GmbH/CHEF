//! Caso de uso para normalizar localmente respuestas Censys archivadas.
//! No realiza HTTP ni otras consultas de red.

use chef_core::{CensysIpImportResult, ChefError, ChefResult, ErrorCode, FixedClock};

const MAX_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

pub async fn import_censys_ip_response(
    target_ip: String,
    response: Vec<u8>,
    imported_at: String,
) -> ChefResult<CensysIpImportResult> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta Censys supera el máximo de 2 MiB CHEF",
        ));
    }

    tokio::task::spawn_blocking(move || {
        chef_core::import_censys_ip_snapshot(&target_ip, &response, &FixedClock(imported_at))
    })
    .await
    .map_err(|_| {
        ChefError::new(
            ErrorCode::WorkerFailed,
            "No se pudo completar la importación local Censys",
        )
    })?
}
