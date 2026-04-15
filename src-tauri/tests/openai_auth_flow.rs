use std::path::PathBuf;

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
        .store_session(summary.clone(), "session-cookie".into(), token_secret.clone())
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
    assert!(pending.authorization_url.contains("client_id=codexlag-desktop"));
    assert!(pending.authorization_url.contains("code_challenge="));
    assert!(pending.authorization_url.contains("state="));
    assert!(
        pending.authorization_url.contains("scope=openid")
            || pending.authorization_url.contains("scope=openid+profile+offline_access")
    );
    assert!(pending.callback_url.starts_with("http://127.0.0.1:"));
    assert!(pending.callback_url.ends_with("/auth/openai/callback"));
}

fn temp_database_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.sqlite3", random_suffix()))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
