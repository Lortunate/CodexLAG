use axum::{
    routing::{get, post},
    Json, Router,
};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{rngs::OsRng, RngCore};
use tokio::net::TcpListener;

use crate::{
    auth::openai::{OpenAiAuthRuntime, OpenAiSessionRefresher},
    commands::{accounts::default_primary_account, relays::default_relays},
    db::repositories::Repositories,
    error::{CodexLagError, Result},
    models::{FailureRules, PlatformKey, RecoveryRules, RoutingPolicy},
    routing::policy::HYBRID,
    secret_store::{SecretKey, SecretStore},
    state::{AppState, RuntimeLogConfig, RuntimeState},
};

const DEFAULT_POLICY_ID: &str = "policy-default";
const DEFAULT_POLICY_NAME: &str = "default";
const DEFAULT_PLATFORM_KEY_ID: &str = "key-default";
const DEFAULT_PLATFORM_KEY_NAME: &str = "default";
pub const DEFAULT_PLATFORM_KEY_SECRET_PREFIX: &str = "ck_local_";
const DEFAULT_OFFICIAL_SESSION_REF: &str = "credential://official/session/official-primary";
const DEFAULT_OFFICIAL_TOKEN_REF: &str = "credential://official/token/official-primary";

fn build_default_app_state(
    database_path: impl AsRef<Path>,
    secret_store: SecretStore,
) -> Result<AppState> {
    let mut repositories = Repositories::open(database_path)?;

    let default_policy = RoutingPolicy {
        id: DEFAULT_POLICY_ID.into(),
        name: DEFAULT_POLICY_NAME.into(),
        selection_order: Vec::new(),
        cross_pool_fallback: true,
        retry_budget: 0,
        failure_rules: FailureRules::default(),
        recovery_rules: RecoveryRules::default(),
    };

    if repositories.policy(DEFAULT_POLICY_NAME).is_none() {
        repositories.insert_policy(default_policy)?;
    }

    let default_policy = repositories
        .policy(DEFAULT_POLICY_NAME)
        .cloned()
        .ok_or_else(|| CodexLagError::new("default policy missing after bootstrap insert"))?;

    let default_key = PlatformKey {
        id: DEFAULT_PLATFORM_KEY_ID.into(),
        name: DEFAULT_PLATFORM_KEY_NAME.into(),
        key_prefix: DEFAULT_PLATFORM_KEY_SECRET_PREFIX.into(),
        allowed_mode: HYBRID.into(),
        policy_id: default_policy.id.clone(),
        enabled: true,
        created_at_ms: now_ms(),
        last_used_at_ms: None,
    };

    let default_key_secret = SecretKey::platform_key(default_key.id.clone());

    if repositories
        .platform_key(DEFAULT_PLATFORM_KEY_NAME)
        .is_none()
    {
        repositories.insert_platform_key(default_key)?;
    }

    if secret_store.get_optional(&default_key_secret)?.is_none() {
        secret_store.set(&default_key_secret, generate_platform_key_secret())?;
    }
    Ok(AppState::new(repositories, secret_store))
}

pub fn bootstrap_state_at(database_path: impl AsRef<Path>) -> Result<AppState> {
    build_default_app_state(database_path, SecretStore::production()?)
}

pub fn bootstrap_runtime_at_with_log_dir(
    database_path: impl AsRef<Path>,
    runtime_log_dir: impl AsRef<Path>,
) -> Result<RuntimeState> {
    let database_path = database_path.as_ref();
    let app_state = bootstrap_state_at(database_path)?;
    let runtime_log = RuntimeLogConfig {
        log_dir: runtime_log_dir.as_ref().to_path_buf(),
    };

    RuntimeState::start(app_state, runtime_log)
}

pub fn bootstrap_runtime_at(database_path: impl AsRef<Path>) -> Result<RuntimeState> {
    let database_path = database_path.as_ref();
    let app_local_data_dir = database_path
        .parent()
        .ok_or_else(|| CodexLagError::new("runtime database path has no parent directory"))?;
    let runtime_log_dir = runtime_log_dir(app_local_data_dir);
    bootstrap_runtime_at_with_log_dir(database_path, runtime_log_dir)
}

pub fn runtime_database_path(app_local_data_dir: impl AsRef<Path>) -> PathBuf {
    app_local_data_dir.as_ref().join("codexlag.sqlite3")
}

pub fn runtime_log_dir(app_local_data_dir: impl AsRef<Path>) -> PathBuf {
    app_local_data_dir.as_ref().join("logs")
}

pub async fn bootstrap_state_for_test() -> Result<AppState> {
    bootstrap_state_for_test_at(test_database_path()).await
}

