use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{extract::State, routing::post, Form, Json, Router};
use codexlag_lib::{
    auth::openai::{OpenAiAuthRuntime, OpenAiBrowserLoginRequest},
    bootstrap::bootstrap_state_for_test_at,
    models::ProviderSessionSummary,
};
use rand::{rngs::OsRng, RngCore};

#[tokio::test]
async fn openai_auth_session_round_trips_through_runtime_storage() {
    let database_path = temp_database_path("codexlag-openai-auth-flow");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);

    let token_secret = serde_json::json!({
        "access_token": "access-token",
        "refresh_token": "refresh-token",
        "expires_at_ms": 1_731_111_111_000_i64,
    })
    .to_string();
    let summary = ProviderSessionSummary {
        provider_id: "openai_official".into(),
        account_id: "openai-primary".into(),
        display_name: "OpenAI Primary".into(),
        auth_state: "active".into(),
        expires_at_ms: Some(1_731_111_111_000),
        last_refresh_at_ms: Some(1_731_111_000_500),
        last_refresh_error: None,
    };

    runtime
        .store_session(
            summary.clone(),
            "session-cookie".into(),
            token_secret.clone(),
        )
        .expect("store openai auth session");

    drop(runtime);

    let reloaded_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("re-bootstrap isolated app state");
    let reloaded_runtime = OpenAiAuthRuntime::new(reloaded_state);
    let stored = reloaded_runtime
        .session("openai-primary")
        .expect("load openai auth session")
        .expect("openai auth session should be present");

    assert_eq!(stored.summary, summary);
    assert_eq!(stored.session_secret, "session-cookie");
    assert_eq!(stored.token_secret, token_secret);
}

#[tokio::test]
async fn starting_openai_browser_login_returns_a_pending_loopback_auth_session() {
    let database_path = temp_database_path("codexlag-openai-auth-login");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);

    let pending = runtime
        .start_browser_login(OpenAiBrowserLoginRequest {
            account_id: "openai-primary".into(),
            display_name: "OpenAI Primary".into(),
            client_id: "codexlag-desktop".into(),
            issuer_url: "https://auth.openai.example".into(),
            authorization_endpoint: "https://auth.openai.example/oauth2/v1/authorize".into(),
            token_endpoint: "https://auth.openai.example/oauth2/v1/token".into(),
            scopes: vec!["openid".into(), "profile".into(), "offline_access".into()],
        })
        .expect("start browser login");

    assert_eq!(pending.summary.provider_id, "openai_official");
    assert_eq!(pending.summary.account_id, "openai-primary");
    assert_eq!(pending.summary.display_name, "OpenAI Primary");
    assert_eq!(pending.summary.auth_state, "pending");
    assert!(pending.authorization_url.contains("response_type=code"));
    assert!(pending
        .authorization_url
        .contains("client_id=codexlag-desktop"));
    assert!(pending.authorization_url.contains("code_challenge="));
    assert!(pending.authorization_url.contains("state="));
    assert!(
        pending.authorization_url.contains("scope=openid")
            || pending
                .authorization_url
                .contains("scope=openid+profile+offline_access")
    );
    assert!(pending.callback_url.starts_with("http://127.0.0.1:"));
    assert!(pending.callback_url.ends_with("/auth/openai/callback"));
}

