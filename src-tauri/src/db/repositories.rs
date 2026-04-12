use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{params, Connection};
use serde::{de::DeserializeOwned, Serialize};

use crate::db::migrations::ensure_schema_up_to_date;
use crate::error::{CodexLagError, Result};
use crate::models::{
    ExpandedRoutingPolicy, PlatformKey, PricingProfile, RequestAttemptLog, RequestLog,
    RoutingPolicy,
};

pub struct Repositories {
    database_path: PathBuf,
    policies: HashMap<String, RoutingPolicy>,
    expanded_policies: HashMap<String, ExpandedRoutingPolicy>,
    keys: HashMap<String, PlatformKey>,
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

        let expanded_policies = Self::load_policies(&connection)?;
        let policies = expanded_policies
            .values()
            .map(|policy| (policy.name.clone(), policy.as_routing_policy()))
            .collect();
        let keys = Self::load_platform_keys(&connection)?;

        Ok(Self {
            database_path,
            policies,
            expanded_policies,
            keys,
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

        self.save_policy(ExpandedRoutingPolicy::from_routing_policy(policy))
    }

    pub fn save_policy(&mut self, policy: ExpandedRoutingPolicy) -> Result<()> {
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

        self.policies
            .insert(policy.name.clone(), policy.as_routing_policy());
        self.expanded_policies.insert(policy.name.clone(), policy);
        Ok(())
    }

    pub fn insert_platform_key(&mut self, key: PlatformKey) -> Result<()> {
        let name = key.name.clone();

        if self.keys.contains_key(&name) {
            return Err(CodexLagError::new(format!(
                "platform key '{}' already exists",
                name
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

    pub fn expanded_policy(&self, name: &str) -> Option<&ExpandedRoutingPolicy> {
        self.expanded_policies.get(name)
    }

    pub fn platform_key(&self, name: &str) -> Option<&PlatformKey> {
        self.keys.get(name)
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

    pub fn iter_policies(&self) -> impl Iterator<Item = &RoutingPolicy> {
        self.policies.values()
    }

    pub fn iter_expanded_policies(&self) -> impl Iterator<Item = &ExpandedRoutingPolicy> {
        self.expanded_policies.values()
    }

    pub fn iter_platform_keys(&self) -> impl Iterator<Item = &PlatformKey> {
        self.keys.values()
    }

    pub fn append_request_with_attempts(
        &self,
        request: &RequestLog,
        attempts: &[RequestAttemptLog],
    ) -> Result<()> {
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

    fn load_policies(connection: &Connection) -> Result<HashMap<String, ExpandedRoutingPolicy>> {
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

            let policy = ExpandedRoutingPolicy {
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
}

fn encode_json<T: Serialize>(value: &T, field_name: &str) -> Result<String> {
    serde_json::to_string(value)
        .map_err(|error| CodexLagError::new(format!("failed to serialize {field_name}: {error}")))
}

fn decode_json<T: DeserializeOwned>(raw: &str, field_name: &str) -> Result<T> {
    serde_json::from_str(raw)
        .map_err(|error| CodexLagError::new(format!("failed to decode {field_name}: {error}")))
}
