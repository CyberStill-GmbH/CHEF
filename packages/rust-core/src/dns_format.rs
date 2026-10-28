//! Presentación segura de texto RDATA DNS ya obtenido.
//!
//! El formato del registro lo produce la capa de resolución y se conserva como
//! texto; el núcleo CHEF no consulta DNS ni intenta reconstruir datos binarios.

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_DNS_RDATA_TEXT_BYTES: usize = 64 * 1024;

/// Devuelve la representación textual del RDATA después de aplicar el límite CHEF.
///
/// El resolver puede entregar su representación estándar (`Record::to_text`).
/// CHEF la presenta sin reescrituras para no alterar evidencia ni asumir una
/// gramática común a todos los tipos de registro.
pub fn format_dns_rdata(record_text: &str) -> ChefResult<&str> {
    if record_text.len() > MAX_DNS_RDATA_TEXT_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El texto del registro DNS supera el máximo CHEF de bytes",
        ));
    }
    Ok(record_text)
}
