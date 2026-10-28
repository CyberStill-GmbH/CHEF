use chef_core::{ErrorCode, ScopePolicy};

use crate::{ImportFormat, ImportJobEvent, spawn_import_job};

fn policy() -> ScopePolicy {
    serde_json::from_str(include_str!("../../../fixtures/scope/lab.json"))
        .expect("fixture scope is valid")
}

#[tokio::test]
async fn background_bbot_job_emits_bounded_lifecycle_and_returns_snapshot() {
    let mut scope = policy();
    scope.allowed_ports.push(53);
    let mut task = spawn_import_job(
        ImportFormat::BbotJsonl,
        include_bytes!("../../../fixtures/event-import/events.ndjson").to_vec(),
        scope,
        "2026-10-18T00:00:00.000Z".to_owned(),
    )
    .await
    .expect("job starts on Tokio runtime");

    assert!(matches!(
        task.next_event().await,
        Some(ImportJobEvent::Started { .. })
    ));
    assert!(matches!(
        task.next_event().await,
        Some(ImportJobEvent::Completed {
            observation_count: 2,
            ..
        })
    ));
    let output = task.join().await.expect("worker completes");
    assert_eq!(output.snapshot.schema_version, "1.1.0");
    assert_eq!(output.snapshot.observations.len(), 2);
    assert!(!output.skipped_event_types.contains_key("DNS_NAME"));
}

#[tokio::test]
async fn background_job_reports_scope_failure_without_publishing_partial_snapshot() {
    let task = spawn_import_job(
        ImportFormat::BbotJsonl,
        include_bytes!("../../../fixtures/event-import/events.ndjson").to_vec(),
        policy(),
        "2026-10-18T00:00:00.000Z".to_owned(),
    )
    .await
    .expect("job starts on Tokio runtime");
    let mut task = task;
    assert!(matches!(
        task.next_event().await,
        Some(ImportJobEvent::Started { .. })
    ));
    assert!(matches!(
        task.next_event().await,
        Some(ImportJobEvent::Failed {
            code: ErrorCode::OutOfScope,
            ..
        })
    ));
    assert_eq!(task.join().await.unwrap_err().code, ErrorCode::OutOfScope);
}
