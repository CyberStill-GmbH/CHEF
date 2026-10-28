//! Plan local de tipos DNS y fases; no invoca el resolver ni autoriza tráfico.

use std::net::IpAddr;

use crate::{DnsQueryRequest, DnsResolutionError};

const INITIAL_NAME_TYPES: [&str; 3] = ["A", "AAAA", "CNAME"];
const EXTENDED_NAME_TYPES: [&str; 5] = ["SRV", "MX", "NS", "SOA", "TXT"];
const MAX_DNS_DISTANCE: u8 = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsQueryPlanSettings {
    minimal: bool,
    scope_distance: u8,
    search_distance: u8,
    resolution_hops: u8,
}

impl DnsQueryPlanSettings {
    pub fn new(
        minimal: bool,
        scope_distance: u8,
        search_distance: u8,
        resolution_hops: u8,
    ) -> Result<Self, DnsResolutionError> {
        if scope_distance > MAX_DNS_DISTANCE
            || search_distance > MAX_DNS_DISTANCE
            || resolution_hops > MAX_DNS_DISTANCE
        {
            return Err(DnsResolutionError::LimitExceeded);
        }
        Ok(Self {
            minimal,
            scope_distance,
            search_distance,
            resolution_hops,
        })
    }

    fn includes_extended_records(self) -> bool {
        !self.minimal && self.scope_distance < self.search_distance
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsQueryStage {
    InitialResolution,
    ExtendedRecords,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedDnsQuery {
    stage: DnsQueryStage,
    request: DnsQueryRequest,
}

impl PlannedDnsQuery {
    pub fn stage(&self) -> DnsQueryStage {
        self.stage
    }

    pub fn request(&self) -> &DnsQueryRequest {
        &self.request
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsQueryPlan {
    host: String,
    resolution_hops: u8,
    queries: Vec<PlannedDnsQuery>,
}

impl DnsQueryPlan {
    /// Planea la resolución de un host con el orden y la distancia de búsqueda CHEF.
    ///
    /// Las IP solo generan PTR. Los nombres generan A/AAAA/CNAME primero y
    /// SRV/MX/NS/SOA/TXT solo cuando el modo no es mínimo y la distancia del
    /// objetivo es menor que la distancia máxima de búsqueda. La autorización
    /// se consulta individualmente al ejecutar cada `DnsQueryRequest`.
    pub fn for_target(
        host: &str,
        settings: DnsQueryPlanSettings,
    ) -> Result<Self, DnsResolutionError> {
        let first_request = DnsQueryRequest::new(host, "A", settings.resolution_hops)?;
        let host = first_request.host().to_owned();
        let is_ip = host.parse::<IpAddr>().is_ok();
        let capacity = if is_ip {
            1
        } else if !settings.includes_extended_records() {
            INITIAL_NAME_TYPES.len()
        } else {
            INITIAL_NAME_TYPES.len() + EXTENDED_NAME_TYPES.len()
        };
        let mut queries = Vec::with_capacity(capacity);

        if is_ip {
            queries.push(PlannedDnsQuery {
                stage: DnsQueryStage::InitialResolution,
                request: DnsQueryRequest::new(&host, "PTR", settings.resolution_hops)?,
            });
        } else {
            append_queries(
                &mut queries,
                &host,
                settings.resolution_hops,
                DnsQueryStage::InitialResolution,
                &INITIAL_NAME_TYPES,
            )?;
            if settings.includes_extended_records() {
                append_queries(
                    &mut queries,
                    &host,
                    settings.resolution_hops,
                    DnsQueryStage::ExtendedRecords,
                    &EXTENDED_NAME_TYPES,
                )?;
            }
        }

        Ok(Self {
            host,
            resolution_hops: settings.resolution_hops,
            queries,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn resolution_hops(&self) -> u8 {
        self.resolution_hops
    }

    pub fn queries(&self) -> &[PlannedDnsQuery] {
        &self.queries
    }

    pub fn queries_for(&self, stage: DnsQueryStage) -> impl Iterator<Item = &PlannedDnsQuery> {
        self.queries
            .iter()
            .filter(move |query| query.stage == stage)
    }
}

fn append_queries(
    queries: &mut Vec<PlannedDnsQuery>,
    host: &str,
    resolution_hops: u8,
    stage: DnsQueryStage,
    record_types: &[&str],
) -> Result<(), DnsResolutionError> {
    for record_type in record_types {
        queries.push(PlannedDnsQuery {
            stage,
            request: DnsQueryRequest::new(host, record_type, resolution_hops)?,
        });
    }
    Ok(())
}
