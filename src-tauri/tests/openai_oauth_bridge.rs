use axum::{extract::State, routing::post, Form, Json, Router};
use base64::Engine as _;
use codexlag_lib::{
    auth::openai::{OpenAiAuthRuntime, OpenAiBrowserLoginRequest},
    bootstrap::bootstrap_state_for_test_at,
    gateway::auth::GatewayState,
};
use rand::{rngs::OsRng, RngCore};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex, RwLock},
    time::Duration,
};

#[tokio::test]
async fn successful_browser_oauth_registers_gateway_usable_official_account() {
    let database_path = temp_database_path("codexlag-openai-oauth-bridge");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let shared_state = Arc::new(RwLock::new(app_state));
    let mut runtime = OpenAiAuthRuntime::from_shared_app_state(Arc::clone(&shared_state));
    let token_requests = Arc::new(Mutex::new(Vec::<String>::new()));
    let token_endpoint = spawn_openai_token_endpoint(Arc::clone(&token_requests)).await;

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

    let gateway = wait_for_gateway_account(Arc::clone(&shared_state), "openai-primary")
        .await
        .expect("oauth callback should register an official gateway account");

    let official_session = gateway
        .official_session_for_candidate("openai-primary")
        .expect("oauth account should resolve to an official session");
    assert_eq!(official_session.session_id, "session:openai-primary");
    assert_eq!(
        official_session.account_identity.as_deref(),
        Some("user@example.com")
    );
    assert_eq!(official_session.refresh_capability, Some(true));
    assert_eq!(official_session.status, "active");
    assert_eq!(official_session.entitlement.plan_type.as_deref(), Some("pro"));
    assert_eq!(
        official_session.entitlement.subscription_active_until.as_deref(),
        Some("2026-05-01T00:00:00Z")
    );
    assert_eq!(
        official_session.entitlement.claim_source.as_deref(),
        Some("id_token_claim")
    );

    let candidate = gateway
        .current_candidates()
        .into_iter()
        .find(|candidate| candidate.id == "openai-primary")
        .expect("oauth account should be a gateway candidate");
    assert!(
        candidate.available,
        "oauth account should be available after token secrets are persisted"
    );

    let requests = token_requests.lock().expect("token request lock");
    assert_eq!(
        requests.len(),
        1,
        "callback should exchange exactly one code"
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
    state
        .requests
        .lock()
        .expect("record token request")
        .push(serde_json::to_string(&form).expect("serialize token request form"));

    Json(serde_json::json!({
        "access_token": "access-token",
        "refresh_token": "refresh-token",
        "id_token": test_openai_jwt(
            r#"{
                "email":"user@example.com",
                "https://api.openai.com/auth":{
                    "chatgpt_account_id":"acc_123",
                    "chatgpt_plan_type":"pro",
                    "chatgpt_subscription_active_start":"2026-04-01T00:00:00Z",
                    "chatgpt_subscription_active_until":"2026-05-01T00:00:00Z"
                }
            }"#,
        ),
        "expires_in": 3600,
        "token_type": "Bearer"
    }))
}

async fn wait_for_gateway_account(
    app_state: Arc<RwLock<codexlag_lib::state::AppState>>,
    account_id: &str,
) -> Option<GatewayState> {
    for _ in 0..20 {
        let gateway = GatewayState::new(Arc::clone(&app_state), Arc::new(RwLock::new(Vec::new())));
        if gateway.official_session_for_candidate(account_id).is_ok() {
            return Some(gateway);
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

fn test_openai_jwt(payload_json: &str) -> String {
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);

    format!("{header}.{payload}.signature")
}
