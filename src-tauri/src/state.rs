use std::{
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::SystemTime,
};

use crate::db::repositories::{Repositories, UsageCostEstimate};
use crate::error::CodexLagError;
use crate::gateway::{
    host::{GatewayHost, GatewayHostStatus, LOOPBACK_BIND_ADDR},
    server::LoopbackGateway,
};
use crate::logging::usage::{append_usage_record, UsageRecord, UsageRecordInput};
use crate::models::{
    ImportedOfficialAccount, ManagedRelay, PlatformKey, ProviderSessionSummary, RoutingPolicy,
};
use crate::routing::policy::RoutingMode;
use crate::secret_store::{SecretKey, SecretStore};
use crate::tray::{build_tray_model_for_runtime, TrayModel};

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

    pub fn store_secret(&self, key: &SecretKey, value: String) -> crate::error::Result<()> {
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

    pub fn get_platform_key_by_id(&self, id: &str) -> Option<&PlatformKey> {
        self.repositories.platform_key_by_id(id)
    }

    pub fn insert_platform_key(&mut self, key: PlatformKey) -> crate::error::Result<()> {
        self.repositories.insert_platform_key(key)
    }

    pub fn set_platform_key_enabled_by_id(
        &mut self,
        key_id: &str,
        enabled: bool,
    ) -> crate::error::Result<()> {
        self.repositories
            .update_platform_key_enabled_by_id(key_id, enabled)
    }

    pub fn save_policy(&mut self, policy: RoutingPolicy) -> crate::error::Result<()> {
        self.repositories.save_policy(policy)
    }

    pub fn imported_official_account(&self, account_id: &str) -> Option<&ImportedOfficialAccount> {
        self.repositories.imported_official_account(account_id)
    }

    pub fn iter_imported_official_accounts(
        &self,
    ) -> impl Iterator<Item = &ImportedOfficialAccount> {
        self.repositories.iter_imported_official_accounts()
    }

    pub fn iter_provider_sessions(&self) -> impl Iterator<Item = &ProviderSessionSummary> {
        self.repositories.iter_provider_sessions()
    }

    pub fn save_imported_official_account(
        &mut self,
        account: ImportedOfficialAccount,
    ) -> crate::error::Result<()> {
        self.repositories.save_imported_official_account(account)
    }

    pub fn save_provider_session(
        &mut self,
        session: ProviderSessionSummary,
    ) -> crate::error::Result<()> {
        self.repositories.save_provider_session(session)
    }

    pub fn managed_relay(&self, relay_id: &str) -> Option<&ManagedRelay> {
        self.repositories.managed_relay(relay_id)
    }

    pub fn iter_managed_relays(&self) -> impl Iterator<Item = &ManagedRelay> {
        self.repositories.iter_managed_relays()
    }

    pub fn save_managed_relay(&mut self, relay: ManagedRelay) -> crate::error::Result<()> {
        self.repositories.save_managed_relay(relay)
    }

    pub fn delete_managed_relay(&mut self, relay_id: &str) -> crate::error::Result<bool> {
        self.repositories.delete_managed_relay(relay_id)
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

    pub fn estimate_usage_cost_for_model_at(
        &self,
        model: &str,
        at_ms: i64,
        input_tokens: u32,
        output_tokens: u32,
        cache_read_tokens: u32,
        cache_write_tokens: u32,
        reasoning_tokens: u32,
    ) -> crate::error::Result<Option<UsageCostEstimate>> {
        self.repositories.estimate_usage_cost_for_model_at(
            model,
            at_ms,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            reasoning_tokens,
        )
    }

    pub fn active_pricing_profile_id_for_model_at(
        &self,
        model: &str,
        at_ms: i64,
    ) -> crate::error::Result<Option<String>> {
        Ok(self
            .repositories
            .active_pricing_profile_by_model(model, at_ms)?
            .map(|profile| profile.id))
    }

    pub fn repositories(&self) -> &Repositories {
        &self.repositories
    }

    pub fn repositories_mut(&mut self) -> &mut Repositories {
        &mut self.repositories
    }
}

#[derive(Clone)]
pub struct RuntimeLogConfig {
    pub log_dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeLogFileMetadata {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: u64,
}

impl RuntimeLogConfig {
    pub fn recent_log_files(
        &self,
        max_files: usize,
    ) -> std::io::Result<Vec<RuntimeLogFileMetadata>> {
        if max_files == 0 {
            return Ok(Vec::new());
        }

        let entries = match std::fs::read_dir(&self.log_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };

        let mut files = Vec::new();
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_) => continue,
            };
            if !file_type.is_file() {
                continue;
            }

            let name = entry.file_name().to_string_lossy().to_string();
            if !is_runtime_log_file_name(&name) {
                continue;
            }

            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
                .unwrap_or(0);

            files.push(RuntimeLogFileMetadata {
                name,
                path: entry.path(),
                size: metadata.len(),
                mtime,
            });
        }

        files.sort_by(|left, right| {
            right
                .mtime
                .cmp(&left.mtime)
                .then_with(|| left.name.cmp(&right.name))
        });
        if files.len() > max_files {
            files.truncate(max_files);
        }

        Ok(files)
    }
}

