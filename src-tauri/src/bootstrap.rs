use crate::{
    db::repositories::Repositories,
    error::Result,
    models::{PlatformKey, RoutingPolicy},
    secret_store::{SecretKey, SecretStore},
    state::AppState,
};

const TEST_DEFAULT_PLATFORM_KEY_SECRET_SEED: &str = "ck_local_default_seed";

pub async fn bootstrap_state_for_test() -> Result<AppState> {
    let mut repositories = Repositories::new();

    let default_policy = RoutingPolicy {
        id: "policy-default".into(),
        name: "default".into(),
    };

    let default_key = PlatformKey {
        id: "key-default".into(),
        name: "default".into(),
        allowed_mode: "hybrid".into(),
        policy_id: default_policy.id.clone(),
        enabled: true,
    };

    repositories.insert_policy(default_policy)?;
    repositories.insert_platform_key(default_key)?;

    let mut secret_store = SecretStore::default();
    secret_store.set(
        &SecretKey::default_platform_key(),
        TEST_DEFAULT_PLATFORM_KEY_SECRET_SEED.into(),
    )?;

    Ok(AppState::new(repositories, secret_store))
}
