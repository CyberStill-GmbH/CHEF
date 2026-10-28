//! Casos de uso Rust de CHEF. Los adaptadores concretos se componen desde las apps.

mod dns;
mod dns_plan;
mod dns_plan_runner;
mod dns_service_plan;
mod dns_service_plan_runner;
mod dns_wildcard;
mod event_rate;
mod http_execution;
mod http_plan;
mod import_history;
mod import_jobs;
mod keyed_lock;

pub use dns::{
    DnsAnswer, DnsAnswerDisposition, DnsAnswerEvidence, DnsAuthorizationControl,
    DnsAuthorizationFailure, DnsAuthorizationPort, DnsAuthorizationReceipt, DnsCancellation,
    DnsLookupOutcome, DnsQueryRequest, DnsRecordType, DnsResolutionError, DnsResolutionResult,
    DnsResolverFailure, DnsResolverPort, ResolveDns,
};
pub use dns_plan::{DnsQueryPlan, DnsQueryPlanSettings, DnsQueryStage, PlannedDnsQuery};
pub use dns_plan_runner::{
    DnsPlanExecution, DnsPlanQueryResult, DnsPlanTermination, ExecuteDnsPlan,
};
pub use dns_service_plan::{DnsServiceQueryCandidate, DnsServiceQueryPlan};
pub use dns_service_plan_runner::{
    DnsServicePlanExecution, DnsServicePlanTermination, DnsServiceQueryResult,
    ExecuteDnsServicePlan,
};
pub use dns_wildcard::{
    DnsWildcardAssessment, DnsWildcardBaseline, DnsWildcardDisposition, DnsWildcardPlanExecution,
    DnsWildcardPlanStatus, DnsWildcardProbeExecution, DnsWildcardProbeGroup, DnsWildcardProbePlan,
    DnsWildcardProbeSet, DnsWildcardRecordAssessment, ExecuteDnsWildcardPlan,
};
pub use event_rate::{ImportEventRateCounter, ImportEventRateError, ImportEventRateSnapshot};
pub use http_execution::{
    ExecuteHttpPlan, HttpAuthorizationControl, HttpAuthorizationFailure, HttpAuthorizationPort,
    HttpAuthorizationReceipt, HttpAuthorizationUse, HttpCancellation, HttpExecutionError,
    HttpExecutionGate, HttpExecutionTermination, HttpPlanExecution, HttpRequestOutcome,
    HttpRequestResult, HttpTransportFailure, HttpTransportPort, HttpTransportRequest,
    HttpTransportResponse,
};
pub use http_plan::{HttpPlanError, HttpPlanSettings, HttpRequestPlan, PlannedHttpRequest};
pub use import_history::{
    IMPORT_HISTORY_MAX_PAGE_SIZE, ImportHistoryError, ImportHistoryPort, ImportRunSummary,
    ListImportRuns,
};
pub use import_jobs::{
    ImportCancellation, ImportFormat, ImportJobEvent, ImportOutput, ImportTask,
    spawn_bbot_import_job, spawn_bbot_import_job_with_config, spawn_import_job,
};
pub use keyed_lock::{KeyedLock, KeyedLockGuard};

#[cfg(test)]
mod tests;
