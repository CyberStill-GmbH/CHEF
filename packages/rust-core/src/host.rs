//! Canonicalización local de hosts adaptada de los validadores BBOT.
//! No resuelve nombres ni hace solicitudes de red.

use std::net::IpAddr;

pub(crate) fn canonical_host(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 1024
        || value.chars().any(char::is_control)
        || value.contains(['*', '/', '\\', '@', '?', '#'])
    {
        return None;
    }
    if let Some(address) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        return address
            .parse::<IpAddr>()
            .ok()
            .map(|address| address.to_string());
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return Some(address.to_string());
    }
    let value = value.trim_end_matches('.');
    if value.is_empty() || value.len() > 253 || value.contains(':') {
        return None;
    }
    match url::Host::parse(value).ok()? {
        url::Host::Domain(domain) if !domain.is_empty() && domain.len() <= 253 => {
            Some(domain.to_owned())
        }
        _ => None,
    }
}

pub(crate) fn canonical_domain(value: &str) -> Option<String> {
    let host = canonical_host(value)?;
    host.parse::<IpAddr>().is_err().then_some(host)
}

/// Devuelve los dominios padre de un host dentro de una raíz registrable
/// ICANN. Se adapta a `domain_parents` de las utilidades de origen, con la
/// política CHEF de no expandir sufijos privados o desconocidos. El resultado
/// es solo contexto local; no implica propiedad ni autorización de red.
pub(crate) fn domain_parent_candidates(value: &str, include_self: bool) -> Vec<String> {
    let Some(host) = canonical_domain(value) else {
        return Vec::new();
    };
    let Some(registrable) = psl::domain(host.as_bytes()) else {
        return Vec::new();
    };
    if registrable.suffix().typ() != Some(psl::Type::Icann) {
        return Vec::new();
    }

    let registrable = String::from_utf8_lossy(registrable.as_bytes()).into_owned();
    if host == registrable {
        return Vec::new();
    }

    let root_depth = registrable.split('.').count();
    let mut labels = host.split('.').collect::<Vec<_>>();
    let mut parents = Vec::new();
    if include_self {
        parents.push(host.clone());
    }
    while labels.len() > root_depth {
        labels.remove(0);
        parents.push(labels.join("."));
    }
    parents
}

pub(crate) fn hosts_match(host: &str, allowed: &str) -> bool {
    let (Some(host), Some(allowed)) = (canonical_host(host), canonical_host(allowed)) else {
        return false;
    };
    let host_is_ip = host.parse::<IpAddr>().is_ok();
    let allowed_is_ip = allowed.parse::<IpAddr>().is_ok();
    if host_is_ip || allowed_is_ip {
        return host == allowed;
    }
    host == allowed || host.ends_with(&format!(".{allowed}"))
}
