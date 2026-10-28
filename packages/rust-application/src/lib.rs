//! Casos de uso Rust de CHEF. Los adaptadores concretos se componen desde las apps.

mod dns;

pub use dns::{
    DnsAnswer, DnsAnswerDisposition, DnsAnswerEvidence, DnsAuthorizationFailure,
    DnsAuthorizationPort, DnsAuthorizationReceipt, DnsCancellation, DnsLookupOutcome,
    DnsQueryRequest, DnsRecordType, DnsResolutionError, DnsResolutionResult, DnsResolverFailure,
    DnsResolverPort, ResolveDns,
};
