//! Canonicalización local de hosts adaptada de los validadores BBOT.
//! No resuelve nombres ni hace solicitudes de red.

use std::net::{IpAddr, Ipv6Addr};

use url::{Host, Url};

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

/// Canonicaliza un nombre DNS y exige etiquetas válidas después de IDNA.
/// `canonical_domain` normaliza el host, pero no aplica por sí sola la gramática
/// de etiquetas DNS (por ejemplo, puede aceptar guiones bajos).
pub(crate) fn canonical_dns_name(value: &str) -> Option<String> {
    let name = canonical_domain(value)?;
    if name.len() > 253
        || !name.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && label.as_bytes()[0].is_ascii_alphanumeric()
                && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return None;
    }
    Some(name)
}

/// Adapta `misc.split_host_port` para entradas de host/netloc. Acepta IP,
/// host con puerto y esquemas comunes; rechaza credenciales, rutas y controles.
/// Nunca resuelve el host ni abre conexiones.
pub(crate) fn split_host_port(value: &str) -> Option<(String, Option<u16>)> {
    let value = value.trim();
    if value.is_empty()
        || value.len() > 8_192
        || value.chars().any(char::is_control)
        || value.chars().any(char::is_whitespace)
        || value.contains(['\\', '@', '?', '#'])
    {
        return None;
    }
    if let Ok(address) = value.parse::<IpAddr>() {
        return Some((address.to_string(), None));
    }

    let (scheme, authority) = match value.split_once("://") {
        Some((scheme, authority)) => {
            if scheme.is_empty()
                || scheme.len() > 20
                || !scheme.bytes().all(|byte| byte.is_ascii_alphanumeric())
            {
                return None;
            }
            (Some(scheme.to_ascii_lowercase()), authority)
        }
        None => (None, value),
    };
    if authority.is_empty() || authority.contains(['/', '\\']) {
        return None;
    }

    let url = Url::parse(&format!("tcp://{authority}")).ok()?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.path().is_empty() || url.path() == "/")
    {
        return None;
    }
    let host = match url.host()? {
        Host::Domain(domain) => domain.to_owned(),
        Host::Ipv4(address) => address.to_string(),
        Host::Ipv6(address) => address.to_string(),
    };
    let default_port = match scheme.as_deref() {
        Some("http" | "ws") => Some(80),
        Some("https" | "wss") => Some(443),
        _ => None,
    };
    Some((host, url.port().or(default_port)))
}

/// Adapta `misc.make_netloc`: encierra literales IPv6 entre corchetes y añade
/// el puerto opcional. Los llamadores validan el host antes de formatear.
pub(crate) fn make_netloc(host: &str, port: Option<u16>) -> String {
    let formatted_host = if host.starts_with('[') && host.ends_with(']') {
        host.to_owned()
    } else if host.parse::<Ipv6Addr>().is_ok() {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    match port {
        Some(port) => format!("{formatted_host}:{port}"),
        None => formatted_host,
    }
}

fn icann_registrable_domain(value: &str) -> Option<(String, String)> {
    let host = canonical_domain(value)?;
    let registrable = psl::domain(host.as_bytes())?;
    if registrable.suffix().typ() != Some(psl::Type::Icann) {
        return None;
    }
    let registrable = String::from_utf8_lossy(registrable.as_bytes()).into_owned();
    Some((host, registrable))
}

/// Indica si el nombre canónico es una raíz registrable bajo un sufijo ICANN
/// de la PSL integrada. Los sufijos privados/desconocidos no se infieren.
pub(crate) fn is_registrable_domain(value: &str) -> bool {
    icann_registrable_domain(value).is_some_and(|(host, registrable)| host == registrable)
}

/// Indica si el nombre es descendiente de una raíz registrable ICANN conocida.
/// No realiza resolución ni concede autorización de alcance.
pub(crate) fn is_subdomain(value: &str) -> bool {
    icann_registrable_domain(value).is_some_and(|(host, registrable)| {
        host != registrable && host.ends_with(&format!(".{registrable}"))
    })
}

/// Devuelve los dominios padre de un host dentro de una raíz registrable
/// ICANN. Se adapta a `domain_parents` de las utilidades de origen, con la
/// política CHEF de no expandir sufijos privados o desconocidos. El resultado
/// es solo contexto local; no implica propiedad ni autorización de red.
pub(crate) fn domain_parent_candidates(value: &str, include_self: bool) -> Vec<String> {
    if is_registrable_domain(value) || !is_subdomain(value) {
        return Vec::new();
    }
    let Some((host, registrable)) = icann_registrable_domain(value) else {
        return Vec::new();
    };

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

/// Devuelve zonas padre DNS de la más general a la más específica.
///
/// Solo usa una raíz registrable ICANN presente en la PSL integrada; el
/// resultado organiza contexto local y no concede autorización para consultar.
pub fn dns_parent_zones(value: &str) -> Vec<String> {
    domain_parent_candidates(value, false)
        .into_iter()
        .rev()
        .collect()
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
