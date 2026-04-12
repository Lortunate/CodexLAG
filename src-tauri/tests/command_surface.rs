use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use codexlag_lib::commands::accounts::{
    get_account_capability_detail, import_official_account_login, list_accounts,
    OfficialAccountImportInput,
};
use codexlag_lib::commands::keys::{
    create_platform_key, disable_platform_key, enable_platform_key, list_platform_keys,
    CreatePlatformKeyInput,
};
use codexlag_lib::commands::logs::{
    export_runtime_diagnostics_from_runtime, usage_ledger_from_runtime,
    usage_request_detail_from_runtime, usage_request_history_from_runtime,
};
use codexlag_lib::commands::policies::{update_policy, PolicyUpdateInput};
use codexlag_lib::commands::relays::{
    add_relay, delete_relay, get_relay_capability_detail, test_relay_connection, update_relay,
    RelayCapabilityDetail, RelayUpsertInput,
};
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance, UsageRecordInput};
use codexlag_lib::providers::official::OfficialBalanceCapability;
use codexlag_lib::providers::relay::{RelayBalanceAdapter, RelayBalanceCapability};
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test, routing::policy::RoutingMode, secret_store::SecretKey,
};
use std::time::{SystemTime, UNIX_EPOCH};
use tower::ServiceExt;

fn unique_suffix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("wall clock after unix epoch")
        .as_nanos();
    nanos.to_string()
}

#[test]
fn account_and_relay_capability_details_expose_balance_metadata() {
    let account = get_account_capability_detail("official-primary".to_string())
        .expect("official account should succeed");
    assert_eq!(account.account_id, "official-primary");
    assert_eq!(account.refresh_capability, Some(true));
    assert_eq!(
        account.balance_capability,
        OfficialBalanceCapability::NonQueryable
    );

    let relay =
        get_relay_capability_detail("relay-newapi".to_string()).expect("relay should succeed");
    assert_eq!(relay.relay_id, "relay-newapi");
    assert_eq!(
        relay.balance_capability,
        RelayBalanceCapability::Queryable {
            adapter: RelayBalanceAdapter::NewApi
        }
    );

    let unsupported = get_relay_capability_detail("relay-nobalance".to_string())
        .expect("unsupported relay should still return capability details");
    assert_eq!(
        unsupported,
        RelayCapabilityDetail {
            relay_id: "relay-nobalance".to_string(),
            endpoint: "https://relay.example.test".to_string(),
            balance_capability: RelayBalanceCapability::Unsupported,
        }
    );
}

#[test]
fn capability_detail_commands_return_explicit_errors_for_unknown_ids() {
    let account_error = get_account_capability_detail("unknown-account".to_string())
        .expect_err("unknown account should be reported");
    assert_eq!(account_error, "unknown account id: unknown-account");

    let relay_error = get_relay_capability_detail("relay-missing".to_string())
        .expect_err("unknown relay should be reported");
    assert_eq!(relay_error, "unknown relay id: relay-missing");
}

#[test]
fn account_import_login_command_validates_and_persists_entry() {
    let suffix = unique_suffix();
    let account_id = format!("official-imported-{suffix}");
    let imported = import_official_account_login(OfficialAccountImportInput {
        account_id: account_id.clone(),
        name: "Imported Official Account".to_string(),
        provider: "openai".to_string(),
        session_credential_ref: format!("credential://official/session/{suffix}"),
        token_credential_ref: format!("credential://official/token/{suffix}"),
        account_identity: Some(format!("user-{suffix}@example.test")),
        auth_mode: Some("oauth".to_string()),
    })
    .expect("account import should succeed");
    assert_eq!(imported.account_id, account_id);

    let accounts = list_accounts();
    assert!(
        accounts
            .iter()
            .any(|account| account.account_id == account_id),
        "imported account should be visible in list_accounts"
    );

    let invalid_error = import_official_account_login(OfficialAccountImportInput {
        account_id: format!("official-invalid-{suffix}"),
        name: "Invalid Official Account".to_string(),
        provider: "openai".to_string(),
        session_credential_ref: "credential://official/token/not-a-session".to_string(),
        token_credential_ref: format!("credential://official/token/{suffix}-bad"),
        account_identity: None,
        auth_mode: None,
    })
    .expect_err("invalid session credential ref should be rejected");
    assert_eq!(
        invalid_error,
        "session credential ref must start with 'credential://official/session/'"
    );
}

