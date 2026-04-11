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

fn build_default_app_state(default_platform_key_secret: &str) -> Result<AppState> {
    let mut repositories = Repositories::new();

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

    repositories.insert_policy(default_policy)?;
    repositories.insert_platform_key(default_key)?;

    let mut secret_store = SecretStore::default();
    secret_store.set(&default_key_secret, default_platform_key_secret.into())?;

    Ok(AppState::new(repositories, secret_store))
}

pub fn bootstrap_state() -> Result<AppState> {
    build_default_app_state(DEFAULT_PLATFORM_KEY_SECRET_SEED)
}

pub fn bootstrap_runtime() -> Result<RuntimeState> {
    bootstrap_state().map(RuntimeState::new)
}

pub async fn bootstrap_state_for_test() -> Result<AppState> {
    bootstrap_state()
}

pub async fn bootstrap_runtime_for_test() -> Result<RuntimeState> {
    bootstrap_runtime()
}
