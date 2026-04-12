use std::path::{Path, PathBuf};

use codexlag_lib::bootstrap::{bootstrap_runtime_for_test, runtime_database_path, runtime_log_dir};
use codexlag_lib::logging::runtime::{format_event_fields, redact_secret_value};

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

#[test]
fn redact_secret_value_masks_sensitive_tokens() {
    assert_eq!(redact_secret_value(""), "");
    assert_eq!(redact_secret_value("abcd"), "****");
    assert_eq!(
        redact_secret_value("ck_local_1234567890"),
        "ck_l***************"
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