#[tokio::test]
async fn openai_loopback_callback_persists_active_session_after_code_exchange() {
    let database_path = temp_database_path("codexlag-openai-auth-callback");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);
    let token_requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let token_endpoint = spawn_openai_token_endpoint(token_requests.clone()).await;

    let pending = runtime
        .start_browser_login(OpenAiBrowserLoginRequest {
            account_id: "openai-primary".into(),
            display_name: "OpenAI Primary".into(),
            client_id: "codexlag-desktop".into(),
            issuer_url: token_endpoint.base_url.clone(),
            authorization_endpoint: format!("{}/oauth2/v1/authorize", token_endpoint.base_url),
            token_endpoint: format!("{}/oauth2/v1/token", token_endpoint.base_url),
            scopes: vec!["openid".into(), "profile".into(), "offline_access".into()],
        })
        .expect("start browser login");

    let state = query_parameter(pending.authorization_url.as_str(), "state")
        .expect("authorization url should include a state parameter");
    let callback_response = reqwest::Client::new()
        .get(format!(
            "{}?code=sample-auth-code&state={state}",
            pending.callback_url
        ))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("complete loopback callback");
    assert!(callback_response.status().is_success());

    let stored = wait_for_session(&runtime, "openai-primary")
        .await
        .expect("callback should persist an active session");

    assert_eq!(stored.summary.provider_id, "openai_official");
    assert_eq!(stored.summary.auth_state, "active");
    assert_eq!(stored.summary.account_id, "openai-primary");
    assert_eq!(stored.session_secret, "access-token");
    assert!(stored.token_secret.contains("refresh-token"));

    let requests = token_requests.lock().expect("token request lock");
    assert_eq!(requests.len(), 1, "callback should exchange exactly one auth code");
    assert!(
        requests[0].contains("grant_type=authorization_code"),
        "token exchange should use the auth code flow"
    );
    assert!(
        requests[0].contains("code_verifier="),
        "token exchange should forward the PKCE verifier"
    );
}

#[tokio::test]
async fn openai_loopback_callback_state_mismatch_marks_session_reauth_required() {
    let database_path = temp_database_path("codexlag-openai-auth-state-mismatch");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);

    let pending = runtime
        .start_browser_login(OpenAiBrowserLoginRequest {
            account_id: "openai-primary".into(),
            display_name: "OpenAI Primary".into(),
            client_id: "codexlag-desktop".into(),
            issuer_url: "https://auth.openai.example".into(),
            authorization_endpoint: "https://auth.openai.example/oauth2/v1/authorize".into(),
            token_endpoint: "https://auth.openai.example/oauth2/v1/token".into(),
            scopes: vec!["openid".into(), "profile".into(), "offline_access".into()],
        })
        .expect("start browser login");

    let callback_response = reqwest::Client::new()
        .get(format!(
            "{}?code=sample-auth-code&state=wrong-state",
            pending.callback_url
        ))
        .timeout(Duration::from_secs(1))
        .send()
        .await
        .expect("complete loopback callback");
    assert!(!callback_response.status().is_success());

    let stored = wait_for_session(&runtime, "openai-primary")
        .await
        .expect("pending session should still be persisted");

    assert_eq!(stored.summary.auth_state, "reauth_required");
    assert!(
        stored
            .summary
            .last_refresh_error
            .as_deref()
            .is_some_and(|value| value.contains("state did not match"))
    );
}

fn temp_database_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.sqlite3", random_suffix()))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct TokenEndpointHandle {
    base_url: String,
}

#[derive(Clone)]
struct TokenEndpointState {
    requests: Arc<Mutex<Vec<String>>>,
}

async fn spawn_openai_token_endpoint(requests: Arc<Mutex<Vec<String>>>) -> TokenEndpointHandle {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind token endpoint listener");
    let address = listener
        .local_addr()
        .expect("token endpoint listener should expose an address");
    let state = TokenEndpointState { requests };
    let router = Router::new()
        .route("/oauth2/v1/token", post(exchange_openai_code))
        .with_state(state);

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve token endpoint");
    });

    TokenEndpointHandle {
        base_url: format!("http://{address}"),
    }
}

async fn exchange_openai_code(
    State(state): State<TokenEndpointState>,
    Form(form): Form<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let encoded = form
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&");
    state
        .requests
        .lock()
        .expect("token request lock")
        .push(encoded);

    Json(serde_json::json!({
        "access_token": "access-token",
        "refresh_token": "refresh-token",
        "expires_in": 3600,
        "token_type": "Bearer"
    }))
}

async fn wait_for_session(
    runtime: &OpenAiAuthRuntime,
    account_id: &str,
) -> Option<codexlag_lib::auth::session_store::StoredProviderSession> {
    for _ in 0..20 {
        if let Some(session) = runtime
            .session(account_id)
            .expect("load provider session while polling")
        {
            return Some(session);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    None
}

fn query_parameter(url: &str, key: &str) -> Option<String> {
    let query = url.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=')?;
        (name == key).then(|| value.to_string())
    })
}
