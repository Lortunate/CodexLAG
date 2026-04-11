use crate::{
    db::repositories::Repositories,
    models::{PlatformKey, RoutingPolicy},
};

pub struct AppStateForTest {
    pub db: Repositories,
}

pub async fn bootstrap_state_for_test() -> Result<AppStateForTest, String> {
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

    Ok(AppStateForTest {
        db: Repositories {
            policies: vec![default_policy],
            keys: vec![default_key],
        },
    })
}
