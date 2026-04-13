use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use codexlag_lib::commands::accounts::{
    get_account_capability_detail_from_runtime, import_official_account_login_from_runtime,
    list_accounts_from_runtime, OfficialAccountImportInput,
};
use codexlag_lib::commands::keys::{
    create_platform_key_from_runtime, disable_platform_key_from_runtime,
    enable_platform_key_from_runtime, list_platform_keys_from_runtime, CreatePlatformKeyInput,
};
use codexlag_lib::commands::logs::{
    export_runtime_diagnostics_from_runtime, runtime_log_metadata_from_runtime,
    usage_ledger_from_runtime, usage_request_detail_from_runtime,
    usage_request_history_from_runtime,
};
use codexlag_lib::commands::policies::{
    policy_summaries_from_state, update_policy_from_runtime, PolicyUpdateInput,
};
use codexlag_lib::commands::relays::{
    add_relay_from_runtime, delete_relay_from_runtime, get_relay_capability_detail_from_runtime,
    list_relays_from_runtime, test_relay_connection_from_runtime, update_relay_from_runtime,
    RelayCapabilityDetail, RelayUpsertInput,
};
use codexlag_lib::error::{CodexLagError, ErrorCategory};
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance, UsageRecordInput};
use codexlag_lib::providers::official::OfficialBalanceCapability;
use codexlag_lib::providers::relay::{RelayBalanceAdapter, RelayBalanceCapability};
use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test_at},
    routing::policy::RoutingMode,
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
};
use rusqlite::{params, Connection};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock after unix epoch")
        .as_nanos();
    nanos.to_string()
}

async fn runtime_for_paths(database_path: &Path, log_dir: &Path) -> RuntimeState {
    let app_state = bootstrap_state_for_test_at(database_path)
        .await
        .expect("bootstrap isolated app state");
    RuntimeState::new(
        app_state,
        RuntimeLogConfig {
            log_dir: log_dir.to_path_buf(),
        },
    )
}

fn isolated_test_root(prefix: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}-{}",
        std::process::id(),
        unique_suffix()
    ))
}

fn spawn_single_accept_listener() -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback listener");
    let endpoint = format!("http://{}", listener.local_addr().expect("listener addr"));
    let accept_once = std::thread::spawn(move || {
        let _ = listener.accept();
    });
    (endpoint, accept_once)
}

fn assert_structured_command_error(
    error: CodexLagError,
    expected_code: &str,
    expected_category: ErrorCategory,
    expected_message: &str,
    expected_context_fragment: &str,
) {
    let payload = error.to_payload();
    assert_eq!(payload.code, expected_code);
    assert_eq!(payload.category, expected_category);
    assert_eq!(payload.message, expected_message);
    let internal_context = payload
        .internal_context
        .expect("internal context should be present");
    assert!(
        internal_context.contains(expected_context_fragment),
        "internal_context should include '{expected_context_fragment}', got: {internal_context}"
    );
}

