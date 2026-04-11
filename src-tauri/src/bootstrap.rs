use crate::{
    db::repositories::Repositories,
    error::Result,
    models::{PlatformKey, RoutingPolicy},
    state::AppState,
};

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

    repositories.insert_policy(default_policy);
    repositories.insert_platform_key(default_key);

    Ok(AppState::new(repositories))
}
