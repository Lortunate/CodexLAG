use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::bootstrap_state_for_test,
    gateway::build_router_for_test,
    secret_store::SecretKey,
};
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

    assert_eq!(body.as_ref(), b"default");
}