#[tokio::test]
async fn account_and_relay_capability_details_expose_balance_metadata() {
    let isolated_root = isolated_test_root("command-capabilities");
    let database_path = isolated_root.join("state.sqlite3");
    let runtime = runtime_for_paths(&database_path, &isolated_root.join("logs")).await;

    let account =
        get_account_capability_detail_from_runtime(&runtime, "official-primary".to_string())
            .expect("official account should succeed");
    assert_eq!(account.account_id, "official-primary");
    assert_eq!(account.refresh_capability, Some(true));
    assert_eq!(
        account.balance_capability,
        OfficialBalanceCapability::NonQueryable
    );

    let relay = get_relay_capability_detail_from_runtime(&runtime, "relay-newapi".to_string())
        .expect("relay should succeed");
    assert_eq!(relay.relay_id, "relay-newapi");
    assert_eq!(
        relay.balance_capability,
        RelayBalanceCapability::Queryable {
            adapter: RelayBalanceAdapter::NewApi
        }
    );

    let unsupported =
        get_relay_capability_detail_from_runtime(&runtime, "relay-nobalance".to_string())
            .expect("unsupported relay should still return capability details");
    assert_eq!(
        unsupported,
        RelayCapabilityDetail {
            relay_id: "relay-nobalance".to_string(),
            endpoint: "https://relay.example.test".to_string(),
            balance_capability: RelayBalanceCapability::Unsupported,
        }
    );
    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn capability_detail_commands_return_explicit_errors_for_unknown_ids() {
    let isolated_root = isolated_test_root("command-capability-errors");
    let database_path = isolated_root.join("state.sqlite3");
    let runtime = runtime_for_paths(&database_path, &isolated_root.join("logs")).await;

    let account_error =
        get_account_capability_detail_from_runtime(&runtime, "unknown-account".to_string())
            .expect_err("unknown account should be reported");
    assert_structured_command_error(
        account_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown account id.",
        "command=get_account_capability_detail;account_id=unknown-account",
    );

    let relay_error =
        get_relay_capability_detail_from_runtime(&runtime, "relay-missing".to_string())
            .expect_err("unknown relay should be reported");
    assert_structured_command_error(
        relay_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown relay id.",
        "command=relay_lookup;field=relay_id;value=relay-missing",
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn account_import_login_command_validates_and_persists_entry() {
    let isolated_root = isolated_test_root("command-account-import");
    let database_path = isolated_root.join("state.sqlite3");
    let first_runtime = runtime_for_paths(&database_path, &isolated_root.join("logs-first")).await;

    let suffix = unique_suffix();
    let account_id = format!("official-imported-{suffix}");
    let session_ref = format!("credential://official/session/{suffix}");
    let imported = import_official_account_login_from_runtime(
        &first_runtime,
        OfficialAccountImportInput {
            account_id: account_id.clone(),
            name: "Imported Official Account".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: session_ref.clone(),
            token_credential_ref: format!("credential://official/token/{suffix}"),
            account_identity: Some(format!("user-{suffix}@example.test")),
            auth_mode: Some("oauth".to_string()),
        },
    )
    .expect("account import should succeed");
    assert_eq!(imported.account_id, account_id);

    let accounts = list_accounts_from_runtime(&first_runtime);
    assert!(
        accounts
            .iter()
            .any(|account| account.account_id == account_id),
        "imported account should be visible in list_accounts"
    );

    let invalid_error = import_official_account_login_from_runtime(
        &first_runtime,
        OfficialAccountImportInput {
            account_id: format!("official-invalid-{suffix}"),
            name: "Invalid Official Account".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: "credential://official/token/not-a-session".to_string(),
            token_credential_ref: format!("credential://official/token/{suffix}-bad"),
            account_identity: None,
            auth_mode: None,
        },
    )
    .expect_err("invalid session credential ref should be rejected");
    assert_structured_command_error(
        invalid_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "session credential ref must start with 'credential://official/session/'",
        "command=account_import_validation;field=session credential ref",
    );

    let reserved_error = import_official_account_login_from_runtime(
        &first_runtime,
        OfficialAccountImportInput {
            account_id: "official-primary".to_string(),
            name: "Reserved Account".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: format!("credential://official/session/{suffix}-reserved"),
            token_credential_ref: format!("credential://official/token/{suffix}-reserved"),
            account_identity: None,
            auth_mode: None,
        },
    )
    .expect_err("reserved built-in account ids should be rejected");
    assert_structured_command_error(
        reserved_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "account_id is reserved and cannot be imported: official-primary",
        "command=account_import_validation;field=account_id;value=official-primary;reason=reserved",
    );

    drop(first_runtime);

    let second_runtime =
        runtime_for_paths(&database_path, &isolated_root.join("logs-second")).await;
    let reloaded_accounts = list_accounts_from_runtime(&second_runtime);
    assert!(
        reloaded_accounts
            .iter()
            .any(|account| account.account_id == account_id),
        "imported account should persist across runtime restart"
    );
    drop(second_runtime);

    let connection = Connection::open(&database_path).expect("open sqlite");
    let credential_ref_rows: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM credential_refs WHERE id = ?1",
            params![session_ref],
            |row| row.get(0),
        )
        .expect("count credential refs");
    assert_eq!(
        credential_ref_rows, 1,
        "account import should persist a credential reference"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn account_import_login_rolls_back_when_credential_ref_persistence_fails() {
    let isolated_root = isolated_test_root("command-account-import-rollback");
    let database_path = isolated_root.join("state.sqlite3");
    let runtime = runtime_for_paths(&database_path, &isolated_root.join("logs")).await;

    let suffix = unique_suffix();
    let account_id = format!("official-imported-rollback-{suffix}");
    let session_ref = format!("credential://official/session/{suffix}-rollback");
    let token_ref = format!("credential://official/token/{suffix}-rollback");
    let escaped_session_ref = session_ref.replace('\'', "''");

    let connection = Connection::open(&database_path).expect("open sqlite");
    connection
        .execute_batch(&format!(
            "
            DROP TRIGGER IF EXISTS fail_credential_ref_insert;
            CREATE TRIGGER fail_credential_ref_insert
            BEFORE INSERT ON credential_refs
            WHEN NEW.id = '{escaped_session_ref}'
            BEGIN
                SELECT RAISE(ABORT, 'simulated credential ref failure');
            END;
            "
        ))
        .expect("install failing credential_ref trigger");

    let error = import_official_account_login_from_runtime(
        &runtime,
        OfficialAccountImportInput {
            account_id: account_id.clone(),
            name: "Rollback Candidate".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: session_ref.clone(),
            token_credential_ref: token_ref.clone(),
            account_identity: None,
            auth_mode: None,
        },
    )
    .expect_err("credential ref write failures should fail import");
    assert_structured_command_error(
        error,
        "config.unknown",
        ErrorCategory::ConfigError,
        "Failed to persist imported official account login.",
        "command=import_official_account_login;operation=save_imported_official_account",
    );
    assert!(
        list_accounts_from_runtime(&runtime)
            .iter()
            .all(|account| account.account_id != account_id),
        "failed import should not leave account visible in runtime state"
    );

    drop(runtime);

    let verification = Connection::open(&database_path).expect("open sqlite for verification");
    let persisted_account_rows: i64 = verification
        .query_row(
            "SELECT COUNT(*) FROM imported_official_accounts WHERE account_id = ?1",
            params![account_id],
            |row| row.get(0),
        )
        .expect("count imported account rows");
    assert_eq!(
        persisted_account_rows, 0,
        "failed import should roll back imported_official_accounts row"
    );

    let persisted_credential_rows: i64 = verification
        .query_row(
            "SELECT COUNT(*) FROM credential_refs WHERE id IN (?1, ?2)",
            params![session_ref, token_ref],
            |row| row.get(0),
        )
        .expect("count credential refs");
    assert_eq!(
        persisted_credential_rows, 0,
        "failed import should not persist credential references"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn relay_crud_and_test_connection_commands_validate_unknown_ids() {
    let isolated_root = isolated_test_root("command-relays");
    let database_path = isolated_root.join("state.sqlite3");
    let first_runtime = runtime_for_paths(&database_path, &isolated_root.join("logs-first")).await;

    let suffix = unique_suffix();
    let relay_id = format!("relay-managed-{suffix}");
    let (first_endpoint, first_accept) = spawn_single_accept_listener();
    let created = add_relay_from_runtime(
        &first_runtime,
        RelayUpsertInput {
            relay_id: relay_id.clone(),
            name: "Managed Relay".to_string(),
            endpoint: first_endpoint,
            adapter: Some("newapi".to_string()),
        },
    )
    .expect("relay add should succeed");
    assert_eq!(created.relay_id, relay_id);

    let tested = test_relay_connection_from_runtime(&first_runtime, relay_id.clone())
        .expect("relay test should succeed");
    assert_eq!(tested.status, "ok");
    first_accept.join().expect("relay probe listener thread");

    let (updated_endpoint, second_accept) = spawn_single_accept_listener();

    let updated = update_relay_from_runtime(
        &first_runtime,
        RelayUpsertInput {
            relay_id: relay_id.clone(),
            name: "Managed Relay Updated".to_string(),
            endpoint: updated_endpoint.clone(),
            adapter: Some("newapi".to_string()),
        },
    )
    .expect("relay update should succeed");
    assert_eq!(updated.name, "Managed Relay Updated");
    let tested_updated = test_relay_connection_from_runtime(&first_runtime, relay_id.clone())
        .expect("updated relay test should succeed");
    assert_eq!(tested_updated.status, "ok");
    second_accept
        .join()
        .expect("updated relay probe listener thread");

    drop(first_runtime);

    let second_runtime =
        runtime_for_paths(&database_path, &isolated_root.join("logs-second")).await;
    assert!(
        list_relays_from_runtime(&second_runtime)
            .iter()
            .any(|relay| relay.relay_id == relay_id && relay.endpoint == updated_endpoint),
        "managed relay should persist across runtime restart"
    );

    delete_relay_from_runtime(&second_runtime, relay_id.clone())
        .expect("relay delete should succeed");
    drop(second_runtime);

    let third_runtime = runtime_for_paths(&database_path, &isolated_root.join("logs-third")).await;
    assert!(
        !list_relays_from_runtime(&third_runtime)
            .iter()
            .any(|relay| relay.relay_id == relay_id),
        "deleted relay should remain absent after restart"
    );

    let unknown_error = test_relay_connection_from_runtime(&third_runtime, relay_id.clone())
        .expect_err("deleted relay should be unknown");
    assert_structured_command_error(
        unknown_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown relay id.",
        format!("command=relay_lookup;field=relay_id;value={relay_id}").as_str(),
    );

    let invalid_error = add_relay_from_runtime(
        &third_runtime,
        RelayUpsertInput {
            relay_id: format!("relay-invalid-{suffix}"),
            name: "Invalid Relay".to_string(),
            endpoint: "ftp://relay.example.test".to_string(),
            adapter: Some("newapi".to_string()),
        },
    )
    .expect_err("invalid endpoint scheme should be rejected");
    assert_structured_command_error(
        invalid_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Relay endpoint must start with 'http://' or 'https://'.",
        "command=relay_validation;field=endpoint;value=ftp://relay.example.test",
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn relay_probe_does_not_block_writes_while_connection_probe_times_out() {
    let isolated_root = isolated_test_root("command-relay-probe-lock-scope");
    let database_path = isolated_root.join("state.sqlite3");
    let runtime = runtime_for_paths(&database_path, &isolated_root.join("logs")).await;

    let suffix = unique_suffix();
    let relay_id = format!("relay-slow-probe-{suffix}");

    let slow_listener = TcpListener::bind("127.0.0.1:0").expect("bind slow probe listener");
    let slow_addr = slow_listener
        .local_addr()
        .expect("resolve slow probe listener address");
    let slow_endpoint = format!("http://{slow_addr}");

    let mut backlog_connections = Vec::new();
    for _ in 0..1024 {
        match TcpStream::connect_timeout(&slow_addr, Duration::from_millis(10)) {
            Ok(stream) => backlog_connections.push(stream),
            Err(_) => break,
        }
    }
    assert!(
        !backlog_connections.is_empty(),
        "expected at least one queued connection for slow probe endpoint"
    );

    add_relay_from_runtime(
        &runtime,
        RelayUpsertInput {
            relay_id: relay_id.clone(),
            name: "Slow Probe Relay".to_string(),
            endpoint: slow_endpoint,
            adapter: Some("newapi".to_string()),
        },
    )
    .expect("relay add should succeed");

    let (probe_elapsed, write_elapsed) = std::thread::scope(|scope| {
        let (probe_started_tx, probe_started_rx) = std::sync::mpsc::channel();
        let probe_relay_id = relay_id.clone();
        let runtime_ref = &runtime;
        let probe_handle = scope.spawn(move || {
            probe_started_tx
                .send(())
                .expect("signal probe thread start");
            let probe_started = Instant::now();
            let probe_result = test_relay_connection_from_runtime(runtime_ref, probe_relay_id)
                .expect("probe command should return a result");
            (probe_started.elapsed(), probe_result)
        });

        probe_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("probe thread should start");
        std::thread::sleep(Duration::from_millis(100));

        let write_started = Instant::now();
        let updated = update_relay_from_runtime(
            &runtime,
            RelayUpsertInput {
                relay_id: relay_id.clone(),
                name: "Updated During Probe".to_string(),
                endpoint: "http://127.0.0.1:1".to_string(),
                adapter: Some("newapi".to_string()),
            },
        )
        .expect("relay update should succeed while probe is in-flight");
        assert_eq!(updated.name, "Updated During Probe");
        let write_elapsed = write_started.elapsed();

        let (probe_elapsed, probe_result) = probe_handle
            .join()
            .expect("probe thread should complete without panic");
        assert_eq!(
            probe_result.status, "failed",
            "saturated listener should eventually fail the probe after timeout"
        );
        (probe_elapsed, write_elapsed)
    });

    assert!(
        probe_elapsed >= Duration::from_secs(2),
        "probe should stay in-flight long enough to validate lock scope; elapsed: {probe_elapsed:?}"
    );
    assert!(
        write_elapsed < Duration::from_millis(500),
        "write operation should not block on probe lock scope; elapsed: {write_elapsed:?}"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn key_inventory_commands_create_list_disable_and_enable() {
    let isolated_root = isolated_test_root("command-keys");
    let database_path = isolated_root.join("state.sqlite3");
    let first_runtime = runtime_for_paths(&database_path, &isolated_root.join("logs-first")).await;

    let suffix = unique_suffix();
    let key_id = format!("pk-{suffix}");
    let key_name = format!("managed-key-{suffix}");
    let created = create_platform_key_from_runtime(
        &first_runtime,
        CreatePlatformKeyInput {
            key_id: key_id.clone(),
            name: key_name.clone(),
            policy_id: "policy-default".to_string(),
            allowed_mode: "hybrid".to_string(),
        },
    )
    .expect("key create should succeed");
    assert!(created.enabled);

    let listed = list_platform_keys_from_runtime(&first_runtime);
    assert!(
        listed.iter().any(|entry| entry.id == key_id),
        "created key should be visible in list_platform_keys"
    );
    assert!(
        first_runtime
            .app_state()
            .iter_platform_keys()
            .any(|key| key.id == key_id),
        "created key should be visible to runtime auth state"
    );

    let disabled = disable_platform_key_from_runtime(&first_runtime, key_id.clone())
        .expect("disable should succeed");
    assert!(!disabled.enabled);
    let enabled = enable_platform_key_from_runtime(&first_runtime, key_id.clone())
        .expect("enable should succeed");
    assert!(enabled.enabled);

    let unknown_error =
        disable_platform_key_from_runtime(&first_runtime, format!("missing-{suffix}"))
            .expect_err("unknown ids should return explicit errors");
    assert_structured_command_error(
        unknown_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown platform key id.",
        format!("command=set_platform_key_enabled;field=key_id;value=missing-{suffix}").as_str(),
    );

    drop(first_runtime);

    let second_runtime =
        runtime_for_paths(&database_path, &isolated_root.join("logs-second")).await;
    assert!(
        list_platform_keys_from_runtime(&second_runtime)
            .iter()
            .any(|entry| entry.id == key_id && entry.enabled),
        "key inventory changes should persist across runtime restart"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn policy_update_command_enforces_strict_validation() {
    let isolated_root = isolated_test_root("command-policies");
    let database_path = isolated_root.join("state.sqlite3");
    let first_runtime = runtime_for_paths(&database_path, &isolated_root.join("logs-first")).await;
    let default_policy_id = first_runtime
        .app_state()
        .default_policy()
        .expect("default policy")
        .id
        .clone();

    let updated = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            policy_id: default_policy_id.clone(),
            name: "default".to_string(),
            selection_order: vec!["official-primary".to_string(), "relay-newapi".to_string()],
            cross_pool_fallback: true,
            retry_budget: 2,
            timeout_open_after: 3,
            server_error_open_after: 3,
            cooldown_ms: 30_000,
            half_open_after_ms: 15_000,
            success_close_after: 1,
        },
    )
    .expect("default policy update should succeed");
    assert_eq!(updated.retry_budget, 2);
    let summaries = policy_summaries_from_state(&first_runtime.app_state());
    let default_summary = summaries
        .iter()
        .find(|summary| summary.policy_id == default_policy_id)
        .expect("default policy should remain visible by id");
    assert_eq!(default_summary.name, "default");
    assert!(
        summaries.iter().any(|summary| summary.name == "default"),
        "updated policy should remain visible via list_policies view"
    );
    assert_eq!(
        first_runtime
            .app_state()
            .get_policy_by_id(&default_policy_id)
            .expect("updated default policy in runtime")
            .retry_budget,
        2
    );

    let invalid_error = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            retry_budget: 0,
            ..updated.clone()
        },
    )
    .expect_err("retry_budget must be strictly positive");
    assert_structured_command_error(
        invalid_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "retry_budget must be greater than 0",
        "command=policy_validation;field=retry_budget;value=0",
    );

    let unknown_endpoint_error = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            selection_order: vec!["official-primary".to_string(), "relay-missing".to_string()],
            ..updated.clone()
        },
    )
    .expect_err("selection_order should reject unknown endpoint ids");
    assert_structured_command_error(
        unknown_endpoint_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown selection_order endpoint id.",
        "command=update_policy;field=selection_order;value=relay-missing",
    );

    let unknown_error = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            policy_id: "missing-policy".to_string(),
            ..updated
        },
    )
    .expect_err("unknown policy id should be explicit");
    assert_structured_command_error(
        unknown_error,
        "config.invalid_payload",
        ErrorCategory::ConfigError,
        "Unknown policy id.",
        "command=update_policy;field=policy_id;value=missing-policy",
    );

    drop(first_runtime);

    let second_runtime =
        runtime_for_paths(&database_path, &isolated_root.join("logs-second")).await;
    assert_eq!(
        second_runtime
            .app_state()
            .get_policy_by_id(&default_policy_id)
            .expect("default policy after restart")
            .retry_budget,
        2,
        "policy update should persist to repository-backed state"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn usage_commands_expose_request_detail_history_and_ledger_provenance() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-1".to_string(),
        endpoint_id: "official-default".to_string(),
        model: None,
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        reasoning_tokens: 0,
        estimated_cost: "0.0123".to_string(),
        cost_provenance: UsageProvenance::Unknown,
        cost_is_estimated: false,
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: None,
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });
    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-2".to_string(),
        endpoint_id: "relay-default".to_string(),
        model: None,
        input_tokens: 40,
        output_tokens: 15,
        cache_read_tokens: 5,
        cache_write_tokens: 2,
        reasoning_tokens: 0,
        estimated_cost: String::new(),
        cost_provenance: UsageProvenance::Unknown,
        cost_is_estimated: false,
        pricing_profile_id: None,
        declared_capability_requirements: None,
        effective_capability_result: None,
        final_upstream_status: None,
        final_upstream_error_code: None,
        final_upstream_error_reason: None,
    });

    let detail = usage_request_detail_from_runtime(&runtime, "req-1").expect("existing request");
    assert_eq!(detail.request_id, "req-1");
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0123"));

    assert!(
        usage_request_detail_from_runtime(&runtime, "req-missing").is_none(),
        "unknown request should return None"
    );

    let history = usage_request_history_from_runtime(&runtime, Some(1));
    assert_eq!(history.len(), 1);

    let ledger = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: Some("relay-default".to_string()),
            request_id_prefix: None,
            limit: None,
        }),
    );
    assert_eq!(ledger.entries.len(), 1);
    assert_eq!(ledger.total_cost.provenance, UsageProvenance::Unknown);
    assert_eq!(ledger.total_cost.amount, None);
}

