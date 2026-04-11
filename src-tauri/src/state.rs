use crate::db::repositories::Repositories;
use crate::models::{PlatformKey, RoutingPolicy};

pub struct AppState {
    repositories: Repositories,
}

impl AppState {
    pub fn new(repositories: Repositories) -> Self {
        Self { repositories }
    }

    pub fn get_policy_by_name(&self, name: &str) -> Option<&RoutingPolicy> {
        self.repositories.policy(name)
    }

    pub fn get_platform_key_by_name(&self, name: &str) -> Option<&PlatformKey> {
        self.repositories.platform_key(name)
    }

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.repositories.iter_policies()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.repositories.iter_platform_keys()
    }
}
