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
    export_runtime_diagnostics_from_runtime, usage_ledger_from_runtime,
    usage_request_detail_from_runtime, usage_request_history_from_runtime,
};
use codexlag_lib::commands::policies::{
    policy_summaries_from_state, update_policy_from_runtime, PolicyUpdateInput,
};
use codexlag_lib::commands::relays::{
    add_relay_from_runtime, delete_relay_from_runtime, get_relay_capability_detail_from_runtime,
    list_relays_from_runtime, test_relay_connection_from_runtime, update_relay_from_runtime,
    RelayCapabilityDetail, RelayUpsertInput,
};
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
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
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
    assert_eq!(account_error, "unknown account id: unknown-account");

    let relay_error =
        get_relay_capability_detail_from_runtime(&runtime, "relay-missing".to_string())
            .expect_err("unknown relay should be reported");
    assert_eq!(relay_error, "unknown relay id: relay-missing");

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
    assert_eq!(
        invalid_error,
        "session credential ref must start with 'credential://official/session/'"
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
    assert_eq!(
        reserved_error,
        "account_id is reserved and cannot be imported: official-primary"
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
    assert!(
        error.contains("failed to persist credential reference"),
        "expected credential persistence failure error, got: {error}"
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
    assert_eq!(unknown_error, format!("unknown relay id: {relay_id}"));

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
    assert_eq!(
        invalid_error,
        "relay endpoint must start with 'http://' or 'https://'"
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
    assert_eq!(unknown_error, format!("unknown key id: missing-{suffix}"));

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
    assert_eq!(invalid_error, "retry_budget must be greater than 0");

    let unknown_endpoint_error = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            selection_order: vec!["official-primary".to_string(), "relay-missing".to_string()],
            ..updated.clone()
        },
    )
    .expect_err("selection_order should reject unknown endpoint ids");
    assert_eq!(
        unknown_endpoint_error,
        "unknown selection_order endpoint id: relay-missing"
    );

    let unknown_error = update_policy_from_runtime(
        &first_runtime,
        PolicyUpdateInput {
            policy_id: "missing-policy".to_string(),
            ..updated
        },
    )
    .expect_err("unknown policy id should be explicit");
    assert_eq!(unknown_error, "unknown policy id: missing-policy");

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
        input_tokens: 120,
        output_tokens: 30,
        cache_read_tokens: 10,
        cache_write_tokens: 0,
        estimated_cost: "0.0123".to_string(),
    });
    runtime.record_usage_request(UsageRecordInput {
        request_id: "req-2".to_string(),
        endpoint_id: "relay-default".to_string(),
        input_tokens: 40,
        output_tokens: 15,
        cache_read_tokens: 5,
        cache_write_tokens: 2,
        estimated_cost: String::new(),
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
