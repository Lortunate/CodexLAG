use std::sync::Arc;

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
}

#[derive(Clone)]
pub struct RuntimeState {
    app_state: Arc<AppState>,
    loopback_gateway: LoopbackGateway,
    tray_model: TrayModel,
}

impl RuntimeState {
    pub fn new(app_state: AppState) -> Self {
        let app_state = Arc::new(app_state);
        let loopback_gateway = LoopbackGateway::new(Arc::clone(&app_state));
        let tray_model = build_tray_model_for_state(app_state.as_ref());

        Self {
            app_state,
            loopback_gateway,
            tray_model,
        }
    }

    pub fn app_state(&self) -> &AppState {
        self.app_state.as_ref()
    }

    pub fn loopback_gateway(&self) -> &LoopbackGateway {
        &self.loopback_gateway
    }

    pub fn tray_model(&self) -> &TrayModel {
        &self.tray_model
    }

    pub fn current_mode(&self) -> RoutingMode {
        self.tray_model()
            .current_mode()
            .unwrap_or(RoutingMode::Hybrid)
    }
}
