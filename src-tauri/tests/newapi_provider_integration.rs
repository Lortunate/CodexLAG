use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    routing::{get, post},
    Json, Router,
};
use codexlag_lib::bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test};
use codexlag_lib::commands::logs::{
    usage_request_detail_from_runtime, usage_request_history_from_runtime,
};
use codexlag_lib::commands::relays::{
    add_relay_from_runtime, refresh_relay_balance_from_runtime, RelayUpsertInput,
};
use codexlag_lib::secret_store::SecretKey;
use codexlag_lib::state::{RuntimeLogConfig, RuntimeState};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct CapturedRelayRequests {
    values: Arc<Mutex<Vec<String>>>,
}

impl CapturedRelayRequests {
    fn push(&self, authorization: String) {
        self.values
            .lock()
            .expect("captured relay request lock")
            .push(authorization);
    }

    fn values(&self) -> Vec<String> {
        self.values
            .lock()
            .expect("captured relay request lock")
            .clone()
    }
}

fn unique_test_dir(prefix: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    std::env::temp_dir()
        .join("codexlag-provider-tests")
        .join(format!("{prefix}-{suffix}"))
}

async fn spawn_newapi_upstream() -> (String, CapturedRelayRequests) {
    async fn handle_balance() -> Json<Value> {
        Json(json!({
            "data": {
                "total_balance": "19.75",
                "used_balance": "4.25"
            }
        }))
    }

    async fn handle_chat(
        State(captured): State<CapturedRelayRequests>,
        request: Request<Body>,
    ) -> Json<Value> {
        let authorization = request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_string();
        captured.push(authorization);
        Json(json!({
            "id": "chatcmpl-relay-1",
            "model": "gpt-4o-mini",
            "usage": {
                "prompt_tokens": 640,
                "completion_tokens": 128,
                "prompt_tokens_details": {
                    "cached_tokens": 256
                },
                "completion_tokens_details": {
                    "reasoning_tokens": 32
                }
            }
        }))
    }

    let captured = CapturedRelayRequests::default();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind relay upstream");
    let address = listener.local_addr().expect("relay upstream address");
    let router = Router::new()
        .route("/v1/api/user/self", get(handle_balance))
        .route("/v1/chat/completions", post(handle_chat))
        .with_state(captured.clone());
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve relay upstream");
    });
    (format!("http://{address}/v1"), captured)
}

#[tokio::test]
async fn newapi_relay_balance_refresh_uses_adapter_logic_instead_of_fixtures() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let snapshot = refresh_relay_balance_from_runtime(&runtime, "relay-newapi".to_string())
        .expect("refresh relay balance");
    assert_eq!(snapshot.relay_id, "relay-newapi");
    assert!(snapshot.endpoint.starts_with("http://127.0.0.1:"));
    assert!(snapshot.endpoint.ends_with("/v1"));
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
async fn managed_newapi_relay_balance_refresh_requires_stored_api_key_secret() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    add_relay_from_runtime(
        &runtime,
        RelayUpsertInput {
            relay_id: "relay-newapi-missing-secret".to_string(),
            name: "Relay Missing Secret".to_string(),
            endpoint: "https://relay.example".to_string(),
            adapter: Some("newapi".to_string()),
            api_key_credential_ref: Some(
                "credential://relay/api-key/relay-newapi-missing-secret".to_string(),
            ),
        },
    )
    .expect("create relay");

    let error =
        refresh_relay_balance_from_runtime(&runtime, "relay-newapi-missing-secret".to_string())
            .expect_err("missing relay api-key secret should fail");
    assert!(error
        .to_string()
        .contains("secret 'credential://relay/api-key/relay-newapi-missing-secret' not found"));
}

#[tokio::test]
async fn relay_provider_executes_real_newapi_http_request() {
    let (endpoint, captured) = spawn_newapi_upstream().await;
    let state = bootstrap_state_for_test().await.expect("bootstrap state");
    let runtime = RuntimeState::new(
        state,
        RuntimeLogConfig {
            log_dir: unique_test_dir("relay-provider"),
        },
    );

    add_relay_from_runtime(
        &runtime,
        RelayUpsertInput {
            relay_id: "relay-live".to_string(),
            name: "Relay Live".to_string(),
            endpoint: endpoint.clone(),
            adapter: Some("newapi".to_string()),
            api_key_credential_ref: Some("credential://relay/api-key/relay-live".to_string()),
        },
    )
    .expect("save relay");
    runtime
        .app_state()
        .store_secret(
            &SecretKey::new("credential://relay/api-key/relay-live".to_string()),
            "relay-live-key".to_string(),
        )
        .expect("store relay secret");
    let default_policy = runtime
        .app_state()
        .default_policy()
        .expect("default policy")
        .clone();
    runtime
        .app_state_mut()
        .save_policy(codexlag_lib::models::RoutingPolicy {
            selection_order: vec!["relay-live".to_string()],
            ..default_policy
        })
        .expect("prioritize live relay");
    runtime
        .rebuild_gateway_candidates()
        .expect("rebuild candidates with stored secrets");

    runtime
        .set_current_mode(codexlag_lib::routing::policy::RoutingMode::RelayOnly)
        .expect("switch relay mode");
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
                .expect("relay request"),
        )
        .await
        .expect("relay response");

    assert_eq!(response.status(), StatusCode::OK);
    let history = usage_request_history_from_runtime(&runtime, Some(10));
    assert_eq!(history.len(), 1);
    let detail = usage_request_detail_from_runtime(&runtime, history[0].request_id.as_str())
        .expect("relay request detail");
    assert_eq!(detail.model.as_deref(), Some("gpt-4o-mini"));
    assert_eq!(detail.input_tokens, 640);
    assert_eq!(detail.output_tokens, 128);
    assert_eq!(detail.cache_read_tokens, 256);
    assert_eq!(detail.reasoning_tokens, 32);
    assert_eq!(detail.total_tokens, 1_056);
    assert_eq!(captured.values(), vec!["Bearer relay-live-key".to_string()]);
}
