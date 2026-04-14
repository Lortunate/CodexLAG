use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use codexlag_lib::{
    bootstrap::{
        bootstrap_runtime_for_test, bootstrap_state_with_provider_inventory_for_test,
    },
    gateway::build_router_for_test,
    routing::{
        engine::{
            choose_endpoint_at, record_failure, CandidateEndpoint, EndpointFailure,
            EndpointHealthState, FailureRules,
        },
        policy::RoutingMode,
    },
    secret_store::SecretKey,
};
use serde_json::Value;
use tower::ServiceExt;

#[test]
fn timeout_and_5xx_classification_follow_configured_thresholds() {
    let rules = FailureRules {
        timeout_open_after: 2,
        server_error_open_after: 3,
        cooldown_ms: 200,
        ..FailureRules::default()
    };
    let mut endpoint = CandidateEndpoint::official("official-1", 10, true);
    let now_ms = 20_000;

    assert_eq!(
        record_failure(&mut endpoint, EndpointFailure::Timeout, now_ms, &rules),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(502),
            now_ms + 1,
            &rules
        ),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(503),
            now_ms + 2,
            &rules
        ),
        EndpointHealthState::Degraded
    );
    assert_eq!(
        record_failure(
            &mut endpoint,
            EndpointFailure::HttpStatus(504),
            now_ms + 3,
            &rules
        ),
        EndpointHealthState::OpenCircuit
    );
}

#[tokio::test]
async fn codex_request_account_only_succeeds_with_imported_provider_inventory() {
    let mut state = bootstrap_state_with_provider_inventory_for_test()
        .await
        .expect("bootstrap");
    state
        .set_default_key_allowed_mode(RoutingMode::AccountOnly)
        .expect("set account_only mode");
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
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");

    assert_eq!(payload["allowed_mode"], "account_only");
    assert_eq!(payload["endpoint_id"], "official-primary");
}

#[tokio::test]
async fn production_requests_ignore_test_route_status_header_by_default() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
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
                .header(
                    "x-codexlag-endpoint-status",
                    "official-primary:503,relay-newapi:503",
                )
                .body(Body::empty())
                .expect("codex request"),
        )
        .await
        .expect("codex response");

    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn cooldown_recovery_reenables_open_candidate() {
    let rules = FailureRules {
        cooldown_ms: 75,
        ..FailureRules::default()
    };
    let now_ms = 30_000;
    let mut endpoint = CandidateEndpoint::relay("relay-1", 10, true);
    let _ = record_failure(
        &mut endpoint,
        EndpointFailure::HttpStatus(429),
        now_ms,
        &rules,
    );
    let blocked = choose_endpoint_at("relay_only", &[endpoint.clone()], now_ms + 10);
    assert!(blocked.is_err());

    let recovered = choose_endpoint_at("relay_only", &[endpoint], now_ms + 80);
    assert!(recovered.is_ok(), "candidate should recover after cooldown");
}

#[tokio::test]
async fn codex_request_returns_typed_routing_error_when_all_candidates_are_unavailable() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let gateway_state = runtime.loopback_gateway().state();
    for candidate in gateway_state.current_candidates() {
        assert!(
            gateway_state.set_endpoint_availability(candidate.id.as_str(), false),
            "candidate availability should be mutable for {}",
            candidate.id
        );
    }

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
                .expect("codex request"),
        )
        .await
        .expect("codex response");
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("route body");
    let payload: Value = serde_json::from_slice(body.as_ref()).expect("route json");
    assert_eq!(payload["category"], "RoutingError");
    assert_eq!(payload["error"], "routing.no_available_endpoint");
}
