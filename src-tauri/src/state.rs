use std::{
    path::PathBuf,
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
    time::SystemTime,
};

use crate::db::repositories::{Repositories, UsageCostEstimate};
use crate::gateway::server::LoopbackGateway;
use crate::logging::usage::{append_usage_record, UsageRecord, UsageRecordInput};
use crate::models::{ImportedOfficialAccount, ManagedRelay, PlatformKey, RoutingPolicy};
use crate::routing::policy::RoutingMode;
use crate::secret_store::{SecretKey, SecretStore};
use crate::tray::{build_tray_model, TrayModel};

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

    pub fn save_imported_official_account(
        &mut self,
        account: ImportedOfficialAccount,
    ) -> crate::error::Result<()> {
        self.repositories.save_imported_official_account(account)
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
    loopback_gateway: LoopbackGateway,
    runtime_log: RuntimeLogConfig,
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
            loopback_gateway,
            runtime_log,
        }
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

    pub fn loopback_gateway(&self) -> &LoopbackGateway {
        &self.loopback_gateway
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

    pub fn tray_model(&self) -> TrayModel {
        let mode = self
            .app_state()
            .current_mode()
            .unwrap_or(RoutingMode::Hybrid);
        let unavailable_reason = self
            .loopback_gateway
            .state()
            .unavailable_reason_for_mode(mode.as_str());
        build_tray_model(mode, unavailable_reason)
    }

    pub fn current_mode(&self) -> RoutingMode {
        self.app_state()
            .current_mode()
            .unwrap_or(RoutingMode::Hybrid)
    }

    pub fn set_current_mode(&self, mode: RoutingMode) -> crate::error::Result<()> {
        self.app_state_mut().set_default_key_allowed_mode(mode)
    }
}
