//! Adaptadores de infraestructura Rust para casos de uso CHEF.

mod dns_resolver;
mod dns_wildcard_labels;
mod http_authorization;
mod http_transport;

pub use dns_resolver::{DnsResolverConfig, DnsResolverConfigError, HickoryDnsResolver};
pub use dns_wildcard_labels::{DnsWildcardLabelGenerationError, SystemDnsWildcardSampleLabels};
pub use http_authorization::{
    HttpAuthorizationGrant, HttpAuthorizationRegistry, HttpAuthorizationRegistryError,
};
pub use http_transport::ReqwestHttpTransport;
