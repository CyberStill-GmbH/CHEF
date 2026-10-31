//! Deriva candidatos de direcciones vecinas desde una IP importada.
//!
//! La lista no representa activos observados ni amplía scope. No se resuelven
//! nombres, no se consulta la red y ningún candidato queda autorizado para uso.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::{
    ChefError, ChefResult, ErrorCode,
    contracts::{SCHEMA_VERSION, ScopePolicy},
    target_seed::{canonical_network, network_contains},
};

const MAX_EXPANSION_BITS: u8 = 8;
const MAX_CANDIDATES: usize = (1 << MAX_EXPANSION_BITS) - 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IpNeighborCandidateSet {
    pub seed_ip: String,
    pub scope_policy_id: String,
    pub expansion_bits: u8,
    pub candidates: Vec<String>,
}

/// Genera hasta 255 candidatos del mismo bloque binario de la IP semilla.
///
/// Este resultado solo es una derivación local para revisión humana. No
/// modifica la política de scope ni inicia escaneo.
pub fn derive_ip_neighbor_candidates(
    seed_ip: &str,
    expansion_bits: u8,
    scope_policy: &ScopePolicy,
) -> ChefResult<IpNeighborCandidateSet> {
    if !(1..=MAX_EXPANSION_BITS).contains(&expansion_bits) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La expansión local de IP debe estar entre 1 y 8 bits",
        ));
    }
    let seed_ip = seed_ip
        .trim()
        .parse::<IpAddr>()
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "La IP semilla no es válida"))?;
    let (allowed_addresses, excluded_addresses) = normalize_scope(scope_policy)?;
    if !address_in_scope(seed_ip, &allowed_addresses, &excluded_addresses) {
        return Err(ChefError::new(
            ErrorCode::OutOfScope,
            "La IP semilla no está dentro del scope CHEF explícito",
        ));
    }
    let candidates = match seed_ip {
        IpAddr::V4(seed) => ipv4_neighbors(seed, expansion_bits),
        IpAddr::V6(seed) => ipv6_neighbors(seed, expansion_bits),
    };
    if candidates.len() > MAX_CANDIDATES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista de candidatos IP supera el máximo CHEF",
        ));
    }
    let candidates = candidates
        .into_iter()
        .filter(|candidate| address_in_scope(*candidate, &allowed_addresses, &excluded_addresses))
        .map(|candidate| candidate.to_string())
        .collect();
    Ok(IpNeighborCandidateSet {
        seed_ip: seed_ip.to_string(),
        scope_policy_id: scope_policy.id.clone(),
        expansion_bits,
        candidates,
    })
}

fn normalize_scope(scope: &ScopePolicy) -> ChefResult<(Vec<String>, Vec<String>)> {
    if scope.schema_version != SCHEMA_VERSION
        || scope.id.trim().is_empty()
        || scope.id.len() > 128
        || scope.id.chars().any(char::is_control)
        || scope.allowed_addresses.is_empty()
        || scope.allowed_addresses.len() > 128
        || scope.excluded_addresses.len() > 128
    {
        return Err(ChefError::new(
            ErrorCode::OutOfScope,
            "Se requiere una política CHEF de scope válida y explícita",
        ));
    }
    let allowed = scope
        .allowed_addresses
        .iter()
        .map(|entry| canonical_scope_entry(entry))
        .collect::<ChefResult<Vec<_>>>()?;
    let excluded = scope
        .excluded_addresses
        .iter()
        .map(|entry| canonical_scope_entry(entry))
        .collect::<ChefResult<Vec<_>>>()?;
    Ok((allowed, excluded))
}

fn canonical_scope_entry(entry: &str) -> ChefResult<String> {
    let entry = entry.trim();
    if entry.len() > 64 || entry.chars().any(char::is_control) {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Una entrada de scope supera el límite CHEF",
        ));
    }
    if entry.contains('/') {
        return canonical_network(entry).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "La política contiene una red IP inválida",
        ));
    }
    entry
        .parse::<IpAddr>()
        .map(|address| address.to_string())
        .map_err(|_| {
            ChefError::new(
                ErrorCode::InvalidInput,
                "La política contiene una IP inválida",
            )
        })
}

fn address_in_scope(address: IpAddr, allowed: &[String], excluded: &[String]) -> bool {
    !excluded
        .iter()
        .any(|entry| scope_entry_contains(entry, address))
        && allowed
            .iter()
            .any(|entry| scope_entry_contains(entry, address))
}

fn scope_entry_contains(entry: &str, address: IpAddr) -> bool {
    if entry.contains('/') {
        network_contains(entry, address)
    } else {
        entry.parse::<IpAddr>().ok() == Some(address)
    }
}

fn ipv4_neighbors(seed: Ipv4Addr, bits: u8) -> Vec<IpAddr> {
    let block_size = 1u32 << bits;
    let seed_number = u32::from(seed);
    let network = seed_number & !(block_size - 1);
    let mut candidates = Vec::with_capacity((block_size - 1) as usize);
    for offset in 0..block_size {
        let Some(candidate) = network.checked_add(offset) else {
            break;
        };
        if candidate != seed_number {
            candidates.push(IpAddr::V4(Ipv4Addr::from(candidate)));
        }
    }
    candidates
}

