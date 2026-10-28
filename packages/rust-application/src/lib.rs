//! Casos de uso Rust de CHEF. Los adaptadores concretos se componen desde las apps.

mod bbot_event_rate;
mod dns;
mod dns_plan;
mod dns_plan_runner;
mod dns_wildcard;
mod import_history;

pub use bbot_event_rate::{BbotEventRateCounter, BbotEventRateError, BbotEventRateSnapshot};
pub use dns::{
    DnsAnswer, DnsAnswerDisposition, DnsAnswerEvidence, DnsAuthorizationFailure,
    DnsAuthorizationPort, DnsAuthorizationReceipt, DnsCancellation, DnsLookupOutcome,
    DnsQueryRequest, DnsRecordType, DnsResolutionError, DnsResolutionResult, DnsResolverFailure,
    DnsResolverPort, ResolveDns,
};
pub use dns_plan::{DnsQueryPlan, DnsQueryPlanSettings, DnsQueryStage, PlannedDnsQuery};
pub use dns_plan_runner::{
    DnsPlanExecution, DnsPlanQueryResult, DnsPlanTermination, ExecuteDnsPlan,
};
pub use dns_wildcard::{
    DnsWildcardAssessment, DnsWildcardBaseline, DnsWildcardDisposition, DnsWildcardPlanStatus,
    DnsWildcardProbeGroup, DnsWildcardProbePlan, DnsWildcardProbeSet, DnsWildcardRecordAssessment,
};
pub use import_history::{
    IMPORT_HISTORY_MAX_PAGE_SIZE, ImportHistoryError, ImportHistoryPort, ImportRunSummary,
    ListImportRuns,
};
