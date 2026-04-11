use crate::models::{PlatformKey, RoutingPolicy};

#[derive(Default)]
pub struct Repositories {
    pub policies: Vec<RoutingPolicy>,
    pub keys: Vec<PlatformKey>,
}

impl Repositories {
    pub fn get_policy_by_name(&self, name: &str) -> Option<RoutingPolicy> {
        self.policies.iter().find(|item| item.name == name).cloned()
    }

    pub fn get_platform_key_by_name(&self, name: &str) -> Option<PlatformKey> {
        self.keys.iter().find(|item| item.name == name).cloned()
    }
}
