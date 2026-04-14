use rusqlite::Connection;

use crate::error::{CodexLagError, Result};

pub fn ensure_schema_up_to_date(connection: &Connection) -> Result<()> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS routing_policies (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE
            );

            CREATE TABLE IF NOT EXISTS platform_keys (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                key_prefix TEXT NOT NULL DEFAULT 'ck_local_',
                allowed_mode TEXT NOT NULL,
                policy_id TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                created_at_ms INTEGER NOT NULL DEFAULT 0,
                last_used_at_ms INTEGER,
                FOREIGN KEY(policy_id) REFERENCES routing_policies(id)
            );

            CREATE TABLE IF NOT EXISTS pricing_profiles (
                id TEXT PRIMARY KEY,
                model TEXT NOT NULL,
                input_price_per_1k_micros INTEGER NOT NULL,
                output_price_per_1k_micros INTEGER NOT NULL,
                cache_read_price_per_1k_micros INTEGER NOT NULL,
                currency TEXT NOT NULL,
                effective_from_ms INTEGER NOT NULL,
                effective_to_ms INTEGER,
                active INTEGER NOT NULL DEFAULT 1
            );

            CREATE INDEX IF NOT EXISTS idx_pricing_profiles_model_active_effective
            ON pricing_profiles(model, active, effective_from_ms DESC);

            CREATE TABLE IF NOT EXISTS credential_refs (
                id TEXT PRIMARY KEY,
                target_name TEXT NOT NULL,
                version INTEGER NOT NULL DEFAULT 1,
                credential_kind TEXT NOT NULL,
                last_verified_at_ms INTEGER
            );

            CREATE TABLE IF NOT EXISTS provider_endpoints (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                kind TEXT NOT NULL,
                enabled INTEGER NOT NULL,
                priority INTEGER NOT NULL DEFAULT 0,
                pool_tags TEXT NOT NULL DEFAULT '[]',
                health_status TEXT NOT NULL DEFAULT 'healthy',
                last_health_check_at_ms INTEGER,
                supports_balance_query INTEGER NOT NULL DEFAULT 0,
                last_balance_snapshot_at_ms INTEGER,
                pricing_profile_id TEXT,
                credential_ref_id TEXT,
                feature_capabilities TEXT NOT NULL DEFAULT '[]',
                FOREIGN KEY(pricing_profile_id) REFERENCES pricing_profiles(id),
                FOREIGN KEY(credential_ref_id) REFERENCES credential_refs(id)
            );

            CREATE TABLE IF NOT EXISTS imported_official_accounts (
                account_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                provider TEXT NOT NULL,
                session_payload TEXT NOT NULL,
                session_credential_ref TEXT NOT NULL,
                token_credential_ref TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS managed_relays (
                relay_id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                endpoint TEXT NOT NULL,
                adapter TEXT NOT NULL,
                api_key_credential_ref TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS request_logs (
                request_id TEXT PRIMARY KEY,
                platform_key_id TEXT NOT NULL,
                request_type TEXT NOT NULL,
                model TEXT NOT NULL,
                selected_endpoint_id TEXT,
                attempt_count INTEGER NOT NULL,
                final_status TEXT NOT NULL,
                http_status INTEGER,
                started_at_ms INTEGER NOT NULL,
                finished_at_ms INTEGER,
                latency_ms INTEGER,
                error_code TEXT,
                error_reason TEXT,
                requested_context_window INTEGER,
                requested_context_compression TEXT,
                effective_context_window INTEGER,
                effective_context_compression TEXT
            );

            CREATE TABLE IF NOT EXISTS request_attempt_logs (
                attempt_id TEXT PRIMARY KEY,
                request_id TEXT NOT NULL,
                attempt_index INTEGER NOT NULL,
                endpoint_id TEXT NOT NULL,
                pool_type TEXT NOT NULL,
                trigger_reason TEXT NOT NULL,
                upstream_status INTEGER,
                timeout_ms INTEGER,
                latency_ms INTEGER,
                token_usage_snapshot TEXT,
                estimated_cost_snapshot TEXT,
                balance_snapshot_id TEXT,
                feature_resolution_snapshot TEXT,
                FOREIGN KEY(request_id) REFERENCES request_logs(request_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_request_attempt_logs_request_id
            ON request_attempt_logs(request_id, attempt_index);
            ",
        )
        .map_err(|error| {
            CodexLagError::new(format!("failed to apply SQLite migrations: {error}"))
        })?;

    add_column_if_missing(
        connection,
        "routing_policies",
        "selection_order",
        "TEXT NOT NULL DEFAULT '[]'",
    )?;
    add_column_if_missing(
        connection,
        "routing_policies",
        "cross_pool_fallback",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    add_column_if_missing(
        connection,
        "routing_policies",
        "retry_budget",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(
        connection,
        "routing_policies",
        "failure_rules",
        "TEXT NOT NULL DEFAULT '{\"cooldown_ms\":30000,\"timeout_open_after\":3,\"server_error_open_after\":3}'",
    )?;
    add_column_if_missing(
        connection,
        "routing_policies",
        "recovery_rules",
        "TEXT NOT NULL DEFAULT '{\"half_open_after_ms\":15000,\"success_close_after\":1}'",
    )?;
    add_column_if_missing(
        connection,
        "platform_keys",
        "key_prefix",
        "TEXT NOT NULL DEFAULT 'ck_local_'",
    )?;
    add_column_if_missing(
        connection,
        "platform_keys",
        "created_at_ms",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    add_column_if_missing(connection, "platform_keys", "last_used_at_ms", "INTEGER")?;
    add_column_if_missing(
        connection,
        "managed_relays",
        "api_key_credential_ref",
        "TEXT NOT NULL DEFAULT ''",
    )?;

    connection
        .execute_batch(
            "
            UPDATE routing_policies SET selection_order = '[]' WHERE selection_order IS NULL OR selection_order = '';
            UPDATE routing_policies SET cross_pool_fallback = 1 WHERE cross_pool_fallback IS NULL;
            UPDATE routing_policies SET retry_budget = 0 WHERE retry_budget IS NULL;
            UPDATE routing_policies
            SET failure_rules = '{\"cooldown_ms\":30000,\"timeout_open_after\":3,\"server_error_open_after\":3}'
            WHERE failure_rules IS NULL OR failure_rules = '';
            UPDATE routing_policies
            SET recovery_rules = '{\"half_open_after_ms\":15000,\"success_close_after\":1}'
            WHERE recovery_rules IS NULL OR recovery_rules = '';
            UPDATE platform_keys
            SET key_prefix = 'ck_local_'
            WHERE key_prefix IS NULL OR key_prefix = '';
            UPDATE platform_keys
            SET created_at_ms = 0
            WHERE created_at_ms IS NULL;
            UPDATE managed_relays
            SET api_key_credential_ref = 'credential://relay/api-key/' || relay_id
            WHERE api_key_credential_ref IS NULL OR api_key_credential_ref = '';
            ",
        )
        .map_err(|error| {
            CodexLagError::new(format!(
                "failed to backfill routing policy defaults during migration: {error}"
            ))
        })?;

    Ok(())
}

fn add_column_if_missing(
    connection: &Connection,
    table_name: &str,
    column_name: &str,
    column_definition: &str,
) -> Result<()> {
    let pragma = format!("PRAGMA table_info({table_name})");
    let mut statement = connection.prepare(&pragma).map_err(|error| {
        CodexLagError::new(format!(
            "failed to prepare schema introspection query for table '{table_name}': {error}"
        ))
    })?;

    let columns: Vec<String> = statement
        .query_map([], |row| row.get(1))
        .map_err(|error| {
            CodexLagError::new(format!(
                "failed to query schema info for table '{table_name}': {error}"
            ))
        })?
        .collect::<std::result::Result<Vec<String>, _>>()
        .map_err(|error| {
            CodexLagError::new(format!(
                "failed to decode schema info for table '{table_name}': {error}"
            ))
        })?;

    if columns.iter().any(|column| column == column_name) {
        return Ok(());
    }

    let alter = format!("ALTER TABLE {table_name} ADD COLUMN {column_name} {column_definition}");
    connection.execute(&alter, []).map_err(|error| {
        CodexLagError::new(format!(
            "failed to add missing column '{column_name}' on table '{table_name}': {error}"
        ))
    })?;
    Ok(())
}