#[tokio::test]
async fn usage_commands_reflect_runtime_gateway_requests_only() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    assert!(
        usage_request_history_from_runtime(&runtime, None).is_empty(),
        "usage history should be empty before any gateway traffic"
    );

    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("switch mode");

    let platform_secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", format!("bearer {}", platform_secret))
                .body(Body::empty())
                .expect("gateway request"),
        )
        .await
        .expect("gateway response");

    assert_eq!(response.status(), StatusCode::OK);

    let history = usage_request_history_from_runtime(&runtime, None);
    assert_eq!(
        history.len(),
        1,
        "exactly one data-plane request should be recorded after one request"
    );
    assert!(
        history[0].request_id.starts_with("default:"),
        "successful request ids should preserve key-prefixed correlation semantics"
    );
    assert!(
        history[0].request_id.contains(":unrouted:"),
        "request ids should remain stable from acceptance through routing without rewrites"
    );

    let relay_entries = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: Some("relay-default".to_string()),
            request_id_prefix: None,
            limit: None,
        }),
    );
    assert_eq!(relay_entries.entries.len(), 1);
    let key_entries = usage_ledger_from_runtime(
        &runtime,
        Some(UsageLedgerQuery {
            endpoint_id: None,
            request_id_prefix: Some("default:".to_string()),
            limit: None,
        }),
    );
    assert_eq!(key_entries.entries.len(), 1);
    assert!(usage_request_detail_from_runtime(
        &runtime,
        relay_entries.entries[0].request_id.as_str()
    )
    .is_some());

    runtime
        .set_current_mode(RoutingMode::Hybrid)
        .expect("switch mode again");
    let history_after_control_plane = usage_request_history_from_runtime(&runtime, None);
    assert_eq!(
        history_after_control_plane.len(),
        1,
        "control-plane mode changes must not add usage request entries"
    );
}

