use serde::{Deserialize, Serialize};

pub use crate::providers::capabilities::{FeatureCapability, FeatureCapabilityPatch};
pub use crate::providers::official::{OfficialAuthMode, OfficialSession};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
}

impl PlatformKey {
    pub fn allowed_mode(&self) -> &str {
        self.allowed_mode.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub id: String,
    pub name: String,
}
