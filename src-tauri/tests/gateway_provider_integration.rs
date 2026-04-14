use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test},
    gateway::build_router_for_test,
    providers::invocation::InvocationFailureClass,
    routing::policy::RoutingMode,
    secret_store::SecretKey,
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn official_provider_path_can_be_selected_from_runtime_inventory() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "official-primary"),
        "official runtime inventory should include official-primary"
    );
}

#[tokio::test]
async fn relay_provider_path_can_be_selected_from_runtime_inventory() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "relay-newapi"),
        "relay runtime inventory should include relay-newapi"
    );
}

#[tokio::test]
async fn request_and_attempt_ids_are_carried_across_failover_attempts() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Http5xx);

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
    assert_eq!(payload["endpoint_id"], "relay-newapi");

    let attempts = gateway_state.invocation_attempts_for_test();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0].endpoint_id, "official-primary");
    assert_eq!(attempts[1].endpoint_id, "relay-newapi");
    assert_eq!(attempts[0].request_id, attempts[1].request_id);
    assert_eq!(
        attempts[0].attempt_id,
        format!("{}:0", attempts[0].request_id)
    );
    assert_eq!(
        attempts[1].attempt_id,
        format!("{}:1", attempts[1].request_id)
    );
}

#[tokio::test]
async fn rate_limited_failover_opens_endpoint_and_models_only_show_routable_pool() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Http429);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let first_response = runtime
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
    assert_eq!(first_response.status(), StatusCode::OK);
    let first_body = to_bytes(first_response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let first_payload: Value = serde_json::from_slice(first_body.as_ref()).expect("route json");
    assert_eq!(first_payload["endpoint_id"], "relay-newapi");

    let models_response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/models")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("models request"),
        )
        .await
        .expect("models response");
    assert_eq!(models_response.status(), StatusCode::OK);
    let models_body = to_bytes(models_response.into_body(), usize::MAX)
        .await
        .expect("models body");
    let models_payload: Value =
        serde_json::from_slice(models_body.as_ref()).expect("models route json");
    assert_eq!(models_payload["allowed_mode"], "hybrid");
    assert_eq!(
        models_payload["models"],
        json!(["gpt-4.1-mini", "gpt-4o-mini"])
    );

    let second_response = runtime
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
    assert_eq!(second_response.status(), StatusCode::OK);
    let second_body = to_bytes(second_response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let second_payload: Value = serde_json::from_slice(second_body.as_ref()).expect("route json");
    assert_eq!(second_payload["endpoint_id"], "relay-newapi");
}

#[tokio::test]
async fn auth_failure_does_not_cross_pool_failover_and_reports_no_endpoint() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state.plan_provider_failure_for_test("official-primary", InvocationFailureClass::Auth);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let first_response = runtime
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
    assert_eq!(first_response.status(), StatusCode::UNAUTHORIZED);
    let first_body = to_bytes(first_response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let first_payload: Value = serde_json::from_slice(first_body.as_ref()).expect("route json");
    assert_eq!(first_payload["category"], "CredentialError");
    assert_eq!(first_payload["error"], "credential.provider_auth_failed");
    assert_eq!(first_payload["attempt_count"], 1);

    let attempts = gateway_state.invocation_attempts_for_test();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].endpoint_id, "official-primary");
}

#[tokio::test]
async fn config_failure_does_not_cross_pool_failover_and_reports_no_endpoint() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    gateway_state
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Config);

    let secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");

    let first_response = runtime
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
    assert_eq!(first_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let first_body = to_bytes(first_response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let first_payload: Value = serde_json::from_slice(first_body.as_ref()).expect("route json");
    assert_eq!(first_payload["category"], "ConfigError");
    assert_eq!(first_payload["error"], "config.provider_rejected_request");
    assert_eq!(first_payload["attempt_count"], 1);

    let attempts = gateway_state.invocation_attempts_for_test();
    assert_eq!(attempts.len(), 1);
    assert_eq!(attempts[0].endpoint_id, "official-primary");
}

#[tokio::test]
async fn quota_failure_without_fallback_emits_typed_quota_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("switch mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Http429);

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
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    assert_eq!(payload["category"], "QuotaError");
    assert_eq!(payload["error"], "quota.provider_rate_limited");
    assert_eq!(payload["attempt_count"], 1);
}

#[tokio::test]
async fn timeout_failure_without_fallback_emits_typed_upstream_error_contract() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime
        .set_current_mode(RoutingMode::AccountOnly)
        .expect("switch mode");
    runtime
        .loopback_gateway()
        .state()
        .plan_provider_failure_for_test("official-primary", InvocationFailureClass::Timeout);

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
    assert_eq!(payload["attempt_count"], 1);
}

#[tokio::test]
async fn models_route_returns_allowed_model_list_for_current_policy_mode() {
    let account_only = models_payload_for_mode(RoutingMode::AccountOnly).await;
    assert_eq!(account_only["allowed_mode"], "account_only");
    assert_eq!(
        account_only["models"],
        json!(["claude-3-5-sonnet", "claude-3-7-sonnet"])
    );

    let relay_only = models_payload_for_mode(RoutingMode::RelayOnly).await;
    assert_eq!(relay_only["allowed_mode"], "relay_only");
    assert_eq!(relay_only["models"], json!(["gpt-4.1-mini", "gpt-4o-mini"]));

    let hybrid = models_payload_for_mode(RoutingMode::Hybrid).await;
    assert_eq!(hybrid["allowed_mode"], "hybrid");
    assert_eq!(
        hybrid["models"],
        json!([
            "claude-3-5-sonnet",
            "claude-3-7-sonnet",
            "gpt-4.1-mini",
            "gpt-4o-mini"
        ])
    );
}

async fn models_payload_for_mode(mode: RoutingMode) -> Value {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap");
    state
        .set_default_key_allowed_mode(mode)
        .expect("set default key mode");
    let secret = state
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");
    let app = build_router_for_test(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/models")
                .header("authorization", format!("bearer {secret}"))
                .body(Body::empty())
                .expect("models request"),
        )
        .await
        .expect("models response");
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("models body");
    serde_json::from_slice(body.as_ref()).expect("models json")
}
