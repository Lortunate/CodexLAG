use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test},
    commands::keys::{create_platform_key_from_runtime, CreatePlatformKeyInput},
    gateway::build_router_for_test,
    routing::policy::RoutingMode, secret_store::SecretKey,
};
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn gateway_auth_health_route_returns_ok() {
    let app = build_router_for_test(bootstrap_state_for_test().await.expect("bootstrap"));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("health request"),
        )
        .await
        .expect("health response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("health body");

    assert_eq!(body.as_ref(), b"ok");
}

#[tokio::test]
async fn gateway_auth_codex_route_rejects_invalid_platform_key() {
    let app = build_router_for_test(bootstrap_state_for_test().await.expect("bootstrap"));

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", "Bearer wrong-platform-key")
                .body(Body::empty())
                .expect("codex request"),
        )
        .await
        .expect("codex response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn gateway_auth_codex_route_accepts_valid_platform_key() {
    let state = bootstrap_state_for_test().await.expect("bootstrap");
    let secret = state
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let app = build_router_for_test(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {}", secret))
                .body(Body::empty())
                .expect("codex request"),
        )
        .await
        .expect("codex response");

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("codex body");

    let payload: Value = serde_json::from_slice(body.as_ref()).expect("gateway response json");

    assert_eq!(payload["platform_key"], "default");
    assert_eq!(payload["policy"], "default");
    assert_eq!(payload["allowed_mode"], "hybrid");
    assert_eq!(payload["endpoint_id"], "official-default");
}

#[tokio::test]
async fn gateway_auth_codex_route_respects_account_only_mode() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap");
    state
        .set_default_key_allowed_mode(RoutingMode::AccountOnly)
        .expect("set account-only mode");
    let secret = state
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let app = build_router_for_test(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {}", secret))
                .body(Body::empty())
                .expect("codex request"),
        )
        .await
        .expect("codex response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("codex body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("gateway response json");
    assert_eq!(payload["endpoint_id"], "official-default");
}

#[tokio::test]
async fn gateway_auth_codex_route_respects_relay_only_mode() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap");
    state
        .set_default_key_allowed_mode(RoutingMode::RelayOnly)
        .expect("set relay-only mode");
    let secret = state
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let app = build_router_for_test(state);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {}", secret))
                .body(Body::empty())
                .expect("codex request"),
        )
        .await
        .expect("codex response");

    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("codex body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("gateway response json");
    assert_eq!(payload["endpoint_id"], "relay-default");
}

#[tokio::test]
async fn newly_created_platform_key_can_authenticate_against_the_gateway() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let created = create_platform_key_from_runtime(
        &runtime,
        CreatePlatformKeyInput {
            key_id: "key-secondary".into(),
            name: "secondary".into(),
            policy_id: "policy-default".into(),
            allowed_mode: "hybrid".into(),
        },
    )
    .expect("create platform key");

    let router = runtime.loopback_gateway().router();
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {}", created.secret))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
}
