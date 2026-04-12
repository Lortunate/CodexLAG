use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test,
    secret_store::SecretKey,
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn gateway_falls_back_to_relay_after_official_server_error_and_keeps_correlation() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .loopback_gateway()
        .state()
        .enable_test_route_headers_for_test();

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .header("x-codexlag-endpoint-status", "official-default:503")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    assert_eq!(payload["endpoint_id"], "relay-default");

    let debug = runtime
        .loopback_gateway()
        .state()
        .last_route_debug()
        .expect("route debug");
    assert_eq!(debug.attempt_count, 2);
    assert_eq!(debug.selected_endpoint_id, "relay-default");
    assert!(debug.request_id.contains(":unrouted:"));
    let usage_records = runtime.loopback_gateway().state().usage_records();
    assert_eq!(usage_records.len(), 1);
    assert_eq!(usage_records[0].request_id, debug.request_id);
    assert_eq!(usage_records[0].endpoint_id, "relay-default");
}

#[tokio::test]
async fn no_available_endpoint_returns_structured_error_with_attempt_context() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .loopback_gateway()
        .state()
        .enable_test_route_headers_for_test();

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {secret}"))
                .header(
                    "x-codexlag-endpoint-status",
                    "official-default:503,relay-default:503",
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    assert_eq!(payload["error"], "no_available_endpoint");
    assert_eq!(payload["mode"], "hybrid");
    assert_eq!(payload["attempt_count"], 2);
    let public_request_id = payload["request_id"].as_str().expect("request id");
    assert!(public_request_id.starts_with("req_"));
    assert!(!public_request_id.contains(":unrouted:"));

    let debug = runtime
        .loopback_gateway()
        .state()
        .last_route_debug()
        .expect("route debug");
    assert_eq!(debug.attempt_count, 2);
    assert_eq!(debug.selected_endpoint_id, "relay-default");
}