#[test]
fn relay_crud_and_test_connection_commands_validate_unknown_ids() {
    let suffix = unique_suffix();
    let relay_id = format!("relay-managed-{suffix}");
    let created = add_relay(RelayUpsertInput {
        relay_id: relay_id.clone(),
        name: "Managed Relay".to_string(),
        endpoint: format!("https://relay-{suffix}.example.test"),
        adapter: Some("newapi".to_string()),
    })
    .expect("relay add should succeed");
    assert_eq!(created.relay_id, relay_id);

    let tested = test_relay_connection(relay_id.clone()).expect("relay test should succeed");
    assert_eq!(tested.status, "ok");

    let updated = update_relay(RelayUpsertInput {
        relay_id: relay_id.clone(),
        name: "Managed Relay Updated".to_string(),
        endpoint: format!("https://relay-updated-{suffix}.example.test"),
        adapter: Some("newapi".to_string()),
    })
    .expect("relay update should succeed");
    assert_eq!(updated.name, "Managed Relay Updated");

    delete_relay(relay_id.clone()).expect("relay delete should succeed");

    let unknown_error =
        test_relay_connection(relay_id.clone()).expect_err("deleted relay should be unknown");
    assert_eq!(unknown_error, format!("unknown relay id: {relay_id}"));

    let invalid_error = add_relay(RelayUpsertInput {
        relay_id: format!("relay-invalid-{suffix}"),
        name: "Invalid Relay".to_string(),
        endpoint: "ftp://relay.example.test".to_string(),
        adapter: Some("newapi".to_string()),
    })
    .expect_err("invalid endpoint scheme should be rejected");
    assert_eq!(
        invalid_error,
        "relay endpoint must start with 'http://' or 'https://'"
    );
}

#[test]
fn key_inventory_commands_create_list_disable_and_enable() {
    let suffix = unique_suffix();
    let key_id = format!("pk-{suffix}");
    let key_name = format!("managed-key-{suffix}");
    let created = create_platform_key(CreatePlatformKeyInput {
        key_id: key_id.clone(),
        name: key_name.clone(),
        policy_id: "default".to_string(),
        allowed_mode: "hybrid".to_string(),
    })
    .expect("key create should succeed");
    assert!(created.enabled);

    let listed = list_platform_keys();
    assert!(
        listed.iter().any(|entry| entry.id == key_id),
        "created key should be visible in list_platform_keys"
    );

    let disabled = disable_platform_key(key_id.clone()).expect("disable should succeed");
    assert!(!disabled.enabled);
    let enabled = enable_platform_key(key_id.clone()).expect("enable should succeed");
    assert!(enabled.enabled);

    let unknown_error = disable_platform_key(format!("missing-{suffix}"))
        .expect_err("unknown ids should return explicit errors");
    assert_eq!(unknown_error, format!("unknown key id: missing-{suffix}"));
}

#[test]
fn policy_update_command_enforces_strict_validation() {
    let updated = update_policy(PolicyUpdateInput {
        policy_id: "default".to_string(),
        name: "Default Policy".to_string(),
        selection_order: vec!["official-primary".to_string(), "relay-newapi".to_string()],
        cross_pool_fallback: true,
        retry_budget: 2,
        timeout_open_after: 3,
        server_error_open_after: 3,
        cooldown_ms: 30_000,
        half_open_after_ms: 15_000,
        success_close_after: 1,
    })
    .expect("default policy update should succeed");
    assert_eq!(updated.retry_budget, 2);

    let invalid_error = update_policy(PolicyUpdateInput {
        retry_budget: 0,
        ..updated.clone()
    })
    .expect_err("retry_budget must be strictly positive");
    assert_eq!(invalid_error, "retry_budget must be greater than 0");

    let unknown_error = update_policy(PolicyUpdateInput {
        policy_id: "missing-policy".to_string(),
        ..updated
    })
    .expect_err("unknown policy id should be explicit");
    assert_eq!(unknown_error, "unknown policy id: missing-policy");
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