pub async fn bootstrap_state_for_test_at(database_path: impl AsRef<Path>) -> Result<AppState> {
    let secret_namespace = format!("test/{}", database_path.as_ref().to_string_lossy());
    build_default_app_state(database_path, SecretStore::in_memory(secret_namespace))
}

pub async fn bootstrap_state_with_provider_inventory_for_test() -> Result<AppState> {
    bootstrap_state_with_provider_inventory_for_test_at(test_database_path()).await
}

pub async fn bootstrap_state_with_provider_inventory_for_test_at(
    database_path: impl AsRef<Path>,
) -> Result<AppState> {
    let mut app_state = bootstrap_state_for_test_at(&database_path).await?;
    let upstream_base_url = spawn_test_provider_upstream().await?;
    seed_runtime_provider_inventory_for_test(&mut app_state, upstream_base_url.as_str())?;
    Ok(app_state)
}

pub async fn bootstrap_runtime_for_test() -> Result<RuntimeState> {
    let database_path = test_database_path();
    let app_state = bootstrap_state_with_provider_inventory_for_test_at(&database_path).await?;
    let app_local_data_dir = database_path
        .parent()
        .ok_or_else(|| CodexLagError::new("runtime database path has no parent directory"))?;
    let runtime_log = RuntimeLogConfig {
        log_dir: runtime_log_dir(app_local_data_dir),
    };

    RuntimeState::start(app_state, runtime_log)
}

pub async fn bootstrap_openai_auth_runtime_for_test_at<R: OpenAiSessionRefresher>(
    database_path: impl AsRef<Path>,
    now_ms: i64,
    refresher: &R,
) -> Result<OpenAiAuthRuntime> {
    let app_state = bootstrap_state_for_test_at(&database_path).await?;
    let mut runtime = OpenAiAuthRuntime::new(app_state);
    let refreshable_accounts = runtime
        .list_sessions()
        .into_iter()
        .filter(|session| session.refreshable)
        .map(|session| session.account_id)
        .collect::<Vec<_>>();

    for account_id in refreshable_accounts {
        let _ = runtime.refresh_session_if_needed(account_id.as_str(), now_ms, refresher)?;
    }

    Ok(runtime)
}

fn test_database_path() -> PathBuf {
    std::env::temp_dir()
        .join("codexlag-tests")
        .join(random_suffix())
        .join("codexlag.sqlite3")
}

pub fn generate_platform_key_secret() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }

    format!("{DEFAULT_PLATFORM_KEY_SECRET_PREFIX}{encoded}")
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or_default()
}

fn random_suffix() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);

    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
}

async fn spawn_test_provider_upstream() -> Result<String> {
    async fn official_response() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "id": "resp_official_fixture",
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

    async fn relay_response() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "id": "chatcmpl_fixture",
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

    async fn relay_balance_response() -> Json<serde_json::Value> {
        Json(serde_json::json!({
            "data": {
                "total_balance": "25.00",
                "used_balance": "7.50"
            }
        }))
    }

    async fn badpayload_balance_response() -> &'static str {
        r#"{"data":{"total_balance":"25.00"}}"#
    }

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| CodexLagError::new(format!("failed to bind test provider upstream: {error}")))?;
    let address = listener
        .local_addr()
        .map_err(|error| {
            CodexLagError::new(format!(
                "failed to read test provider upstream address: {error}"
            ))
        })?;
    let router = Router::new()
        .route("/responses", post(official_response))
        .route("/v1/chat/completions", post(relay_response))
        .route("/v1/api/user/self", get(relay_balance_response))
        .route("/badpayload/v1/api/user/self", get(badpayload_balance_response));
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve test provider upstream");
    });

    Ok(format!("http://{address}"))
}

fn seed_runtime_provider_inventory_for_test(
    state: &mut AppState,
    upstream_base_url: &str,
) -> Result<()> {
    state.save_imported_official_account(default_primary_account())?;
    state.store_secret(
        &SecretKey::new(DEFAULT_OFFICIAL_SESSION_REF),
        "session-secret".to_string(),
    )?;
    state.store_secret(
        &SecretKey::new(DEFAULT_OFFICIAL_TOKEN_REF),
        serde_json::json!({
            "api_key": "official-live-key",
            "base_url": upstream_base_url,
        })
        .to_string(),
    )?;

    for mut relay in default_relays() {
        relay.endpoint = match relay.relay_id.as_str() {
            "relay-newapi" => format!("{upstream_base_url}/v1"),
            "relay-badpayload" => format!("{upstream_base_url}/badpayload/v1"),
            _ => relay.endpoint,
        };
        let relay_id = relay.relay_id.clone();
        let credential_ref = relay.api_key_credential_ref.clone();
        state.save_managed_relay(relay)?;
        state.store_secret(&SecretKey::new(credential_ref), format!("rk_local_{relay_id}"))?;
    }

    Ok(())
}