fn ipv6_neighbors(seed: Ipv6Addr, bits: u8) -> Vec<IpAddr> {
    let block_size = 1u128 << bits;
    let seed_number = u128::from(seed);
    let network = seed_number & !(block_size - 1);
    let mut candidates = Vec::with_capacity((block_size - 1) as usize);
    for offset in 0..block_size {
        let Some(candidate) = network.checked_add(offset) else {
            break;
        };
        if candidate != seed_number {
            candidates.push(IpAddr::V6(Ipv6Addr::from(candidate)));
        }
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::derive_ip_neighbor_candidates;
    use crate::{
        ErrorCode,
        contracts::{SCHEMA_VERSION, ScopePolicy},
    };

    fn scope(allowed_addresses: &[&str], excluded_addresses: &[&str]) -> ScopePolicy {
        ScopePolicy {
            schema_version: SCHEMA_VERSION.to_owned(),
            id: "synthetic-scope".to_owned(),
            allowed_addresses: allowed_addresses
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            excluded_addresses: excluded_addresses
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            allowed_ports: Vec::new(),
            allowed_protocols: Vec::new(),
            max_input_bytes: 1024,
            max_observations: 1024,
        }
    }

    #[test]
    fn derives_only_allowed_ipv4_candidates_and_applies_exclusions() {
        let result = derive_ip_neighbor_candidates(
            "192.0.2.10",
            2,
            &scope(&["192.0.2.8/30"], &["192.0.2.9"]),
        )
        .unwrap();
        assert_eq!(result.seed_ip, "192.0.2.10");
        assert_eq!(result.scope_policy_id, "synthetic-scope");
        assert_eq!(result.candidates, ["192.0.2.8", "192.0.2.11"]);
        assert!(!result.candidates.contains(&result.seed_ip));
    }

    #[test]
    fn derives_ipv6_candidates_but_rejects_seed_outside_scope() {
        let result =
            derive_ip_neighbor_candidates("2001:db8::5", 2, &scope(&["2001:db8::4/126"], &[]))
                .unwrap();
        assert_eq!(
            result.candidates,
            ["2001:db8::4", "2001:db8::6", "2001:db8::7"]
        );
        assert_eq!(
            derive_ip_neighbor_candidates("not-an-ip", 2, &scope(&["192.0.2.0/24"], &[]))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert_eq!(
            derive_ip_neighbor_candidates("192.0.2.10", 2, &scope(&["192.0.2.11/32"], &[]))
                .unwrap_err()
                .code,
            ErrorCode::OutOfScope
        );
    }

    #[test]
    fn requires_explicit_scope_and_caps_expansion() {
        let empty_scope = scope(&[], &[]);
        assert_eq!(
            derive_ip_neighbor_candidates("192.0.2.10", 2, &empty_scope)
                .unwrap_err()
                .code,
            ErrorCode::OutOfScope
        );
        for bits in [0, 9, u8::MAX] {
            assert_eq!(
                derive_ip_neighbor_candidates("192.0.2.10", bits, &scope(&["192.0.2.0/24"], &[]),)
                    .unwrap_err()
                    .code,
                ErrorCode::LimitExceeded
            );
        }
        let result =
            derive_ip_neighbor_candidates("192.0.2.10", 8, &scope(&["192.0.2.0/24"], &[])).unwrap();
        assert_eq!(result.candidates.len(), MAX_EXPECTED_CANDIDATES);
        let restricted =
            derive_ip_neighbor_candidates("192.0.2.10", 8, &scope(&["192.0.2.10/32"], &[]))
                .unwrap();
        assert!(restricted.candidates.is_empty());
    }

    #[test]
    fn handles_maximum_ipv4_and_ipv6_seeds_without_integer_overflow_or_wraparound() {
        let ipv4 =
            derive_ip_neighbor_candidates("255.255.255.255", 8, &scope(&["255.255.255.0/24"], &[]))
                .unwrap();
        assert_eq!(ipv4.candidates.len(), 255);
        assert!(ipv4.candidates.contains(&"255.255.255.254".to_owned()));
        assert!(!ipv4.candidates.contains(&"0.0.0.0".to_owned()));

        let ipv6 = derive_ip_neighbor_candidates(
            "ffff:ffff:ffff:ffff:ffff:ffff:ffff:ffff",
            8,
            &scope(&["ffff:ffff:ffff:ffff:ffff:ffff:ffff:ff00/120"], &[]),
        )
        .unwrap();
        assert_eq!(ipv6.candidates.len(), 255);
        assert!(
            ipv6.candidates
                .contains(&"ffff:ffff:ffff:ffff:ffff:ffff:ffff:fffe".to_owned())
        );
        assert!(!ipv6.candidates.contains(&"::".to_owned()));
    }

    const MAX_EXPECTED_CANDIDATES: usize = 255;
}
