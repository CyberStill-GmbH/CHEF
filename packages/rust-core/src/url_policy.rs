//! Validaciones y métricas locales para destinos HTTP de CHEF.
//! Estas funciones no autorizan tráfico ni resuelven nombres.

use std::net::IpAddr;

use crate::url_tools::canonical_http_url;

/// Canonicaliza una IP o un nombre DNS con al menos dos etiquetas.
///
/// Se usa para campos que requieren un destino completo. No implica que el
/// destino esté autorizado ni que pueda recibir conexiones.
pub fn canonical_fqdn_or_ip(value: &str) -> Option<String> {
    let host = crate::host::canonical_host(value)?;
    if host.parse::<IpAddr>().is_ok() {
        return Some(host);
    }

    let labels: Vec<_> = host.split('.').collect();
    if labels.len() < 2
        || labels
            .iter()
            .any(|label| label.is_empty() || label.len() > 63)
    {
        return None;
    }
    Some(host)
}

/// Cuenta los segmentos no vacíos de la ruta de una URL HTTP(S) canónica.
/// Query, fragmento, credenciales y la ruta misma no se devuelven.
pub fn url_path_depth(value: &str) -> Option<usize> {
    let canonical = canonical_http_url(value)?;
    let url = url::Url::parse(&canonical).ok()?;
    Some(
        url.path_segments()
            .into_iter()
            .flatten()
            .filter(|segment| !segment.is_empty())
            .count(),
    )
}
