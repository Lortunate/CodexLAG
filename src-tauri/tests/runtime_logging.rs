use std::path::{Path, PathBuf};

use codexlag_lib::bootstrap::{bootstrap_runtime_for_test, runtime_database_path, runtime_log_dir};
use codexlag_lib::commands::logs::runtime_log_metadata_from_runtime;
use codexlag_lib::logging::runtime::{
    build_attempt_id, format_event_fields, format_runtime_event_fields, redact_secret_value,
};

#[test]
fn runtime_log_dir_uses_app_local_data_logs_subfolder() {
    let app_local_data_dir = Path::new("/tmp/codexlag-app");

    assert_eq!(
        runtime_log_dir(app_local_data_dir),
        PathBuf::from("/tmp/codexlag-app").join("logs")
    );
    assert_eq!(
        runtime_database_path(app_local_data_dir),
        PathBuf::from("/tmp/codexlag-app").join("codexlag.sqlite3")
    );
}

#[tokio::test]
async fn bootstrapped_runtime_exposes_runtime_log_dir_metadata() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let log_dir = &runtime.runtime_log().log_dir;

    assert!(!log_dir.as_os_str().is_empty());
    assert!(log_dir.ends_with("logs"));
}

#[tokio::test]
async fn runtime_log_metadata_includes_name_path_size_and_mtime_fields() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let log_dir = runtime.runtime_log().log_dir.clone();
    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    std::fs::write(log_dir.join("runtime-contract.log"), "entry").expect("write runtime log file");

    let metadata = runtime_log_metadata_from_runtime(&runtime).expect("runtime log metadata");
    let file = metadata
        .files
        .iter()
        .find(|candidate| candidate.name == "runtime-contract.log")
        .expect("runtime-contract.log metadata");

    assert_eq!(metadata.log_dir, "<app-local-data>/logs");
    assert_eq!(file.path, "<app-local-data>/logs/runtime-contract.log");
    assert!(file.size > 0);
    assert!(file.mtime > 0);
}

#[test]
fn redact_secret_value_masks_sensitive_tokens() {
    assert_eq!(redact_secret_value(""), "");
    assert_eq!(redact_secret_value("abcd"), "****");
    assert_eq!(
        redact_secret_value("ck_local_1234567890"),
        "ck_l****************"
    );
}

#[test]
fn format_event_fields_outputs_stable_key_value_pairs() {
    let formatted = format_event_fields(&[
        ("event", "routing.endpoint.downgraded"),
        ("request_id", "req-123"),
        ("endpoint_id", "ep-1"),
    ]);

    assert_eq!(
        formatted,
        "event=routing.endpoint.downgraded request_id=req-123 endpoint_id=ep-1"
    );
    assert!(formatted.contains("event="));
    assert!(formatted.contains("request_id="));
    assert!(formatted.contains("endpoint_id="));
}

#[test]
fn format_event_fields_escapes_unsafe_values_and_stays_single_line() {
    let formatted = format_event_fields(&[
        ("event", "routing.endpoint.rejected"),
        ("request_id", "req 123"),
        ("error", "invalid=mode"),
        ("detail", "line1\nline2"),
    ]);

    assert_eq!(
        formatted,
        "event=routing.endpoint.rejected request_id=\"req 123\" error=\"invalid=mode\" detail=\"line1\\nline2\""
    );
    assert!(!formatted.contains('\n'));
}

#[test]
fn format_runtime_event_fields_sets_required_schema_defaults() {
    let formatted = format_runtime_event_fields(
        "gateway",
        "gateway.request.accepted",
        "req-100",
        None,
        None,
        None,
        None,
        &[("mode", "hybrid")],
    );

    assert!(formatted.contains("component=gateway"));
    assert!(formatted.contains("event=gateway.request.accepted"));
    assert!(formatted.contains("request_id=req-100"));
    assert!(formatted.contains("attempt_id=none"));
    assert!(formatted.contains("endpoint_id=none"));
    assert!(formatted.contains("latency_ms=none"));
    assert!(formatted.contains("error_code=none"));
    assert!(formatted.contains("mode=hybrid"));
}

#[test]
fn build_attempt_id_uses_request_id_and_zero_based_index() {
    assert_eq!(build_attempt_id("req-abc", 0), "req-abc:0");
    assert_eq!(build_attempt_id("req-abc", 2), "req-abc:2");
}
