use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::providers::invocation::InvocationFailureClass;
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test,
    routing::engine::choose_endpoint,
    secret_store::SecretKey,
    state::RuntimeState,
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn gateway_falls_back_to_relay_after_official_server_error_and_keeps_correlation() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let official_id = selected_endpoint_id(&runtime, "account_only");
    let relay_id = selected_endpoint_id(&runtime, "relay_only");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test(official_id.as_str(), InvocationFailureClass::Http5xx);

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
    assert_eq!(payload["endpoint_id"], relay_id);

    let debug = runtime
        .loopback_gateway()
        .state()
        .last_route_debug()
        .expect("route debug");
    assert_eq!(debug.attempt_count, 2);
    assert_eq!(debug.selected_endpoint_id, relay_id);
    assert!(debug.request_id.contains(":unrouted:"));
    let usage_records = runtime.loopback_gateway().state().usage_records();
    assert_eq!(usage_records.len(), 1);
    assert_eq!(usage_records[0].request_id, debug.request_id);
    assert_eq!(usage_records[0].endpoint_id, relay_id);
}

#[tokio::test]
async fn no_available_endpoint_returns_structured_error_with_attempt_context() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let official_id = selected_endpoint_id(&runtime, "account_only");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test(official_id.as_str(), InvocationFailureClass::Http5xx);
    for fallback_relay_id in relay_candidate_ids(&runtime) {
        runtime.loopback_gateway().state().plan_provider_failure_for_test(
            fallback_relay_id.as_str(),
            InvocationFailureClass::Timeout,
        );
    }

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
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    assert_eq!(payload["category"], "UpstreamError");
    assert_eq!(payload["error"], "upstream.provider_timeout");
    let public_request_id = payload["request_id"].as_str().expect("request id");
    assert!(public_request_id.starts_with("req_"));
    assert!(!public_request_id.contains(":unrouted:"));
    let attempts = runtime.loopback_gateway().state().invocation_attempts_for_test();
    let last_attempt = attempts.last().expect("last invocation attempt");
    assert_eq!(payload["attempt_count"], attempts.len());

    let debug = runtime
        .loopback_gateway()
        .state()
        .last_route_debug()
        .expect("route debug");
    assert_eq!(debug.attempt_count, attempts.len());
    assert_eq!(debug.selected_endpoint_id, last_attempt.endpoint_id);
}

fn selected_endpoint_id(runtime: &RuntimeState, mode: &str) -> String {
    choose_endpoint(mode, &runtime.loopback_gateway().state().current_candidates())
        .expect("endpoint selected for mode")
        .id
}

fn relay_candidate_ids(runtime: &RuntimeState) -> Vec<String> {
    runtime
        .loopback_gateway()
        .state()
        .current_candidates()
        .into_iter()
        .filter(|candidate| candidate.pool == codexlag_lib::routing::engine::PoolKind::Relay)
        .map(|candidate| candidate.id)
        .collect()
}