fn is_runtime_log_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    if file_name.ends_with(".log") {
        return true;
    }

    if let Some((_, suffix)) = file_name.split_once(".log.") {
        return !suffix.is_empty();
    }

    false
}

#[derive(Clone)]
pub struct RuntimeState {
    app_state: Arc<RwLock<AppState>>,
    usage_records: Arc<RwLock<Vec<UsageRecord>>>,
    loopback_gateway: Arc<RwLock<LoopbackGateway>>,
    gateway_host: Arc<RwLock<Option<GatewayHost>>>,
    runtime_log: RuntimeLogConfig,
    last_balance_refresh: Arc<RwLock<Option<String>>>,
    last_restart_feedback: Arc<RwLock<Option<String>>>,
}

impl RuntimeState {
    pub fn new(app_state: AppState, runtime_log: RuntimeLogConfig) -> Self {
        let app_state = Arc::new(RwLock::new(app_state));
        let usage_records = Arc::new(RwLock::new(Vec::new()));
        let loopback_gateway =
            LoopbackGateway::new(Arc::clone(&app_state), Arc::clone(&usage_records));

        Self {
            app_state,
            usage_records,
            loopback_gateway: Arc::new(RwLock::new(loopback_gateway)),
            gateway_host: Arc::new(RwLock::new(None)),
            runtime_log,
            last_balance_refresh: Arc::new(RwLock::new(None)),
            last_restart_feedback: Arc::new(RwLock::new(None)),
        }
    }

    pub fn start(app_state: AppState, runtime_log: RuntimeLogConfig) -> crate::error::Result<Self> {
        let runtime = Self::new(app_state, runtime_log);
        let host = GatewayHost::start(runtime.loopback_gateway().router())?;
        runtime.set_gateway_host(Some(host))?;
        Ok(runtime)
    }

