//! Rust port of CHEF's offline evidence core.
//!
//! This crate deliberately has no filesystem, network, Tauri, or database access.
//! The caller supplies bounded bytes, scope, and timestamps through explicit inputs.

mod affiliate_summary;
mod ajaxpro_response_hints;
mod anubisdb_subdomains;
mod apk_attachment_hints;
mod application;
mod asn;
mod asn_report;
mod aspnet_binary_response_hints;
mod asset_inventory_export;
mod bimi;
mod bucket_file_hints;
mod builtwith;
mod c99_subdomain_hints;
mod caa;
mod cdn_port_filter;
mod censys_dns;
mod censys_ip;
mod censys_ip_import;
mod certspotter;
mod chaos;
mod cloud_tenant_hints;
mod config;
mod config_merge;
mod container_repository_hints;
mod contracts;
mod crt_db_hints;
mod crtsh;
mod dns_cache;
mod dns_format;
mod dns_mutation_candidates;
mod dns_policy;
mod dns_records;
mod dns_service_candidates;
mod dns_word_segmentation;
mod domain;
mod domain_export;
mod dotnetnuke_response_hints;
mod email_export;
mod email_form_hints;
mod email_format;
mod event_engine;
mod event_import;
mod evidence_csv;
mod fullhunt;
mod git_exposure_hints;
mod github_code_search_hints;
mod github_profile_hints;
mod github_repository_hints;
mod github_workflow_hints;
mod gitlab_instance_project_hints;
mod gitlab_project_hints;
mod graphql_schema_hints;
mod host;
mod http_status_preference;
mod hunter_source_hints;
mod identity;
mod import_byte_size;
mod ip_geolocation_hints;
mod ip_neighbor_candidates;
mod ipstack_geolocation_hints;
mod javascript_component_hints;
mod mobile_app_hints;
mod mobile_osint_hints;
mod module_catalog;
mod module_flag_catalog;
mod myssl_subdomain_hints;
mod nmap;
mod nmap_export;
mod ntlm_identity_hints;
mod oidc_config_hints;
mod otx;
mod pgp_emails;
mod port_spec;
mod postman_collection_hints;
mod presets;
mod provider_subdomains;
mod rate_limit;
mod recon;
mod report_module_policy;
mod repository_link_hints;
mod resource_file_hints;
mod reverse_whois_hints;
mod robots_hints;
mod security_contact_hints;
mod service_fingerprint_hints;
mod shodan_dns;
mod shodan_host_hints;
mod shodan_internetdb_hints;
mod similarity_fingerprint;
mod skymem_domain_hints;
mod social_profile_hints;
mod speculate;
mod statistics;
mod subdomainradar_hints;
mod target_seed;
mod telerik_response_hints;
mod text_policy;
mod tls_certificate_hints;
mod tlsrpt;
mod trickest_subdomain_hints;
mod url_fingerprint;
mod url_policy;
mod url_tools;
mod urlscan_import;
mod utc_datetime;
mod virustotal;
mod wayback_cdx_hints;
mod web_parameter_hints;
mod web_report_export;
mod wordcloud;

