use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    ChefError, ErrorCode, FixedClock, ImportBbotEvidence, ImportEvidence, ScopePolicy,
    parse_securitytrails_subdomains, parse_subdomaincenter_subdomains,
};

const NMAP: &[u8] = include_bytes!("../../../fixtures/nmap/lab.xml");
const SCOPE: &str = include_str!("../../../fixtures/scope/lab.json");

fn policy() -> ScopePolicy {
    serde_json::from_str(SCOPE).expect("fixture scope is valid")
}

fn import(bytes: &[u8], policy: &ScopePolicy) -> Result<crate::Snapshot, ChefError> {
    ImportEvidence::execute(
        bytes,
        policy,
        &FixedClock("2026-09-29T00:00:00.000Z".to_owned()),
        None,
    )
}

#[test]
fn fixture_import_preserves_evidence_and_negative_controls() {
    let snapshot = import(NMAP, &policy()).expect("fixture imports");
    assert_eq!(snapshot.schema_version, "1.0.0");
    assert_eq!(snapshot.observations.len(), 3);
    assert_eq!(snapshot.assets.len(), 4);
    assert_eq!(snapshot.relationships.len(), 2);
    assert_eq!(snapshot.paths.len(), 2);
    assert!(
        snapshot
            .paths
            .iter()
            .all(|path| !path.is_confirmed_vulnerability)
    );
    assert!(
        snapshot
            .relationships
            .iter()
            .any(|relation| relation.observation_ids.len() == 2)
    );
    assert!(
        snapshot
            .assets
            .iter()
            .all(|asset| !asset.observation_ids.is_empty())
    );
}

#[test]
fn import_is_deterministic_and_scope_is_normalized_before_hashing() {
    let policy = policy();
    let first = import(NMAP, &policy).expect("fixture imports");
    let second = import(NMAP, &policy).expect("fixture imports twice");
    assert_eq!(first, second);
    assert_eq!(
        first.scope_policy.allowed_addresses,
        ["127.0.0.1", "127.0.0.2", "::1"]
    );
    assert_eq!(first.scope_policy.allowed_ports, [22, 80, 443, 8080]);
    assert_eq!(
        first
            .observations
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        [
            "observation:7159742ae182053416a4c963af4e74a08163a659ddd0d5f5bdc725de67a70a24",
            "observation:8de568b54a3ac9835d3e70a9501c6e04310213268642b26cb7e3fcb605908d46",
            "observation:f680b1959834483683e62871bd3793c1b9c885b27e862de838caddbaf036abf6",
        ]
    );
    assert_eq!(
        first.run.id,
        "run:39805e6cacad839063818cc0a3cf7f8b27855ae4ca62162c919f258beb026bdb"
    );
    assert_eq!(
        first.evidence[0].id,
        "evidence:3e88e4c3523b65df50b3438307d5186b8be15315dea5972cf5fd485a15bd59bb"
    );
    assert_eq!(
        first
            .relationships
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        [
            "relationship:5f62147f50dc4abe5261d9ad87580d02cebb31744effb7df0a920948ff834fbb",
            "relationship:98bf0c9e52192bb2d8e774c34c307bf9b3ae09fc4a3d14325db317d0a09cb759",
        ]
    );
    assert_eq!(
        first
            .paths
            .iter()
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        [
            "path:4760c9dd67ff0aaa7b4423fbf709077fe8ed958519060b151d267e3e02952d6c",
            "path:9b12d524581a29f22c6f9293656345eca768a6eb31a49e507e913e0e454e9568",
        ]
    );
}

