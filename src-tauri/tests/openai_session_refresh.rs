use std::path::PathBuf;

use codexlag_lib::{
    auth::{
        openai::{OpenAiAuthRuntime, OpenAiSessionRefresh, OpenAiSessionRefresher},
        session_store::StoredProviderSession,
    },
    bootstrap::{bootstrap_openai_auth_runtime_for_test_at, bootstrap_state_for_test_at},
    error::Result,
    models::ProviderSessionSummary,
};
use rand::{rngs::OsRng, RngCore};

struct FakeOpenAiSessionRefresher;

impl OpenAiSessionRefresher for FakeOpenAiSessionRefresher {
    fn refresh(&self, _session: &StoredProviderSession) -> Result<OpenAiSessionRefresh> {
        Ok(OpenAiSessionRefresh {
            session_secret: "new-session-cookie".into(),
            token_secret: serde_json::json!({
                "access_token": "fresh-access-token",
                "refresh_token": "refresh-token",
            })
            .to_string(),
            expires_at_ms: Some(1_700_000_003_600),
            refreshed_at_ms: 1_700_000_000_100,
        })
    }
}

#[tokio::test]
async fn expired_openai_session_is_refreshed_during_runtime_startup_when_refreshable() {
    let database_path = temp_database_path("codexlag-openai-session-refresh");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);
    let expired_summary = ProviderSessionSummary {
        provider_id: "openai_official".into(),
        account_id: "openai-primary".into(),
        display_name: "OpenAI Primary".into(),
        auth_state: "active".into(),
        expires_at_ms: Some(1_700_000_000_000),
        last_refresh_at_ms: None,
        last_refresh_error: None,
    };

    runtime
        .store_session(
            expired_summary,
            "old-session-cookie".into(),
            serde_json::json!({
                "access_token": "stale-access-token",
                "refresh_token": "refresh-token",
            })
            .to_string(),
        )
        .expect("store expired openai session");

    drop(runtime);

    let refreshed_runtime =
        bootstrap_openai_auth_runtime_for_test_at(&database_path, &FakeOpenAiSessionRefresher)
            .await
            .expect("bootstrap openai auth runtime with refresh");
    let sessions = refreshed_runtime
        .list_provider_sessions()
        .expect("list provider sessions after startup refresh");
    assert!(sessions
        .iter()
        .all(|session| session.auth_state != "expired"));

    let refreshed = refreshed_runtime
        .openai_auth_mut()
        .session("openai-primary")
        .expect("load refreshed session")
        .expect("refreshed session should exist");

    assert_eq!(refreshed.summary.auth_state, "active");
    assert_eq!(refreshed.summary.expires_at_ms, Some(1_700_000_003_600));
    assert_eq!(
        refreshed.summary.last_refresh_at_ms,
        Some(1_700_000_000_100)
    );
    assert_eq!(refreshed.summary.last_refresh_error, None);
    assert!(refreshed.is_refreshable());
    assert_eq!(refreshed.session_secret, "new-session-cookie");
    assert!(refreshed.token_secret.contains("fresh-access-token"));
}

#[tokio::test]
async fn near_expiry_openai_session_refreshes_before_it_is_fully_expired() {
    let database_path = temp_database_path("codexlag-openai-near-expiry-refresh");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let mut runtime = OpenAiAuthRuntime::new(app_state);
    runtime
        .store_session(
            ProviderSessionSummary {
                provider_id: "openai_official".into(),
                account_id: "openai-primary".into(),
                display_name: "OpenAI Primary".into(),
                auth_state: "active".into(),
                expires_at_ms: Some(1_700_000_240_000),
                last_refresh_at_ms: None,
                last_refresh_error: None,
            },
            "old-session-cookie".into(),
            serde_json::json!({
                "access_token": "stale-access-token",
                "refresh_token": "refresh-token",
            })
            .to_string(),
        )
        .expect("store near-expiry session");

    let refreshed = runtime
        .refresh_session_if_needed("openai-primary", 1_700_000_000_000, &FakeOpenAiSessionRefresher)
        .expect("refresh near-expiry session")
        .expect("session should refresh inside the refresh window");

    assert_eq!(refreshed.summary.auth_state, "active");
    assert_eq!(refreshed.summary.expires_at_ms, Some(1_700_000_003_600));
    assert_eq!(refreshed.summary.last_refresh_at_ms, Some(1_700_000_000_100));
    assert!(refreshed.token_secret.contains("fresh-access-token"));
}

fn temp_database_path(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!("{prefix}-{}.sqlite3", random_suffix()))
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