pub use affiliate_summary::{AffiliateDomainHint, summarize_affiliate_domains};
pub use ajaxpro_response_hints::{ArchivedAjaxProPathHints, parse_archived_ajaxpro_html};
pub use anubisdb_subdomains::{ArchivedSubdomainCandidates, parse_archived_subdomain_candidates};
pub use apk_attachment_hints::{
    AndroidPackageFormat, ArchivedApkAttachmentHint, parse_archived_apk_attachment,
};
pub use application::{
    BbotImportResult, Clock, FixedClock, ImportBbotEvidence, ImportEvidence, SystemClock,
    parse_bbot_event_graph, validate_bbot_event_graph,
};
pub use asn::{
    AsnEnrichmentCircuitBreaker, AsnRecord, AsnTargetRetryPolicy, normalize_asn_response,
};
pub use asn_report::{AsnSummaryRow, summarize_asn_records};
pub use aspnet_binary_response_hints::{
    ArchivedAspNetBinaryResponseHint, parse_archived_aspnet_binary_response,
};
pub use asset_inventory_export::{ChefAssetInventoryExport, export_asset_inventory_csv};
pub use bucket_file_hints::{ArchivedBucketFileHint, parse_archived_bucket_file_listing};
pub use builtwith::parse_builtwith_subdomains;
pub use c99_subdomain_hints::parse_archived_c99_subdomains;
pub use cdn_port_filter::{
    CdnPortFilterAssessment, CdnPortFilterPolicy, CdnPortFilterReason, parse_cdn_port_filter_policy,
};
pub use censys_dns::{CensysDnsCertificatePage, parse_censys_dns_certificate_page};
pub use censys_ip::{
    CensysDnsNameSource, CensysIpDnsNameHint, CensysIpHostResponse, CensysIpPortHint,
    CensysIpProtocolHint, CensysIpTransport, parse_censys_ip_host_response,
};
pub use censys_ip_import::{CensysIpImportResult, import_censys_ip_snapshot};
pub use certspotter::{CertSpotterSubdomainPage, parse_certspotter_subdomains};
pub use chaos::{ChaosParentCollapse, ChaosSubdomainPage, parse_chaos_subdomain_response};
pub use cloud_tenant_hints::{ArchivedCloudTenantHints, parse_archived_cloud_tenant};
pub use config::{ChefConfig, DefectDojoSettings, RuntimeSettings, ScopeSettings, SourceSettings};
pub use config_merge::{
    deep_merge_toml, dotted_get, dotted_set, iter_dotted_paths, parse_layered_toml,
};
pub use container_repository_hints::{
    ArchivedContainerRepositories, ContainerRepositoryHint, parse_archived_container_repositories,
};
pub use contracts::{ScopePolicy, Snapshot};
pub use crt_db_hints::parse_archived_crt_db_rows;
pub use crtsh::{CrtShSubdomainPage, parse_crtsh_subdomains};
pub use dns_cache::DnsCache;
pub use dns_format::format_dns_rdata;
pub use dns_mutation_candidates::{
    DnsMutationCandidate, DnsMutationCandidateBatch, DnsMutationCandidateOptions,
    DnsMutationReason, generate_dns_mutation_candidates,
};
pub use dns_policy::{DnsQueryPolicy, DnsZoneDecision};
pub use dns_records::{DNS_RECORD_TYPES, DnsTarget, extract_dns_targets, is_dns_service_record};
pub use dns_service_candidates::common_dns_service_candidates;
pub use domain::{ChefError, ChefResult, ErrorCode, ServiceInput};
pub use domain_export::{
    ChefObservedDomain, ChefObservedDomainEvidence, ChefObservedDomainExport,
    export_observed_domain_list,
};
pub use dotnetnuke_response_hints::{
    ArchivedDotNetNukeResponseHints, DotNetNukeResponseMarker, parse_archived_dotnetnuke_response,
};
pub use email_export::{
    ChefEmailEvidence, ChefObservedEmail, ChefObservedEmailExport, export_observed_emails,
};
pub use email_form_hints::{ArchivedEmailFormHints, parse_archived_email_form_hints};
pub use email_format::parse_archived_cfemail_html;
pub use event_engine::{
    BbotEvent, BbotEventDraft, BbotEventEngine, BbotEventGraph, BbotModule, EventScopeGate,
    ModuleDescriptor,
};
pub use evidence_csv::{ChefEvidenceCsvExport, export_evidence_csv};
pub use fullhunt::{FullHuntSubdomainPage, parse_fullhunt_subdomains};
pub use git_exposure_hints::{ArchivedGitExposureHint, classify_archived_git_config};
pub use github_code_search_hints::{
    ArchivedGithubCodeSearchHints, parse_archived_github_code_search,
};
pub use github_profile_hints::{
    ArchivedGithubProfileHints, GithubProfileHint, parse_archived_github_profile_search,
};
pub use github_repository_hints::{
    ArchivedGithubRepositoryHints, GithubRepositoryHint, parse_archived_github_repositories,
};
pub use github_workflow_hints::{
    ArchivedGithubWorkflows, GithubWorkflowHint, parse_archived_github_workflows,
};
pub use gitlab_instance_project_hints::{
    ArchivedGitlabInstanceProjectHints, GitlabInstanceProjectHint,
    parse_archived_gitlab_instance_projects,
};
pub use gitlab_project_hints::{
    ArchivedGitlabProjectHints, GitlabProjectHint, parse_archived_gitlab_projects,
};
pub use graphql_schema_hints::{ArchivedGraphqlSchemaHint, summarize_archived_graphql_schema};
pub use host::dns_parent_zones;
pub use http_status_preference::select_preferred_http_status;
pub use hunter_source_hints::{ArchivedHunterSourceHints, parse_archived_hunter_sources};
pub use identity::{digest as digest_bytes, id as deterministic_id};
pub use import_byte_size::{format_import_byte_limit, parse_import_byte_limit};
pub use ip_geolocation_hints::{ArchivedIpGeolocationHint, parse_archived_ip_geolocation};
pub use ip_neighbor_candidates::{IpNeighborCandidateSet, derive_ip_neighbor_candidates};
pub use ipstack_geolocation_hints::parse_archived_ipstack_geolocation;
pub use javascript_component_hints::{
    ArchivedJavaScriptInventory, JavaScriptComponentHint, parse_archived_javascript_inventory,
};
pub use mobile_app_hints::{ArchivedMobileAppHints, parse_archived_mobile_app_ids};
pub use mobile_osint_hints::{ArchivedMobileOsintHints, parse_archived_mobile_osint_response};
pub use module_catalog::{
    ModuleCapability, ModuleCapabilityKind, ModuleCapabilityState, module_capabilities,
    module_capability,
};
pub use module_flag_catalog::describe_module_flag;
pub use myssl_subdomain_hints::{ArchivedMysslSubdomainHints, parse_archived_myssl_subdomains};
pub use nmap_export::{ChefNmapEvidenceExport, export_nmap_evidence_xml};
pub use ntlm_identity_hints::{ArchivedNtlmIdentityHints, parse_archived_ntlm_challenge};
pub use oidc_config_hints::{ArchivedOidcEndpointHint, parse_archived_oidc_config};
pub use otx::{OtxPassiveDnsPage, parse_otx_passive_dns};
pub use pgp_emails::parse_archived_pgp_emails;
pub use port_spec::parse_port_string;
pub use postman_collection_hints::{ArchivedPostmanUrlHints, parse_archived_postman_collection};
pub use presets::{ChefPreset, PresetCatalog, ResolvedPreset};
pub use provider_subdomains::{
    BufferOverrunSubdomainPage, DnsDumpsterSubdomainPage, HackerTargetSubdomainPage,
    LeakIxSubdomainPage, ProviderSubdomainPage, RapidDnsSubdomainPage, SecurityTrailsSubdomainPage,
    SubdomainCenterPage, UrlScanSubdomainPage, parse_bufferoverrun_subdomains,
    parse_dnsdumpster_subdomains, parse_hackertarget_subdomains, parse_leakix_subdomains,
    parse_provider_subdomains, parse_rapiddns_subdomains, parse_securitytrails_subdomains,
    parse_subdomaincenter_subdomains, parse_urlscan_subdomains,
};
pub use rate_limit::RateLimiter;
pub use recon::{
    ChefReconGraph, ReconAsset, ReconAssetKind, ReconEvidence, ReconRelationship,
    ReconRelationshipKind, adapt_bbot_to_chef, validate_chef_recon_graph,
};
pub use report_module_policy::filter_report_module_statistics;
pub use repository_link_hints::{
    ArchivedRepositoryLinkHints, RepositoryLinkHint, RepositoryPlatform,
    parse_archived_repository_links,
};
pub use resource_file_hints::{
    ArchivedResourceFileHint, FileExtensionSource, classify_archived_resource,
};
pub use reverse_whois_hints::{ArchivedReverseWhoisHints, parse_archived_reverse_whois};
pub use robots_hints::{
    ArchivedRobotsHints, RobotsHintKind, RobotsPathHint, parse_archived_robots_hints,
};
pub use security_contact_hints::{ArchivedSecurityContactHints, parse_archived_security_contacts};
pub use service_fingerprint_hints::{
    ArchivedServiceFingerprintHints, ServiceFingerprintHint, parse_archived_service_fingerprints,
};
pub use shodan_dns::{ShodanDnsSubdomainPage, parse_shodan_dns_subdomains};
pub use shodan_host_hints::{
    ArchivedShodanHostHints, ShodanPortHint, ShodanTechnologyHint, parse_archived_shodan_host,
};
pub use shodan_internetdb_hints::{
    ArchivedShodanInternetDbHints, ShodanInternetDbPortHint, parse_archived_shodan_internetdb,
};
pub use similarity_fingerprint::{compute_similarity_fingerprint, similarity_fingerprint_score};
pub use skymem_domain_hints::{ArchivedSkymemDomainHints, parse_archived_skymem_domains};
pub use social_profile_hints::{
    ArchivedSocialProfileHint, SocialPlatform, parse_archived_social_profile,
};
pub use statistics::{
    BbotEventStatistics, BbotRuntimeStatistics, EventTypeCount, ModuleProducedCount,
    ModuleRuntimeStatistics, summarize_bbot_events,
};
pub use subdomainradar_hints::{ArchivedSubdomainRadarPage, parse_archived_subdomainradar_page};
pub use target_seed::{ChefTargetKind, ChefTargetSeed, parse_chef_target_seed};
pub use telerik_response_hints::{
    ArchivedTelerikResponseHints, TelerikMarkerCount, TelerikResponseMarker,
    parse_archived_telerik_response,
};
pub use text_policy::{escape_terminal_controls, is_printable_text, truncate_text};
pub use tls_certificate_hints::{
    ArchivedTlsCertificateHints, parse_archived_tls_certificate_hints,
};
pub use trickest_subdomain_hints::{
    ArchivedTrickestSubdomains, parse_archived_trickest_subdomains,
};
pub use url_fingerprint::url_shape_fingerprint;
pub use url_policy::{canonical_fqdn_or_ip, url_path_depth};
pub use urlscan_import::{UrlScanImportResult, import_urlscan_snapshot};
pub use utc_datetime::normalize_utc_datetime;
pub use virustotal::{VirusTotalSubdomainPage, parse_virustotal_subdomain_page};
pub use wayback_cdx_hints::{ArchivedWaybackCdxHints, parse_archived_wayback_cdx};
pub use web_parameter_hints::{ArchivedWebParameterHints, parse_archived_web_parameters};
pub use web_report_export::{ChefWebEvidenceReportExport, export_web_evidence_report};

#[cfg(test)]
mod tests;
