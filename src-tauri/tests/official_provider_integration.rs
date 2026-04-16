use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use codexlag_lib::bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test};
use codexlag_lib::commands::accounts::{
    import_official_account_login_from_runtime, OfficialAccountImportInput,
};
use codexlag_lib::commands::logs::{
    usage_request_detail_from_runtime, usage_request_history_from_runtime,
};
use codexlag_lib::providers::official::official_entitlement_from_token_secret;
use codexlag_lib::secret_store::SecretKey;
use codexlag_lib::state::{RuntimeLogConfig, RuntimeState};
use serde_json::{json, Value};
use tokio::net::TcpListener;
use tower::ServiceExt;

#[derive(Clone, Default)]
struct CapturedOfficialRequests {
    values: Arc<Mutex<Vec<String>>>,
}

impl CapturedOfficialRequests {
    fn push(&self, authorization: String) {
        self.values
            .lock()
            .expect("captured official request lock")
            .push(authorization);
    }

    fn values(&self) -> Vec<String> {
        self.values
            .lock()
            .expect("captured official request lock")
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

async fn spawn_official_upstream() -> (String, CapturedOfficialRequests) {
    async fn handle_request(
        State(captured): State<CapturedOfficialRequests>,
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
            "id": "resp_official_1",
            "model": "gpt-5-mini",
            "usage": {
                "input_tokens": 11,
                "output_tokens": 7,
                "input_tokens_details": {
                    "cached_tokens": 2
                },
                "output_tokens_details": {
                    "reasoning_tokens": 3
                }
            }
        }))
    }

    let captured = CapturedOfficialRequests::default();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind official upstream");
    let address = listener.local_addr().expect("official upstream address");
    let router = Router::new()
        .route("/responses", post(handle_request))
        .with_state(captured.clone());
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve official upstream");
    });
    (format!("http://{address}"), captured)
}

#[tokio::test]
async fn imported_official_account_exposes_runtime_status_and_identity() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let detail = codexlag_lib::commands::accounts::get_account_capability_detail_from_runtime(
        &runtime,
        "official-primary".to_string(),
    )
    .expect("official capability detail");

    assert_eq!(detail.provider, "openai");
    assert!(detail.refresh_capability.is_some());
    assert!(!detail.status.is_empty());
    assert!(detail.account_identity.is_some());
}

#[tokio::test]
async fn imported_official_account_runtime_path_requires_stored_session_secret() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let account_id = "official-runtime-secret-check".to_string();
    let session_ref = "credential://official/session/runtime-secret-check".to_string();
    let token_ref = "credential://official/token/runtime-secret-check".to_string();

    import_official_account_login_from_runtime(
        &runtime,
        OfficialAccountImportInput {
            account_id: account_id.clone(),
            name: "Runtime Secret Check".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: session_ref.clone(),
            token_credential_ref: token_ref.clone(),
            account_identity: Some("runtime-secret-check@example.test".to_string()),
            auth_mode: Some("api_key".to_string()),
        },
    )
    .expect("import official account");

    let missing = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate(account_id.as_str());
    assert!(
        missing.is_err(),
        "runtime should require stored session/token credentials before selecting imported official account"
    );

    runtime
        .app_state()
        .store_secret(&SecretKey::new(session_ref), "session-secret".to_string())
        .expect("store official session secret");
    runtime
        .app_state()
        .store_secret(&SecretKey::new(token_ref), "token-secret".to_string())
        .expect("store official token secret");

    let hydrated = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate(account_id.as_str())
        .expect("runtime should hydrate official session when both secrets exist");
    assert_eq!(hydrated.session_id, format!("session:{account_id}"));
}

#[tokio::test]
async fn official_entitlement_projects_from_stored_openai_token_secret() {
    let token_secret = json!({
        "access_token": "access-token",
        "refresh_token": "refresh-token",
        "id_token": test_openai_jwt(
            r#"{
                "email":"official-entitled@example.test",
                "https://api.openai.com/auth":{
                    "chatgpt_plan_type":"pro",
                    "chatgpt_subscription_active_start":"2026-04-01T00:00:00Z",
                    "chatgpt_subscription_active_until":"2026-05-01T00:00:00Z"
                }
            }"#,
        )
    })
    .to_string();

    let entitlement = official_entitlement_from_token_secret(token_secret.as_str());

    assert_eq!(entitlement.plan_type.as_deref(), Some("pro"));
    assert_eq!(entitlement.claim_source.as_deref(), Some("id_token_claim"));
    assert_eq!(
        entitlement.subscription_active_start.as_deref(),
        Some("2026-04-01T00:00:00Z")
    );
    assert_eq!(
        entitlement.subscription_active_until.as_deref(),
        Some("2026-05-01T00:00:00Z")
    );
}

#[tokio::test]
async fn official_provider_invokes_real_upstream_request_from_imported_login_state() {
    let (base_url, captured) = spawn_official_upstream().await;
    let state = bootstrap_state_for_test().await.expect("bootstrap state");
    let account_id = "official-live".to_string();
    let session_ref = "credential://official/session/official-live".to_string();
    let token_ref = "credential://official/token/official-live".to_string();
    let runtime = RuntimeState::new(
        state,
        RuntimeLogConfig {
            log_dir: unique_test_dir("official-provider"),
        },
    );
    import_official_account_login_from_runtime(
        &runtime,
        OfficialAccountImportInput {
            account_id: account_id.clone(),
            name: "Official Live".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: session_ref.clone(),
            token_credential_ref: token_ref.clone(),
            account_identity: Some("official-live@example.test".to_string()),
            auth_mode: Some("api_key".to_string()),
        },
    )
    .expect("import official account");
    runtime
        .app_state()
        .store_secret(&SecretKey::new(session_ref), "session-secret".to_string())
        .expect("store session secret");
    runtime
        .app_state()
        .store_secret(
            &SecretKey::new(token_ref),
            json!({
                "api_key": "official-live-key",
                "base_url": base_url,
            })
            .to_string(),
        )
        .expect("store token secret");
    let default_policy = runtime
        .app_state()
        .default_policy()
        .expect("default policy")
        .clone();
    runtime
        .app_state_mut()
        .save_policy(codexlag_lib::models::RoutingPolicy {
            selection_order: vec![account_id.clone()],
            ..default_policy
        })
        .expect("prioritize imported account");
    runtime
        .rebuild_gateway_candidates()
        .expect("rebuild candidates with stored secrets");

    runtime
        .set_current_mode(codexlag_lib::routing::policy::RoutingMode::AccountOnly)
        .expect("switch mode");
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
                .expect("official request"),
        )
        .await
        .expect("official response");

    assert_eq!(response.status(), StatusCode::OK);
    let history = usage_request_history_from_runtime(&runtime, Some(10));
    assert_eq!(history.len(), 1);
    let detail = usage_request_detail_from_runtime(&runtime, history[0].request_id.as_str())
        .expect("official request detail");
    assert_eq!(detail.model.as_deref(), Some("gpt-5-mini"));
    assert_eq!(detail.input_tokens, 11);
    assert_eq!(detail.output_tokens, 7);
    assert_eq!(detail.cache_read_tokens, 2);
    assert_eq!(detail.reasoning_tokens, 3);
    assert_eq!(
        captured.values(),
        vec!["Bearer official-live-key".to_string()]
    );
}

fn test_openai_jwt(payload_json: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(payload_json);

    format!("{header}.{payload}.signature")
}
