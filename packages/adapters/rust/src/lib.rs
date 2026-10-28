//! Adaptadores de infraestructura Rust para casos de uso CHEF.

mod dns_resolver;

pub use dns_resolver::{DnsResolverConfig, DnsResolverConfigError, HickoryDnsResolver};