#[tokio::test]
async fn unauthorized_gateway_request_does_not_create_usage_record() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    assert!(
        usage_request_history_from_runtime(&runtime, None).is_empty(),
        "history should start empty"
    );

    let response = runtime
        .loopback_gateway()
        .router()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/codex/request")
                .header("authorization", "bearer wrong-platform-secret")
                .body(Body::empty())
                .expect("gateway request"),
        )
        .await
        .expect("gateway response");

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    assert!(
        usage_request_history_from_runtime(&runtime, None).is_empty(),
        "unauthorized requests must not be recorded"
    );
}

#[tokio::test]
async fn diagnostics_export_manifest_redacts_plain_platform_secret() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let platform_secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("platform key secret");
    std::fs::create_dir_all(runtime.runtime_log().log_dir.as_path())
        .expect("create runtime log directory");
    std::fs::write(
        runtime
            .runtime_log()
            .log_dir
            .join(format!("Bearer {platform_secret}.log")),
        "trigger bearer redaction",
    )
    .expect("write redaction trigger log file");

    let _ = export_runtime_diagnostics_from_runtime(&runtime).expect("export diagnostics");
    let manifest_path = runtime
        .runtime_log()
        .log_dir
        .join("diagnostics")
        .join("diagnostics-manifest.txt");
    let manifest_contents =
        std::fs::read_to_string(&manifest_path).expect("read diagnostics manifest");

    assert!(
        !manifest_contents.contains(platform_secret.as_str()),
        "diagnostics manifest should not leak platform secret"
    );
    assert!(
        manifest_contents.contains("bearer [redacted]"),
        "diagnostics manifest should redact bearer token values"
    );
}

