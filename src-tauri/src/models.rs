use serde::{Deserialize, Serialize};

pub const DEFAULT_PLATFORM_KEY_SECRET: &str = "ck_local_default_seed";
pub const DEFAULT_PLATFORM_KEY_SECRET_PREFIX: &str = "ck_local_";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub id: String,
    pub name: String,
}
