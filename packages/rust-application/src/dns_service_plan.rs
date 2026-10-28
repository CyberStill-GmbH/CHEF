//! Plan local de nombres propietarios para consultas DNS SRV.
//!
//! Este caso de uso prepara candidatos desde el catálogo CHEF, pero no crea
//! solicitudes de red, no valida respuestas y no concede autorización.

use std::collections::BTreeSet;

use chef_core::{ChefTargetKind, common_dns_service_candidates, parse_chef_target_seed};

use crate::DnsResolutionError;

const MAX_DNS_SERVICE_QUERIES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServiceQueryCandidate {
    owner_name: String,
}

impl DnsServiceQueryCandidate {
    pub fn owner_name(&self) -> &str {
        &self.owner_name
    }

    pub const fn record_type(&self) -> &'static str {
        "SRV"
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsServiceQueryPlan {
    root_domain: String,
    candidates: Vec<DnsServiceQueryCandidate>,
}

impl DnsServiceQueryPlan {
    /// Prepara como máximo 256 nombres del catálogo estático para un dominio raíz.
    ///
    /// Los nombres empiezan por etiquetas de servicio con `_`. Son candidatos
    /// de propietario SRV y no deben convertirse en activos observados ni
    /// ejecutarse antes de una autorización por consulta.
    pub fn for_root_domain(root_domain: &str) -> Result<Self, DnsResolutionError> {
        let seed =
            parse_chef_target_seed(root_domain).map_err(|_| DnsResolutionError::InvalidQuery)?;
        if seed.kind != ChefTargetKind::DnsName {
            return Err(DnsResolutionError::InvalidQuery);
        }
        let root_domain = seed.host.ok_or(DnsResolutionError::InvalidQuery)?;
        let owners = common_dns_service_candidates(&root_domain)
            .map_err(|_| DnsResolutionError::InvalidQuery)?;
        if owners.len() > MAX_DNS_SERVICE_QUERIES {
            return Err(DnsResolutionError::LimitExceeded);
        }

        let mut seen = BTreeSet::new();
        let candidates = owners
            .into_iter()
            .filter(|owner_name| seen.insert(owner_name.clone()))
            .map(|owner_name| DnsServiceQueryCandidate { owner_name })
            .collect();

        Ok(Self {
            root_domain,
            candidates,
        })
    }

    pub fn root_domain(&self) -> &str {
        &self.root_domain
    }

    pub fn candidates(&self) -> &[DnsServiceQueryCandidate] {
        &self.candidates
    }
}
