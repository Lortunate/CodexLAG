use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test, providers::invocation::InvocationFailureClass,
    routing::policy::RoutingMode, secret_store::SecretKey, state::RuntimeState,
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn gateway_auth_failure_maps_to_credential_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-default", InvocationFailureClass::Auth);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_contract(
        &payload,
        "CredentialError",
        "credential.provider_auth_failed",
    );
}

#[tokio::test]
async fn gateway_quota_failure_maps_to_quota_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-default", InvocationFailureClass::Http429);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_contract(&payload, "QuotaError", "quota.provider_rate_limited");
}

#[tokio::test]
async fn gateway_timeout_failure_maps_to_upstream_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-default", InvocationFailureClass::Timeout);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_contract(&payload, "UpstreamError", "upstream.provider_timeout");
}

#[tokio::test]
async fn gateway_config_failure_maps_to_config_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-default", InvocationFailureClass::Config);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_contract(&payload, "ConfigError", "config.provider_rejected_request");
}

#[tokio::test]
async fn gateway_relay_auth_failure_maps_to_credential_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("relay-default", InvocationFailureClass::Auth);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_contract(
        &payload,
        "CredentialError",
        "credential.provider_auth_failed",
    );
}

#[tokio::test]
async fn gateway_relay_config_failure_maps_to_config_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("set default mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("relay-default", InvocationFailureClass::Config);

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_contract(&payload, "ConfigError", "config.provider_rejected_request");
}

#[tokio::test]
async fn gateway_no_endpoint_maps_to_routing_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    assert!(gateway_state.set_endpoint_availability("official-default", false));
    assert!(gateway_state.set_endpoint_availability("relay-default", false));

    let (status, payload) = request_codex_error(&runtime).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_contract(&payload, "RoutingError", "routing.no_available_endpoint");
}

async fn request_codex_error(runtime: &RuntimeState) -> (StatusCode, Value) {
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
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    (status, payload)
}

fn assert_contract(payload: &Value, expected_category: &str, expected_code: &str) {
    assert_eq!(payload["category"], expected_category);
    assert_eq!(payload["error"], expected_code);
    assert!(
        payload["message"].is_string(),
        "message should be a user-safe string, payload: {payload:?}"
    );
    assert!(
        payload["internal_context"].is_string(),
        "internal_context should be a non-empty string, payload: {payload:?}"
    );
}
