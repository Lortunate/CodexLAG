use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{
    db::repositories::Repositories,
    error::Result,
    models::{PlatformKey, RoutingPolicy},
    routing::policy::HYBRID,
    secret_store::{SecretKey, SecretStore},
    state::{AppState, RuntimeState},
};

const DEFAULT_POLICY_ID: &str = "policy-default";
const DEFAULT_POLICY_NAME: &str = "default";
const DEFAULT_PLATFORM_KEY_ID: &str = "key-default";
const DEFAULT_PLATFORM_KEY_NAME: &str = "default";
const DEFAULT_PLATFORM_KEY_SECRET_SEED: &str = "ck_local_default_seed";

fn build_default_app_state(
    database_path: impl AsRef<Path>,
    default_platform_key_secret: &str,
) -> Result<AppState> {
    let mut repositories = Repositories::open(database_path)?;

    let default_policy = RoutingPolicy {
        id: DEFAULT_POLICY_ID.into(),
        name: DEFAULT_POLICY_NAME.into(),
    };

    let default_key = PlatformKey {
        id: DEFAULT_PLATFORM_KEY_ID.into(),
        name: DEFAULT_PLATFORM_KEY_NAME.into(),
        allowed_mode: HYBRID.into(),
        policy_id: default_policy.id.clone(),
        enabled: true,
    };

    let default_key_secret = SecretKey::platform_key(default_key.id.clone());

    if repositories.policy(DEFAULT_POLICY_NAME).is_none() {
        repositories.insert_policy(default_policy)?;
    }

    if repositories.platform_key(DEFAULT_PLATFORM_KEY_NAME).is_none() {
        repositories.insert_platform_key(default_key)?;
    }

    let mut secret_store = SecretStore::default();
    secret_store.set(&default_key_secret, default_platform_key_secret.into())?;

    Ok(AppState::new(repositories, secret_store))
}

pub fn bootstrap_state() -> Result<AppState> {
    build_default_app_state(default_database_path(), DEFAULT_PLATFORM_KEY_SECRET_SEED)
}

pub fn bootstrap_runtime() -> Result<RuntimeState> {
    bootstrap_state().map(RuntimeState::new)
}

pub async fn bootstrap_state_for_test() -> Result<AppState> {
    bootstrap_state_for_test_at(test_database_path()).await
}

pub async fn bootstrap_state_for_test_at(database_path: impl AsRef<Path>) -> Result<AppState> {
    build_default_app_state(database_path, DEFAULT_PLATFORM_KEY_SECRET_SEED)
}

pub async fn bootstrap_runtime_for_test() -> Result<RuntimeState> {
    bootstrap_state_for_test().await.map(RuntimeState::new)
}

fn default_database_path() -> PathBuf {
    std::env::temp_dir().join("codexlag").join("codexlag.sqlite3")
}

fn test_database_path() -> PathBuf {
    let unique_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();

    std::env::temp_dir()
        .join("codexlag-tests")
        .join(format!("codexlag-{unique_suffix}.sqlite3"))
}
