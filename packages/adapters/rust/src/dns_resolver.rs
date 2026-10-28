//! Adaptador Hickory con servidores DNS explícitos y límites conservadores.
//!
//! Construir este adaptador no inicia tráfico. Cada consulta requiere que una
//! aplicación invoque el puerto desde un caso de uso con autorización vigente.

use std::{collections::BTreeSet, net::IpAddr, time::Duration};

use chef_application::{DnsAnswer, DnsQueryRequest, DnsResolverFailure, DnsResolverPort};
use hickory_resolver::{
    TokioResolver,
    config::{NameServerConfig, ResolveHosts, ResolverConfig, ResolverOpts},
    net::runtime::TokioRuntimeProvider,
    proto::{op::ResponseCode, rr::RecordType},
};

const MIN_UPSTREAMS: usize = 1;
const MAX_UPSTREAMS: usize = 4;
const MIN_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsResolverConfigError {
    InvalidUpstreamCount,
    DuplicateUpstream,
    InvalidTimeout,
    ResolverInitialization,
}

/// Configuración de upstreams por IP literal; nunca lee resolv.conf ni hosts del SO.
#[derive(Debug, Clone)]
pub struct DnsResolverConfig {
    nameservers: Vec<IpAddr>,
    timeout: Duration,
}

impl DnsResolverConfig {
    pub fn new(
        nameservers: Vec<IpAddr>,
        timeout: Duration,
    ) -> Result<Self, DnsResolverConfigError> {
        if !(MIN_UPSTREAMS..=MAX_UPSTREAMS).contains(&nameservers.len()) {
            return Err(DnsResolverConfigError::InvalidUpstreamCount);
        }
        if nameservers.iter().collect::<BTreeSet<_>>().len() != nameservers.len() {
            return Err(DnsResolverConfigError::DuplicateUpstream);
        }
        if !(MIN_TIMEOUT..=MAX_TIMEOUT).contains(&timeout) {
            return Err(DnsResolverConfigError::InvalidTimeout);
        }
        Ok(Self {
            nameservers,
            timeout,
        })
    }
}

/// Implementa el puerto de aplicación con `hickory-resolver`.
pub struct HickoryDnsResolver {
    resolver: TokioResolver,
    timeout: Duration,
}

impl HickoryDnsResolver {
    pub fn new(config: DnsResolverConfig) -> Result<Self, DnsResolverConfigError> {
        let name_servers = config
            .nameservers
            .into_iter()
            .map(NameServerConfig::udp_and_tcp)
            .collect();
        let resolver_config = ResolverConfig::from_name_servers(name_servers);
        let mut options = ResolverOpts::default();
        options.timeout = config.timeout;
        options.attempts = 1;
        options.max_active_requests = 1;
        options.num_concurrent_reqs = 1;
        options.use_hosts_file = ResolveHosts::Never;
        options.cache_size = 0;
        options.preserve_intermediates = true;

        let resolver =
            TokioResolver::builder_with_config(resolver_config, TokioRuntimeProvider::default())
                .with_options(options)
                .build()
                .map_err(|_| DnsResolverConfigError::ResolverInitialization)?;

        Ok(Self {
            resolver,
            timeout: config.timeout,
        })
    }
}

impl DnsResolverPort for HickoryDnsResolver {
    async fn lookup(
        &self,
        request: &DnsQueryRequest,
    ) -> Result<Vec<DnsAnswer>, DnsResolverFailure> {
        let record_type = parse_record_type(request.record_type().as_str())
            .ok_or(DnsResolverFailure::InvalidResponse)?;
        let query_name = dns_query_name(request.host(), request.record_type().as_str())
            .ok_or(DnsResolverFailure::InvalidResponse)?;
        let lookup =
            tokio::time::timeout(self.timeout, self.resolver.lookup(query_name, record_type))
                .await
                .map_err(|_| DnsResolverFailure::Timeout)?;
        let lookup = match lookup {
            Ok(lookup) => lookup,
            Err(hickory_resolver::net::NetError::Dns(
                hickory_resolver::net::DnsError::NoRecordsFound(no_records),
            )) if no_records.response_code == ResponseCode::NoError => {
                return Ok(Vec::new());
            }
            Err(error) => return Err(map_resolver_error(error)),
        };

        lookup
            .answers()
            .iter()
            .map(|record| {
                DnsAnswer::new(
                    &record.record_type().to_string(),
                    record.data.to_string(),
                    Duration::from_secs(u64::from(record.ttl)),
                )
                .map_err(|_| DnsResolverFailure::InvalidResponse)
            })
            .collect()
    }
}

fn parse_record_type(value: &str) -> Option<RecordType> {
    match value.to_ascii_uppercase().as_str() {
        "A" => Some(RecordType::A),
        "AAAA" => Some(RecordType::AAAA),
        "CNAME" => Some(RecordType::CNAME),
        "MX" => Some(RecordType::MX),
        "NS" => Some(RecordType::NS),
        "PTR" => Some(RecordType::PTR),
        "SOA" => Some(RecordType::SOA),
        "SRV" => Some(RecordType::SRV),
        "TXT" => Some(RecordType::TXT),
        _ => None,
    }
}

fn dns_query_name(host: &str, record_type: &str) -> Option<String> {
    if record_type.eq_ignore_ascii_case("PTR") {
        return reverse_name(host);
    }
    let host = host.strip_suffix('.').unwrap_or(host);
    if host.is_empty() {
        return None;
    }
    Some(format!("{host}."))
}

fn reverse_name(host: &str) -> Option<String> {
    let address = host.parse::<IpAddr>().ok()?;
    Some(match address {
        IpAddr::V4(address) => {
            let octets = address.octets();
            format!(
                "{}.{}.{}.{}.in-addr.arpa.",
                octets[3], octets[2], octets[1], octets[0]
            )
        }
        IpAddr::V6(address) => {
            let nibbles = address
                .octets()
                .into_iter()
                .flat_map(|octet| [octet >> 4, octet & 0x0f])
                .rev()
                .map(|nibble| format!("{nibble:x}"))
                .collect::<Vec<_>>()
                .join(".");
            format!("{nibbles}.ip6.arpa.")
        }
    })
}

fn map_resolver_error(error: hickory_resolver::net::NetError) -> DnsResolverFailure {
    use hickory_resolver::net::{DnsError, NetError};

    match error {
        NetError::Dns(DnsError::NoRecordsFound(no_records)) => match no_records.response_code {
            ResponseCode::NXDomain => DnsResolverFailure::NameNotFound,
            _ => DnsResolverFailure::Other,
        },
        NetError::Dns(DnsError::ResponseCode(ResponseCode::ServFail)) => {
            DnsResolverFailure::ServerFailure
        }
        NetError::Dns(DnsError::ResponseCode(ResponseCode::Refused)) => DnsResolverFailure::Refused,
        NetError::Timeout => DnsResolverFailure::Timeout,
        NetError::NoConnections | NetError::Io(_) => DnsResolverFailure::Network,
        _ => DnsResolverFailure::Other,
    }
}
