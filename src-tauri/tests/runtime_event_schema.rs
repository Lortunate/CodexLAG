use codexlag_lib::logging::runtime::{format_event_fields, format_runtime_event_fields};

#[test]
fn runtime_event_schema_includes_required_fields() {
    let line = format_runtime_event_fields(
        "routing",
        "routing.endpoint.selected",
        "req-123",
        Some("req-123:0"),
        Some("relay-1"),
        Some(42),
        Some("none"),
        &[("mode", "hybrid")],
    );

    for required_key in [
        "component",
        "event",
        "request_id",
        "attempt_id",
        "endpoint_id",
        "latency_ms",
        "error_code",
    ] {
        assert!(
            line.contains(&format!("{required_key}=")),
            "missing required key: {required_key}; line={line}"
        );
    }

    assert!(line.contains("component=routing"));
    assert!(line.contains("event=routing.endpoint.selected"));
    assert!(line.contains("request_id=req-123"));
    assert!(line.contains("attempt_id=req-123:0"));
    assert!(line.contains("endpoint_id=relay-1"));
    assert!(line.contains("latency_ms=42"));
    assert!(line.contains("error_code=none"));
}

#[test]
fn runtime_event_schema_defaults_optional_values_to_none() {
    let line = format_runtime_event_fields(
        "gateway",
        "gateway.request.accepted",
        "req-789",
        None,
        None,
        None,
        None,
        &[],
    );

    assert!(line.contains("attempt_id=none"));
    assert!(line.contains("endpoint_id=none"));
    assert!(line.contains("latency_ms=none"));
    assert!(line.contains("error_code=none"));
}

#[test]
fn runtime_event_redaction_masks_bearer_tokens_api_keys_and_session_queries() {
    let line = format_event_fields(&[
        ("authorization", "Bearer top-secret-token"),
        (
            "target",
            "https://localhost:8787/v1/chat?api_key=demo-key&session_token=abc123",
        ),
        ("local_key", "ck_local_abc123xyz"),
    ]);

    assert!(!line.contains("top-secret-token"));
    assert!(!line.contains("demo-key"));
    assert!(!line.contains("abc123"));
    assert!(!line.contains("ck_local_abc123xyz"));
    assert!(line.contains("bearer [redacted]"));
    assert!(line.contains("api_key=[redacted]"));
    assert!(line.contains("session_token=[redacted]"));
    assert!(line.contains("ck_local_[redacted]"));
}
