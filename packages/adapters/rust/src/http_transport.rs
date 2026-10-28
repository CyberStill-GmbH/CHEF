//! Transporte HTTP de una sola solicitud, fijado a IPs autorizadas.
//!
//! Construir el adaptador no hace conexiones. La ejecución ocurre solo si una
//! composición invoca `HttpTransportPort` después de autorizar el destino.

use std::net::{IpAddr, SocketAddr};

use chef_application::{
    HttpTransportFailure, HttpTransportPort, HttpTransportRequest, HttpTransportResponse,
};
use reqwest::{Client, Url, header::LOCATION};
use url::Host;

/// Adaptador asíncrono Reqwest con redirects/proxy/reintentos automáticos
/// desactivados y resolución limitada a las direcciones entregadas por CHEF.
#[derive(Debug, Clone, Copy, Default)]
pub struct ReqwestHttpTransport;

impl HttpTransportPort for ReqwestHttpTransport {
    async fn get_once(
        &self,
        request: HttpTransportRequest,
    ) -> Result<HttpTransportResponse, HttpTransportFailure> {
        let url = Url::parse(request.url()).map_err(|_| HttpTransportFailure::InvalidResponse)?;
        if !matches!(url.scheme(), "http" | "https")
            || !url.username().is_empty()
            || url.password().is_some()
            || request.allowed_ips().is_empty()
        {
            return Err(HttpTransportFailure::InvalidResponse);
        }
        let port = url
            .port_or_known_default()
            .filter(|port| *port != 0)
            .ok_or(HttpTransportFailure::InvalidResponse)?;
        let host = url.host().ok_or(HttpTransportFailure::InvalidResponse)?;

        let mut builder = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .retry(reqwest::retry::never())
            .no_proxy()
            .referer(false)
            .timeout(request.timeout())
            .connect_timeout(request.timeout());

        match host {
            Host::Domain(domain) => {
                let addresses = request
                    .allowed_ips()
                    .iter()
                    .copied()
                    .map(|ip| SocketAddr::new(ip, port))
                    .collect::<Vec<_>>();
                if addresses.is_empty() {
                    return Err(HttpTransportFailure::InvalidResponse);
                }
                builder = builder.resolve_to_addrs(domain, &addresses);
            }
            _ => {
                let target = host_ip(&url).ok_or(HttpTransportFailure::InvalidResponse)?;
                if !request.allowed_ips().contains(&target) {
                    return Err(HttpTransportFailure::Connect);
                }
            }
        }

        let client = builder.build().map_err(|_| HttpTransportFailure::Other)?;
        let mut response = client.get(url).send().await.map_err(map_reqwest_error)?;
        let peer_ip = response
            .remote_addr()
            .map(|address| address.ip())
            .ok_or(HttpTransportFailure::InvalidResponse)?;
        let status = response.status().as_u16();
        let location = response
            .headers()
            .get(LOCATION)
            .map(|value| {
                value
                    .to_str()
                    .map(str::to_owned)
                    .map_err(|_| HttpTransportFailure::InvalidResponse)
            })
            .transpose()?;

        let max_bytes = request.max_response_bytes();
        if response
            .content_length()
            .is_some_and(|size| size > max_bytes as u64)
        {
            return Ok(HttpTransportResponse {
                status,
                peer_ip,
                response_bytes: max_bytes.saturating_add(1),
                location,
            });
        }

        let mut response_bytes = 0_usize;
        while let Some(chunk) = response.chunk().await.map_err(map_reqwest_error)? {
            response_bytes = response_bytes.saturating_add(chunk.len());
            if response_bytes > max_bytes {
                response_bytes = max_bytes.saturating_add(1);
                break;
            }
        }

        Ok(HttpTransportResponse {
            status,
            peer_ip,
            response_bytes,
            location,
        })
    }
}

fn host_ip(url: &Url) -> Option<IpAddr> {
    match url.host()? {
        Host::Ipv4(address) => Some(IpAddr::V4(address)),
        Host::Ipv6(address) => Some(IpAddr::V6(address)),
        Host::Domain(_) => None,
    }
}

fn map_reqwest_error(error: reqwest::Error) -> HttpTransportFailure {
    if error.is_timeout() {
        HttpTransportFailure::Timeout
    } else if error.is_connect() {
        HttpTransportFailure::Connect
    } else if error.is_body() || error.is_decode() {
        HttpTransportFailure::InvalidResponse
    } else {
        HttpTransportFailure::Other
    }
}