#[tokio::test]
async fn logs_commands_return_structured_errors_when_log_root_is_invalid() {
    let isolated_root = isolated_test_root("command-logs-errors");
    std::fs::create_dir_all(&isolated_root).expect("create isolated root");
    let metadata_db_path = isolated_root.join("state-metadata.sqlite3");
    let invalid_log_root = isolated_root.join("logs-file");
    std::fs::write(&invalid_log_root, "not a directory").expect("create invalid log root file");
    let metadata_runtime = runtime_for_paths(&metadata_db_path, &invalid_log_root).await;

    let metadata_error = runtime_log_metadata_from_runtime(&metadata_runtime)
        .expect_err("log metadata should fail when log root is not a directory");
    assert_structured_command_error(
        metadata_error,
        "config.unknown",
        ErrorCategory::ConfigError,
        "Failed to read runtime log metadata.",
        "command=get_runtime_log_metadata;log_dir=",
    );
    drop(metadata_runtime);

    let diagnostics_db_path = isolated_root.join("state-diagnostics.sqlite3");
    let diagnostics_log_root = isolated_root.join("logs-diagnostics");
    std::fs::create_dir_all(&diagnostics_log_root).expect("create diagnostics log root directory");
    std::fs::write(diagnostics_log_root.join("gateway.log"), "entry").expect("seed log file");
    std::fs::write(diagnostics_log_root.join("diagnostics"), "conflict")
        .expect("create diagnostics path conflict");
    let diagnostics_runtime = runtime_for_paths(&diagnostics_db_path, &diagnostics_log_root).await;

    let diagnostics_error = export_runtime_diagnostics_from_runtime(&diagnostics_runtime)
        .expect_err("diagnostics export should fail when diagnostics directory cannot be created");
    assert_structured_command_error(
        diagnostics_error,
        "config.unknown",
        ErrorCategory::ConfigError,
        "Failed to create diagnostics directory.",
        "command=export_runtime_diagnostics;operation=create_dir_all",
    );
    drop(diagnostics_runtime);

    let _ = std::fs::remove_dir_all(&isolated_root);
}