    pub fn app_state(&self) -> RwLockReadGuard<'_, AppState> {
        self.app_state
            .read()
            .expect("runtime app state lock poisoned")
    }

    pub fn app_state_mut(&self) -> RwLockWriteGuard<'_, AppState> {
        self.app_state
            .write()
            .expect("runtime app state lock poisoned")
    }

    pub fn loopback_gateway(&self) -> LoopbackGateway {
        self.loopback_gateway
            .read()
            .expect("runtime loopback gateway lock poisoned")
            .clone()
    }

    pub fn runtime_log(&self) -> &RuntimeLogConfig {
        &self.runtime_log
    }

    pub fn usage_records(&self) -> Vec<UsageRecord> {
        self.usage_records
            .read()
            .expect("runtime usage records lock poisoned")
            .clone()
    }

    pub fn record_usage_request(&self, input: UsageRecordInput) {
        let mut records = self
            .usage_records
            .write()
            .expect("runtime usage records lock poisoned");
        append_usage_record(&mut records, input);
    }

    pub fn gateway_host_status(&self) -> GatewayHostStatus {
        self.gateway_host
            .read()
            .ok()
            .and_then(|host| host.as_ref().map(GatewayHost::status))
            .unwrap_or(GatewayHostStatus {
                is_running: false,
                listen_addr: LOOPBACK_BIND_ADDR,
            })
    }

    pub fn tray_model(&self) -> TrayModel {
        build_tray_model_for_runtime(self)
    }

    pub fn current_mode(&self) -> RoutingMode {
        self.app_state()
            .current_mode()
            .unwrap_or(RoutingMode::Hybrid)
    }

    pub fn set_current_mode(&self, mode: RoutingMode) -> crate::error::Result<()> {
        self.app_state_mut().set_default_key_allowed_mode(mode)
    }

    pub fn list_provider_sessions(&self) -> crate::error::Result<Vec<ProviderSessionSummary>> {
        let mut sessions = self
            .app_state()
            .iter_provider_sessions()
            .cloned()
            .collect::<Vec<_>>();
        sessions.sort_by(|left, right| {
            left.provider_id
                .cmp(&right.provider_id)
                .then_with(|| left.account_id.cmp(&right.account_id))
        });
        Ok(sessions)
    }

    pub fn rebuild_gateway_candidates(&self) -> crate::error::Result<()> {
        let candidates = {
            let state = self.app_state.read().map_err(|_| {
                CodexLagError::new("Failed to rebuild loopback gateway candidates.")
                    .with_internal_context(
                        "operation=rebuild_gateway_candidates;cause=app_state_lock_poisoned",
                    )
            })?;
            crate::gateway::server::build_candidates_from_state(&state)
        };
        let next_gateway = LoopbackGateway::new_with_runtime(
            Arc::clone(&self.app_state),
            Arc::clone(&self.usage_records),
            candidates,
        );
        let mut gateway = self.loopback_gateway.write().map_err(|_| {
            CodexLagError::new("Failed to rebuild loopback gateway candidates.")
                .with_internal_context(
                    "operation=rebuild_gateway_candidates;cause=loopback_lock_poisoned",
                )
        })?;
        *gateway = next_gateway;
        Ok(())
    }

    pub fn on_inventory_changed(&self) -> crate::error::Result<()> {
        if self.gateway_host_status().is_running {
            self.restart_gateway()?;
            return Ok(());
        }

        self.rebuild_gateway_candidates()
    }

    pub fn record_balance_refresh_summary(&self, summary: String) {
        if let Ok(mut last_balance_refresh) = self.last_balance_refresh.write() {
            *last_balance_refresh = Some(summary);
        }
    }

    pub fn record_restart_feedback(&self, summary: String) {
        if let Ok(mut last_restart_feedback) = self.last_restart_feedback.write() {
            *last_restart_feedback = Some(summary);
        }
    }

    pub fn last_balance_refresh_summary(&self) -> Option<String> {
        self.last_balance_refresh
            .read()
            .ok()
            .and_then(|summary| summary.clone())
    }

    pub fn last_restart_feedback(&self) -> Option<String> {
        self.last_restart_feedback
            .read()
            .ok()
            .and_then(|summary| summary.clone())
    }

    pub fn restart_gateway(&self) -> crate::error::Result<()> {
        let replacement =
            LoopbackGateway::new(Arc::clone(&self.app_state), Arc::clone(&self.usage_records));
        let router = replacement.router();

        let existing_host = {
            let mut host = match self.gateway_host.write() {
                Ok(host) => host,
                Err(_) => {
                    self.record_restart_feedback("failed".to_string());
                    return Err(
                        CodexLagError::new("Failed to restart loopback gateway host.")
                            .with_internal_context(
                                "operation=restart_gateway;cause=gateway_host_lock_poisoned",
                            ),
                    );
                }
            };
            host.take()
        };
        if let Some(existing_host) = existing_host {
            if let Err(error) = existing_host.shutdown() {
                self.record_restart_feedback("failed".to_string());
                return Err(
                    error.with_internal_context("operation=restart_gateway;cause=shutdown_failed")
                );
            }
        }

        let restarted_host = match GatewayHost::start(router) {
            Ok(host) => host,
            Err(error) => {
                if let Ok(fallback_host) = GatewayHost::start(self.loopback_gateway().router()) {
                    let _ = self.set_gateway_host(Some(fallback_host));
                }
                self.record_restart_feedback("failed".to_string());
                return Err(
                    error.with_internal_context("operation=restart_gateway;cause=start_failed")
                );
            }
        };
        {
            let mut gateway = match self.loopback_gateway.write() {
                Ok(gateway) => gateway,
                Err(_) => {
                    self.record_restart_feedback("failed".to_string());
                    return Err(CodexLagError::new("Failed to restart loopback gateway.")
                        .with_internal_context(
                            "operation=restart_gateway;cause=loopback_lock_poisoned",
                        ));
                }
            };
            *gateway = replacement;
        }
        self.set_gateway_host(Some(restarted_host))?;

        self.record_restart_feedback("ok".to_string());
        Ok(())
    }

    fn set_gateway_host(&self, host: Option<GatewayHost>) -> crate::error::Result<()> {
        let mut gateway_host = self.gateway_host.write().map_err(|_| {
            CodexLagError::new("Failed to update loopback gateway host.")
                .with_internal_context("operation=set_gateway_host;cause=lock_poisoned")
        })?;
        *gateway_host = host;
        Ok(())
    }
}
