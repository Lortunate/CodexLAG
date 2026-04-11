use crate::models::{PlatformKey, RoutingPolicy};
use std::collections::HashMap;

#[derive(Default)]
pub struct Repositories {
    policies: HashMap<String, RoutingPolicy>,
    keys: HashMap<String, PlatformKey>,
}

impl Repositories {
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
            keys: HashMap::new(),
        }
    }

    pub fn insert_policy(&mut self, policy: RoutingPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    pub fn insert_platform_key(&mut self, key: PlatformKey) {
        self.keys.insert(key.name.clone(), key);
    }

    pub fn policy(&self, name: &str) -> Option<&RoutingPolicy> {
        self.policies.get(name)
    }

    pub fn platform_key(&self, name: &str) -> Option<&PlatformKey> {
        self.keys.get(name)
    }

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.policies.values()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.keys.values()
    }
}