#[test]
fn hostile_xml_malformed_input_and_out_of_scope_fail_closed() {
    let hostile = include_bytes!("../../../fixtures/nmap/hostile-xxe.xml");
    assert_eq!(
        import(hostile, &policy()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        import(b"<nmaprun>", &policy()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        import(&[0xff], &policy()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    let nested = format!(
        "<nmaprun scanner=\"nmap\" start=\"1790640000\">{}<host/></nmaprun>",
        "<x>".repeat(33) + &"</x>".repeat(33)
    );
    assert_eq!(
        import(nested.as_bytes(), &policy()).unwrap_err().code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        import(
            br#"<nmaprun scanner="nmap" start="1790640000"><host><status state="up"/><address addr="192.0.2.1" addrtype="ipv4"/><ports><port protocol="tcp" portid="80"><state state="open"/></port></ports></host></nmaprun>"#,
            &policy(),
        )
        .unwrap_err()
        .code,
        ErrorCode::OutOfScope
    );
}

#[test]
fn limits_and_cancellation_are_enforced_before_publication() {
    let mut limited = policy();
    limited.max_input_bytes = 1;
    assert_eq!(
        import(NMAP, &limited).unwrap_err().code,
        ErrorCode::LimitExceeded
    );

    let cancelled = AtomicBool::new(true);
    let result = ImportEvidence::execute(
        NMAP,
        &policy(),
        &FixedClock("2026-09-29T00:00:00.000Z".to_owned()),
        Some(&cancelled),
    );
    assert_eq!(result.unwrap_err().code, ErrorCode::Cancelled);
    cancelled.store(false, Ordering::SeqCst);
}

#[test]
fn emitted_snapshot_has_only_the_v1_wire_fields() {
    let snapshot = import(NMAP, &policy()).expect("fixture imports");
    let value = serde_json::to_value(snapshot).expect("snapshot serializes");
    assert_eq!(value["run"]["state"], "completed");
    assert_eq!(value["evidence"][0]["source"], "nmap-xml");
    assert_eq!(value["paths"][0]["isConfirmedVulnerability"], false);
    assert!(value["evidence"][0].get("filesystemPath").is_none());
}

#[test]
fn bbot_jsonl_import_is_deterministic_scoped_and_keeps_source_provenance() {
    let bytes = include_bytes!("../../../fixtures/event-import/events.ndjson");
    let mut scope = policy();
    scope.allowed_ports.push(53);
    let first = ImportBbotEvidence::execute(
        bytes,
        &scope,
        &FixedClock("2026-10-18T00:00:00.000Z".to_owned()),
        None,
    )
    .expect("synthetic BBOT export imports");
    let second = ImportBbotEvidence::execute(
        bytes,
        &scope,
        &FixedClock("2026-10-18T00:00:00.000Z".to_owned()),
        None,
    )
    .expect("same export imports deterministically");

    assert_eq!(first, second);
    assert_eq!(first.snapshot.schema_version, "1.1.0");
    assert_eq!(first.snapshot.evidence[0].source, "bbot-ndjson");
    assert_eq!(first.snapshot.observations.len(), 2);
    assert!(
        first
            .snapshot
            .observations
            .iter()
            .all(|item| item.source == "bbot-ndjson" && item.fact == "open-port")
    );
    assert!(!first.skipped_event_types.contains_key("DNS_NAME"));
    assert!(
        first
            .snapshot
            .paths
            .iter()
            .all(|path| !path.is_confirmed_vulnerability)
    );
}

#[test]
fn bbot_jsonl_rejects_malformed_and_out_of_scope_open_port_events() {
    let scope = policy();
    let malformed = ImportBbotEvidence::execute(
        b"{invalid json}\n",
        &scope,
        &FixedClock("2026-10-18T00:00:00.000Z".to_owned()),
        None,
    )
    .unwrap_err();
    assert_eq!(malformed.code, ErrorCode::InvalidInput);

    let outside = br#"{"type":"OPEN_TCP_PORT","data":"192.0.2.10:443","host":"192.0.2.10","port":443,"timestamp":1792281600}"#;
    let error = ImportBbotEvidence::execute(
        outside,
        &scope,
        &FixedClock("2026-10-18T00:00:00.000Z".to_owned()),
        None,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::OutOfScope);
}

#[test]
fn securitytrails_archived_prefixes_become_sorted_scoped_candidates() {
    let page = parse_securitytrails_subdomains(
        "Example.COM.",
        br#"{"subdomains":["www","api","www","*.ignored","outside.example.net","bad..label"]}"#,
    )
    .expect("valid archived SecurityTrails response");

    assert_eq!(page.query_domain, "example.com");
    assert_eq!(
        page.candidates
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            "api.example.com",
            "outside.example.net.example.com",
            "www.example.com"
        ]
    );
}

#[test]
fn subdomaincenter_archived_names_filter_external_and_invalid_candidates() {
    let page = parse_subdomaincenter_subdomains(
        "example.com",
        br#"["WWW.EXAMPLE.COM.","api.example.com","example.com","evil-example.com","*.wild.example.com",42]"#,
    )
    .expect("valid archived Subdomain Center response");

    assert_eq!(
        page.candidates
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["api.example.com", "www.example.com"]
    );
}

#[test]
fn archived_subdomain_parsers_reject_bad_shapes_roots_and_entry_limits() {
    assert_eq!(
        parse_securitytrails_subdomains("example.com", br#"{"subdomains":"www"}"#)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    assert_eq!(
        parse_subdomaincenter_subdomains("com", b"[]")
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );

    let oversized = format!("{{\"subdomains\":[{}]}}", vec!["\"x\""; 2_049].join(","));
    assert_eq!(
        parse_securitytrails_subdomains("example.com", oversized.as_bytes())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
}
