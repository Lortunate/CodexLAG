use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::logs::{
    usage_request_detail_from_runtime, usage_request_history_from_runtime,
};
use codexlag_lib::providers::invocation::InvocationFailureClass;
use codexlag_lib::routing::policy::RoutingMode;
use codexlag_lib::secret_store::SecretKey;
use rusqlite::Connection;
use tower::ServiceExt;

#[tokio::test]
async fn runtime_gateway_requests_are_persisted_to_request_logs_and_attempt_logs() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::Hybrid)
        .expect("switch mode");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Http5xx);

    let history = usage_request_history_from_runtime(&runtime, Some(10));
    assert!(history.is_empty(), "fresh runtime should start empty");

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("gateway request"),
        )
        .await
        .expect("gateway response");

    assert_eq!(response.status(), StatusCode::OK);

    let history = usage_request_history_from_runtime(&runtime, Some(10));
    assert_eq!(history.len(), 1);
    let request_id = history[0].request_id.clone();
    let detail = usage_request_detail_from_runtime(&runtime, request_id.as_str())
        .expect("persisted request detail");
    assert_eq!(detail.endpoint_id, "relay-newapi");
    assert_eq!(detail.final_upstream_status, Some(200));

    let database_path = runtime
        .runtime_log()
        .log_dir
        .parent()
        .expect("runtime log parent")
        .join("codexlag.sqlite3");
    let sqlite = Connection::open(database_path).expect("open sqlite");
    let request_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_logs", [], |row| row.get(0))
        .expect("count request rows");
    let attempt_rows: i64 = sqlite
        .query_row("SELECT COUNT(*) FROM request_attempt_logs", [], |row| {
            row.get(0)
        })
        .expect("count attempt rows");
    assert_eq!(request_rows, 1);
    assert_eq!(attempt_rows, 2);

    let attempts: Vec<(String, i64, String)> = {
        let mut statement = sqlite
            .prepare(
                "
                SELECT attempt_id, attempt_index, endpoint_id
                FROM request_attempt_logs
                WHERE request_id = ?1
                ORDER BY attempt_index ASC
                ",
            )
            .expect("prepare attempt query");
        statement
            .query_map([request_id.as_str()], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .expect("query attempt rows")
            .map(|row| row.expect("decode attempt row"))
            .collect()
    };
    assert_eq!(attempts.len(), 2);
    assert_eq!(
        attempts[0],
        (format!("{request_id}:0"), 0, "official-primary".to_string())
    );
    assert_eq!(
        attempts[1],
        (format!("{request_id}:1"), 1, "relay-newapi".to_string())
    );

    let timing: (i64, i64, i64) = sqlite
        .query_row(
            "
            SELECT started_at_ms, finished_at_ms, latency_ms
            FROM request_logs
            WHERE request_id = ?1
            ",
            [request_id.as_str()],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("load request timing");
    assert!(
        timing.1 > timing.0,
        "persisted request timing should reflect a real request duration"
    );
    assert!(
        timing.2 > 0,
        "persisted request latency should be derived from the runtime execution path"
    );

    let persisted_attempt_statuses: Vec<(i64, Option<i64>)> = {
        let mut statement = sqlite
            .prepare(
                "
                SELECT attempt_index, upstream_status
                FROM request_attempt_logs
                WHERE request_id = ?1
                ORDER BY attempt_index ASC
                ",
            )
            .expect("prepare upstream status query");
        statement
            .query_map([request_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?)))
            .expect("query upstream status rows")
            .map(|row| row.expect("decode attempt status row"))
            .collect()
    };
    assert_eq!(persisted_attempt_statuses.len(), 2);
    assert_eq!(
        persisted_attempt_statuses[0],
        (0, Some(503)),
        "failed attempts should persist their upstream status instead of only the final attempt"
    );
}
