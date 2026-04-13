use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, Connection, Transaction};
use serde::{de::DeserializeOwned, Serialize};

use crate::db::migrations::ensure_schema_up_to_date;
use crate::error::{CodexLagError, Result};
use crate::logging::usage::UsageProvenance;
use crate::models::{
    CredentialKind, ImportedOfficialAccount, ManagedRelay, PlatformKey, PricingProfile,
    RequestAttemptLog, RequestLog, RoutingPolicy,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageCostEstimate {
    pub amount: String,
    pub currency: String,
    pub provenance: UsageProvenance,
    pub estimated: bool,
    pub pricing_profile_id: String,
}

pub struct Repositories {
    database_path: PathBuf,
    policies: HashMap<String, RoutingPolicy>,
    keys: HashMap<String, PlatformKey>,
    imported_official_accounts: HashMap<String, ImportedOfficialAccount>,
    managed_relays: HashMap<String, ManagedRelay>,
}

impl Repositories {
    pub fn open(database_path: impl AsRef<Path>) -> Result<Self> {
        let database_path = database_path.as_ref().to_path_buf();

        if let Some(parent) = database_path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to create database directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }

        let connection = Self::open_sqlite(&database_path)?;
        ensure_schema_up_to_date(&connection)?;

        let policies = Self::load_policies(&connection)?;
        let keys = Self::load_platform_keys(&connection)?;
        let imported_official_accounts = Self::load_imported_official_accounts(&connection)?;
        let managed_relays = Self::load_managed_relays(&connection)?;

        Ok(Self {
            database_path,
            policies,
            keys,
            imported_official_accounts,
            managed_relays,
        })
    }

    pub fn insert_policy(&mut self, policy: RoutingPolicy) -> Result<()> {
        let name = policy.name.clone();

        if self.policies.contains_key(&name) {
            return Err(CodexLagError::new(format!(
                "policy '{}' already exists",
                name
            )));
        }

        self.save_policy(policy)
    }

    pub fn save_policy(&mut self, policy: RoutingPolicy) -> Result<()> {
        let selection_order = encode_json(&policy.selection_order, "selection_order")?;
        let failure_rules = encode_json(&policy.failure_rules, "failure_rules")?;
        let recovery_rules = encode_json(&policy.recovery_rules, "recovery_rules")?;
        let connection = self.open_connection()?;

        connection
            .execute(
                "
                INSERT INTO routing_policies (
                    id,
                    name,
                    selection_order,
                    cross_pool_fallback,
                    retry_budget,
                    failure_rules,
                    recovery_rules
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    selection_order = excluded.selection_order,
                    cross_pool_fallback = excluded.cross_pool_fallback,
                    retry_budget = excluded.retry_budget,
                    failure_rules = excluded.failure_rules,
                    recovery_rules = excluded.recovery_rules
                ",
                params![
                    &policy.id,
                    &policy.name,
                    selection_order,
                    policy.cross_pool_fallback as i64,
                    i64::from(policy.retry_budget),
                    failure_rules,
                    recovery_rules
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist policy '{}': {error}",
                    policy.name
                ))
            })?;

        let policy_id = policy.id.clone();
        let policy_name = policy.name.clone();

        self.policies
            .retain(|name, cached_policy| cached_policy.id != policy_id || name == &policy_name);
        self.policies.insert(policy_name, policy);
        Ok(())
    }

    pub fn insert_platform_key(&mut self, key: PlatformKey) -> Result<()> {
        let name = key.name.clone();
        let id = key.id.clone();

        if self.keys.contains_key(&name) || self.keys.values().any(|existing| existing.id == id) {
            return Err(CodexLagError::new(format!(
                "platform key '{}' already exists",
                id
            )));
        }

        let connection = self.open_connection()?;

        connection
            .execute(
                "INSERT INTO platform_keys (id, name, allowed_mode, policy_id, enabled) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    &key.id,
                    &key.name,
                    &key.allowed_mode,
                    &key.policy_id,
                    key.enabled as i64
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist platform key '{}': {error}",
                    name
                ))
            })?;

        self.keys.insert(name, key);
        Ok(())
    }

    pub fn policy(&self, name: &str) -> Option<&RoutingPolicy> {
        self.policies.get(name)
    }

    pub fn platform_key(&self, name: &str) -> Option<&PlatformKey> {
        self.keys.get(name)
    }

    pub fn platform_key_by_id(&self, id: &str) -> Option<&PlatformKey> {
        self.keys.values().find(|key| key.id == id)
    }

    pub fn update_platform_key_allowed_mode(
        &mut self,
        name: &str,
        allowed_mode: &str,
    ) -> Result<()> {
        if !self.keys.contains_key(name) {
            return Err(CodexLagError::new(format!(
                "platform key '{}' not found",
                name
            )));
        }

        let connection = self.open_connection()?;

        connection
            .execute(
                "UPDATE platform_keys SET allowed_mode = ?1 WHERE name = ?2",
                params![allowed_mode, name],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to update platform key '{}' mode: {error}",
                    name
                ))
            })?;

        let key = self
            .keys
            .get_mut(name)
            .expect("platform key existence checked before update");
        key.allowed_mode = allowed_mode.into();
        Ok(())
    }

    pub fn update_platform_key_enabled_by_id(&mut self, key_id: &str, enabled: bool) -> Result<()> {
        let Some((name, _)) = self
            .keys
            .iter()
            .find(|(_, key)| key.id == key_id)
            .map(|(name, key)| (name.clone(), key.clone()))
        else {
            return Err(CodexLagError::new(format!(
                "platform key '{}' not found",
                key_id
            )));
        };

        let connection = self.open_connection()?;
        connection
            .execute(
                "UPDATE platform_keys SET enabled = ?1 WHERE id = ?2",
                params![enabled as i64, key_id],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to update platform key '{}' enabled state: {error}",
                    key_id
                ))
            })?;

        let key = self
            .keys
            .get_mut(&name)
            .expect("platform key existence checked before enabled update");
        key.enabled = enabled;
        Ok(())
    }

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.policies.values()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.keys.values()
    }

    pub fn save_imported_official_account(
        &mut self,
        account: ImportedOfficialAccount,
    ) -> Result<()> {
        let session_payload = encode_json(&account.session, "session_payload")?;
        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|error| {
            CodexLagError::new(format!(
                "failed to begin imported official account transaction for '{}': {error}",
                account.account_id
            ))
        })?;

        transaction
            .execute(
                "
                INSERT INTO imported_official_accounts (
                    account_id,
                    name,
                    provider,
                    session_payload,
                    session_credential_ref,
                    token_credential_ref
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                ON CONFLICT(account_id) DO UPDATE SET
                    name = excluded.name,
                    provider = excluded.provider,
                    session_payload = excluded.session_payload,
                    session_credential_ref = excluded.session_credential_ref,
                    token_credential_ref = excluded.token_credential_ref
                ",
                params![
                    &account.account_id,
                    &account.name,
                    &account.provider,
                    session_payload,
                    &account.session_credential_ref,
                    &account.token_credential_ref
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist imported official account '{}': {error}",
                    account.account_id
                ))
            })?;

        Self::upsert_credential_ref(
            &transaction,
            account.session_credential_ref.as_str(),
            account.account_id.as_str(),
            CredentialKind::OfficialSession,
        )?;
        Self::upsert_credential_ref(
            &transaction,
            account.token_credential_ref.as_str(),
            account.account_id.as_str(),
            CredentialKind::OfficialSession,
        )?;
        transaction.commit().map_err(|error| {
            CodexLagError::new(format!(
                "failed to commit imported official account transaction for '{}': {error}",
                account.account_id
            ))
        })?;

        self.imported_official_accounts
            .insert(account.account_id.clone(), account);
        Ok(())
    }

    pub fn imported_official_account(&self, account_id: &str) -> Option<&ImportedOfficialAccount> {
        self.imported_official_accounts.get(account_id)
    }

    pub fn iter_imported_official_accounts(
        &self,
    ) -> impl Iterator<Item = &ImportedOfficialAccount> {
        self.imported_official_accounts.values()
    }

    pub fn save_managed_relay(&mut self, relay: ManagedRelay) -> Result<()> {
        let adapter = encode_json(&relay.adapter, "relay_adapter")?;
        let connection = self.open_connection()?;
        connection
            .execute(
                "
                INSERT INTO managed_relays (
                    relay_id,
                    name,
                    endpoint,
                    adapter
                )
                VALUES (?1, ?2, ?3, ?4)
                ON CONFLICT(relay_id) DO UPDATE SET
                    name = excluded.name,
                    endpoint = excluded.endpoint,
                    adapter = excluded.adapter
                ",
                params![&relay.relay_id, &relay.name, &relay.endpoint, adapter],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist relay '{}': {error}",
                    relay.relay_id
                ))
            })?;

        self.managed_relays.insert(relay.relay_id.clone(), relay);
        Ok(())
    }

    pub fn delete_managed_relay(&mut self, relay_id: &str) -> Result<bool> {
        if !self.managed_relays.contains_key(relay_id) {
            return Ok(false);
        }

        let connection = self.open_connection()?;
        connection
            .execute(
                "DELETE FROM managed_relays WHERE relay_id = ?1",
                params![relay_id],
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to delete relay '{}': {error}", relay_id))
            })?;

        self.managed_relays.remove(relay_id);
        Ok(true)
    }

    pub fn managed_relay(&self, relay_id: &str) -> Option<&ManagedRelay> {
        self.managed_relays.get(relay_id)
    }

    pub fn iter_managed_relays(&self) -> impl Iterator<Item = &ManagedRelay> {
        self.managed_relays.values()
    }

    pub fn append_request_with_attempts(
        &self,
        request: &RequestLog,
        attempts: &[RequestAttemptLog],
    ) -> Result<()> {
        let expected_attempts = usize::try_from(request.attempt_count).map_err(|_| {
            CodexLagError::new(format!(
                "request '{}' attempt_count '{}' is not representable as usize",
                request.request_id, request.attempt_count
            ))
        })?;
        if expected_attempts != attempts.len() {
            return Err(CodexLagError::new(format!(
                "request '{}' attempt_count {} does not match provided attempts length {}",
                request.request_id,
                request.attempt_count,
                attempts.len()
            )));
        }

        for attempt in attempts {
            if attempt.request_id != request.request_id {
                return Err(CodexLagError::new(format!(
                    "attempt '{}' has request_id '{}' but expected '{}'",
                    attempt.attempt_id, attempt.request_id, request.request_id
                )));
            }
        }

        let mut connection = self.open_connection()?;
        let transaction = connection.transaction().map_err(|error| {
            CodexLagError::new(format!(
                "failed to begin request log transaction for '{}': {error}",
                request.request_id
            ))
        })?;

        transaction
            .execute(
                "
                INSERT INTO request_logs (
                    request_id,
                    platform_key_id,
                    request_type,
                    model,
                    selected_endpoint_id,
                    attempt_count,
                    final_status,
                    http_status,
                    started_at_ms,
                    finished_at_ms,
                    latency_ms,
                    error_code,
                    error_reason,
                    requested_context_window,
                    requested_context_compression,
                    effective_context_window,
                    effective_context_compression
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
                ",
                params![
                    &request.request_id,
                    &request.platform_key_id,
                    &request.request_type,
                    &request.model,
                    request.selected_endpoint_id.as_deref(),
                    i64::from(request.attempt_count),
                    &request.final_status,
                    request.http_status,
                    request.started_at_ms,
                    request.finished_at_ms,
                    request.latency_ms,
                    request.error_code.as_deref(),
                    request.error_reason.as_deref(),
                    request.requested_context_window,
                    request.requested_context_compression.as_deref(),
                    request.effective_context_window,
                    request.effective_context_compression.as_deref(),
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to insert request log '{}': {error}",
                    request.request_id
                ))
            })?;

        for attempt in attempts {
            transaction
                .execute(
                    "
                    INSERT INTO request_attempt_logs (
                        attempt_id,
                        request_id,
                        attempt_index,
                        endpoint_id,
                        pool_type,
                        trigger_reason,
                        upstream_status,
                        timeout_ms,
                        latency_ms,
                        token_usage_snapshot,
                        estimated_cost_snapshot,
                        balance_snapshot_id,
                        feature_resolution_snapshot
                    )
                    VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
                    ",
                    params![
                        &attempt.attempt_id,
                        &attempt.request_id,
                        i64::from(attempt.attempt_index),
                        &attempt.endpoint_id,
                        &attempt.pool_type,
                        &attempt.trigger_reason,
                        attempt.upstream_status,
                        attempt.timeout_ms,
                        attempt.latency_ms,
                        attempt.token_usage_snapshot.as_deref(),
                        attempt.estimated_cost_snapshot.as_deref(),
                        attempt.balance_snapshot_id.as_deref(),
                        attempt.feature_resolution_snapshot.as_deref(),
                    ],
                )
                .map_err(|error| {
                    CodexLagError::new(format!(
                        "failed to insert request attempt '{}' for request '{}': {error}",
                        attempt.attempt_id, request.request_id
                    ))
                })?;
        }

        transaction.commit().map_err(|error| {
            CodexLagError::new(format!(
                "failed to commit request log transaction for '{}': {error}",
                request.request_id
            ))
        })?;

        Ok(())
    }

    pub fn upsert_pricing_profile(&self, profile: &PricingProfile) -> Result<()> {
        let connection = self.open_connection()?;
        connection
            .execute(
                "
                INSERT INTO pricing_profiles (
                    id,
                    model,
                    input_price_per_1k_micros,
                    output_price_per_1k_micros,
                    cache_read_price_per_1k_micros,
                    currency,
                    effective_from_ms,
                    effective_to_ms,
                    active
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT(id) DO UPDATE SET
                    model = excluded.model,
                    input_price_per_1k_micros = excluded.input_price_per_1k_micros,
                    output_price_per_1k_micros = excluded.output_price_per_1k_micros,
                    cache_read_price_per_1k_micros = excluded.cache_read_price_per_1k_micros,
                    currency = excluded.currency,
                    effective_from_ms = excluded.effective_from_ms,
                    effective_to_ms = excluded.effective_to_ms,
                    active = excluded.active
                ",
                params![
                    &profile.id,
                    &profile.model,
                    profile.input_price_per_1k_micros,
                    profile.output_price_per_1k_micros,
                    profile.cache_read_price_per_1k_micros,
                    &profile.currency,
                    profile.effective_from_ms,
                    profile.effective_to_ms,
                    profile.active as i64
                ],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist pricing profile '{}': {error}",
                    profile.id
                ))
            })?;
        Ok(())
    }

    pub fn active_pricing_profile_by_model(
        &self,
        model: &str,
        at_ms: i64,
    ) -> Result<Option<PricingProfile>> {
        let connection = self.open_connection()?;
        let mut statement = connection
            .prepare(
                "
                SELECT
                    id,
                    model,
                    input_price_per_1k_micros,
                    output_price_per_1k_micros,
                    cache_read_price_per_1k_micros,
                    currency,
                    effective_from_ms,
                    effective_to_ms,
                    active
                FROM pricing_profiles
                WHERE model = ?1
                  AND active = 1
                  AND effective_from_ms <= ?2
                  AND (effective_to_ms IS NULL OR effective_to_ms > ?2)
                ORDER BY effective_from_ms DESC
                LIMIT 1
                ",
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare pricing profile query: {error}"))
            })?;

        let row = statement.query_row(params![model, at_ms], |row| {
            Ok(PricingProfile {
                id: row.get(0)?,
                model: row.get(1)?,
                input_price_per_1k_micros: row.get(2)?,
                output_price_per_1k_micros: row.get(3)?,
                cache_read_price_per_1k_micros: row.get(4)?,
                currency: row.get(5)?,
                effective_from_ms: row.get(6)?,
                effective_to_ms: row.get(7)?,
                active: row.get::<_, i64>(8)? != 0,
            })
        });

        match row {
            Ok(profile) => Ok(Some(profile)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(error) => Err(CodexLagError::new(format!(
                "failed to query active pricing profile for model '{model}': {error}"
            ))),
        }
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
    ) -> Result<Option<UsageCostEstimate>> {
        let Some(profile) = self.active_pricing_profile_by_model(model, at_ms)? else {
            return Ok(None);
        };

        let total_micros = estimate_total_cost_micros(
            &profile,
            input_tokens,
            output_tokens,
            cache_read_tokens,
            cache_write_tokens,
            reasoning_tokens,
        );
        let amount = format!("{:.4}", (total_micros as f64) / 1_000_000.0_f64);

        Ok(Some(UsageCostEstimate {
            amount,
            currency: profile.currency.clone(),
            provenance: UsageProvenance::Estimated,
            estimated: true,
            pricing_profile_id: profile.id,
        }))
    }

    fn open_connection(&self) -> Result<Connection> {
        Self::open_sqlite(&self.database_path)
    }

    fn open_sqlite(database_path: &Path) -> Result<Connection> {
        let connection = Connection::open(database_path).map_err(|error| {
            CodexLagError::new(format!(
                "failed to open SQLite database '{}': {error}",
                database_path.display()
            ))
        })?;

        connection
            .busy_timeout(Duration::from_secs(5))
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to configure SQLite busy timeout for '{}': {error}",
                    database_path.display()
                ))
            })?;

        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to enable SQLite foreign keys for '{}': {error}",
                    database_path.display()
                ))
            })?;

        Ok(connection)
    }

    fn load_policies(connection: &Connection) -> Result<HashMap<String, RoutingPolicy>> {
        let mut statement = connection
            .prepare(
                "
                SELECT
                    id,
                    name,
                    selection_order,
                    cross_pool_fallback,
                    retry_budget,
                    failure_rules,
                    recovery_rules
                FROM routing_policies
                ",
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare policy query: {error}"))
            })?;

        let mut rows = statement
            .query([])
            .map_err(|error| CodexLagError::new(format!("failed to query policies: {error}")))?;

        let mut policies = HashMap::new();

        while let Some(row) = rows.next().map_err(|error| {
            CodexLagError::new(format!(
                "failed to read policy row from sqlite cursor: {error}"
            ))
        })? {
            let id: String = row.get(0).map_err(|error| {
                CodexLagError::new(format!("failed to decode policy id: {error}"))
            })?;
            let name: String = row.get(1).map_err(|error| {
                CodexLagError::new(format!("failed to decode policy name for '{id}': {error}"))
            })?;
            let selection_order_raw: String = row.get(2).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode selection_order for policy '{id}': {error}"
                ))
            })?;
            let cross_pool_fallback: i64 = row.get(3).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode cross_pool_fallback for policy '{id}': {error}"
                ))
            })?;
            let retry_budget_raw: i64 = row.get(4).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode retry_budget for policy '{id}': {error}"
                ))
            })?;
            let failure_rules_raw: String = row.get(5).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode failure_rules for policy '{id}': {error}"
                ))
            })?;
            let recovery_rules_raw: String = row.get(6).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode recovery_rules for policy '{id}': {error}"
                ))
            })?;

            let policy = RoutingPolicy {
                id,
                name: name.clone(),
                selection_order: decode_json(&selection_order_raw, "selection_order")?,
                cross_pool_fallback: cross_pool_fallback != 0,
                retry_budget: u32::try_from(retry_budget_raw).map_err(|_| {
                    CodexLagError::new(format!(
                        "retry_budget for policy '{name}' must be >= 0 but was {retry_budget_raw}"
                    ))
                })?,
                failure_rules: decode_json(&failure_rules_raw, "failure_rules")?,
                recovery_rules: decode_json(&recovery_rules_raw, "recovery_rules")?,
            };

            policies.insert(name, policy);
        }

        Ok(policies)
    }

    fn load_platform_keys(connection: &Connection) -> Result<HashMap<String, PlatformKey>> {
        let mut statement = connection
            .prepare("SELECT id, name, allowed_mode, policy_id, enabled FROM platform_keys")
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare platform key query: {error}"))
            })?;

        let rows = statement
            .query_map([], |row| {
                Ok(PlatformKey {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    allowed_mode: row.get(2)?,
                    policy_id: row.get(3)?,
                    enabled: row.get::<_, i64>(4)? != 0,
                })
            })
            .map_err(|error| {
                CodexLagError::new(format!("failed to query platform keys: {error}"))
            })?;

        let mut keys = HashMap::new();

        for row in rows {
            let key = row.map_err(|error| {
                CodexLagError::new(format!("failed to decode platform key row: {error}"))
            })?;
            keys.insert(key.name.clone(), key);
        }

        Ok(keys)
    }

    fn load_imported_official_accounts(
        connection: &Connection,
    ) -> Result<HashMap<String, ImportedOfficialAccount>> {
        let mut statement = connection
            .prepare(
                "
                SELECT
                    account_id,
                    name,
                    provider,
                    session_payload,
                    session_credential_ref,
                    token_credential_ref
                FROM imported_official_accounts
                ",
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to prepare imported official account query: {error}"
                ))
            })?;

        let mut rows = statement.query([]).map_err(|error| {
            CodexLagError::new(format!(
                "failed to query imported official accounts: {error}"
            ))
        })?;

        let mut accounts = HashMap::new();
        while let Some(row) = rows.next().map_err(|error| {
            CodexLagError::new(format!(
                "failed to read imported official account row from sqlite cursor: {error}"
            ))
        })? {
            let account_id: String = row.get(0).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account id: {error}"
                ))
            })?;
            let name: String = row.get(1).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account name for '{account_id}': {error}"
                ))
            })?;
            let provider: String = row.get(2).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account provider for '{account_id}': {error}"
                ))
            })?;
            let session_payload: String = row.get(3).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account session payload for '{account_id}': {error}"
                ))
            })?;
            let session_credential_ref: String = row.get(4).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account session credential ref for '{account_id}': {error}"
                ))
            })?;
            let token_credential_ref: String = row.get(5).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode imported official account token credential ref for '{account_id}': {error}"
                ))
            })?;

            let account = ImportedOfficialAccount {
                account_id: account_id.clone(),
                name,
                provider,
                session: decode_json(&session_payload, "session_payload")?,
                session_credential_ref,
                token_credential_ref,
            };
            accounts.insert(account_id, account);
        }

        Ok(accounts)
    }

    fn load_managed_relays(connection: &Connection) -> Result<HashMap<String, ManagedRelay>> {
        let mut statement = connection
            .prepare(
                "
                SELECT
                    relay_id,
                    name,
                    endpoint,
                    adapter
                FROM managed_relays
                ",
            )
            .map_err(|error| {
                CodexLagError::new(format!("failed to prepare managed relay query: {error}"))
            })?;

        let mut rows = statement.query([]).map_err(|error| {
            CodexLagError::new(format!("failed to query managed relays: {error}"))
        })?;
        let mut relays = HashMap::new();

        while let Some(row) = rows.next().map_err(|error| {
            CodexLagError::new(format!(
                "failed to read managed relay row from sqlite cursor: {error}"
            ))
        })? {
            let relay_id: String = row.get(0).map_err(|error| {
                CodexLagError::new(format!("failed to decode managed relay id: {error}"))
            })?;
            let name: String = row.get(1).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode managed relay name for '{relay_id}': {error}"
                ))
            })?;
            let endpoint: String = row.get(2).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode managed relay endpoint for '{relay_id}': {error}"
                ))
            })?;
            let adapter_raw: String = row.get(3).map_err(|error| {
                CodexLagError::new(format!(
                    "failed to decode managed relay adapter for '{relay_id}': {error}"
                ))
            })?;
            let adapter = decode_json(&adapter_raw, "relay_adapter")?;

            relays.insert(
                relay_id.clone(),
                ManagedRelay {
                    relay_id,
                    name,
                    endpoint,
                    adapter,
                },
            );
        }

        Ok(relays)
    }

    fn upsert_credential_ref(
        transaction: &Transaction<'_>,
        id: &str,
        target_name: &str,
        credential_kind: CredentialKind,
    ) -> Result<()> {
        let kind = encode_json(&credential_kind, "credential_kind")?;
        transaction
            .execute(
                "
                INSERT INTO credential_refs (
                    id,
                    target_name,
                    version,
                    credential_kind,
                    last_verified_at_ms
                )
                VALUES (?1, ?2, 1, ?3, NULL)
                ON CONFLICT(id) DO UPDATE SET
                    target_name = excluded.target_name,
                    credential_kind = excluded.credential_kind
                ",
                params![id, target_name, kind],
            )
            .map_err(|error| {
                CodexLagError::new(format!(
                    "failed to persist credential reference '{id}': {error}"
                ))
            })?;
        Ok(())
    }
}

fn estimate_total_cost_micros(
    profile: &PricingProfile,
    input_tokens: u32,
    output_tokens: u32,
    cache_read_tokens: u32,
    _cache_write_tokens: u32,
    reasoning_tokens: u32,
) -> i128 {
    // Reasoning tokens are charged using output-token pricing until a dedicated
    // reasoning price dimension exists.
    let billable_nano_micros = i128::from(input_tokens)
        * i128::from(profile.input_price_per_1k_micros)
        + i128::from(output_tokens) * i128::from(profile.output_price_per_1k_micros)
        + i128::from(cache_read_tokens) * i128::from(profile.cache_read_price_per_1k_micros)
        + i128::from(reasoning_tokens) * i128::from(profile.output_price_per_1k_micros);

    billable_nano_micros / 1_000
}

fn encode_json<T: Serialize>(value: &T, field_name: &str) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| CodexLagError::new(format!("failed to serialize {field_name}: {error}")))
}

fn decode_json<T: DeserializeOwned>(raw: &str, field_name: &str) -> Result<T> {
    serde_json::from_str(raw)
        .map_err(|error| CodexLagError::new(format!("failed to decode {field_name}: {error}")))
}
