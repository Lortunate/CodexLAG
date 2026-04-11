use std::sync::{Arc, RwLock, RwLockReadGuard};

use crate::db::repositories::Repositories;
use crate::gateway::server::LoopbackGateway;
use crate::models::{PlatformKey, RoutingPolicy};
use crate::routing::policy::RoutingMode;
use crate::secret_store::{SecretKey, SecretStore};
use crate::tray::{build_tray_model_for_state, TrayModel};

pub struct AppState {
    repositories: Repositories,
    secret_store: SecretStore,
}

impl AppState {
    pub fn new(repositories: Repositories, secret_store: SecretStore) -> Self {
        Self {
            repositories,
            secret_store,
        }
    }

    pub fn store_secret(&mut self, key: &SecretKey, value: String) -> crate::error::Result<()> {
        self.secret_store.set(key, value)
    }

    pub fn secret(&self, key: &SecretKey) -> crate::error::Result<String> {
        self.secret_store.get(key)
    }

    pub fn get_policy_by_name(&self, name: &str) -> Option<&RoutingPolicy> {
        self.repositories.policy(name)
    }

    pub fn get_policy_by_id(&self, id: &str) -> Option<&RoutingPolicy> {
        self.iter_policies().find(|policy| policy.id == id)
    }

    pub fn get_platform_key_by_name(&self, name: &str) -> Option<&PlatformKey> {
        self.repositories.platform_key(name)
    }

    pub fn default_policy(&self) -> Option<&RoutingPolicy> {
        self.get_policy_by_name("default")
    }

    pub fn default_platform_key(&self) -> Option<&PlatformKey> {
        self.get_platform_key_by_name("default")
    }

    pub fn current_mode(&self) -> Option<RoutingMode> {
        self.default_platform_key()
            .and_then(|key| RoutingMode::parse(key.allowed_mode()))
    }

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.repositories.iter_policies()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.repositories.iter_platform_keys()
    }

    pub fn authenticate_platform_key(&self, provided_secret: &str) -> Option<PlatformKey> {
        self.iter_platform_keys()
            .find(|key| {
                key.enabled
                    && self
                        .secret(&SecretKey::platform_key(key.id.clone()))
                        .is_ok_and(|stored_secret| stored_secret == provided_secret)
            })
            .cloned()
    }

    pub fn set_default_key_allowed_mode(&mut self, mode: RoutingMode) -> crate::error::Result<()> {
        self.repositories
            .update_platform_key_allowed_mode("default", mode.as_str())
    }
}

#[derive(Clone)]
pub struct RuntimeState {
    app_state: Arc<RwLock<AppState>>,
    loopback_gateway: LoopbackGateway,
}

impl RuntimeState {
    pub fn new(app_state: AppState) -> Self {
        let app_state = Arc::new(RwLock::new(app_state));
        let loopback_gateway = LoopbackGateway::new(Arc::clone(&app_state));

        Self {
            app_state,
            loopback_gateway,
        }
    }

    pub fn app_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.app_state.read().expect("runtime app state lock poisoned")
    }

    pub fn loopback_gateway(&self) -> &LoopbackGateway {
        &self.loopback_gateway
    }

    pub fn tray_model(&self) -> TrayModel {
        build_tray_model_for_state(&self.app_state())
    }

    pub fn current_mode(&self) -> RoutingMode {
        self.app_state().current_mode().unwrap_or(RoutingMode::Hybrid)
    }

    pub fn set_current_mode(&self, mode: RoutingMode) -> crate::error::Result<()> {
        self.app_state
            .write()
            .expect("runtime app state lock poisoned")
            .set_default_key_allowed_mode(mode)
    }
}
