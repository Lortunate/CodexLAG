use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use codexlag_lib::commands::accounts::get_account_capability_detail;
use codexlag_lib::commands::logs::{
    usage_ledger_from_runtime, usage_request_detail_from_runtime,
    usage_request_history_from_runtime,
};
use codexlag_lib::commands::relays::{get_relay_capability_detail, RelayCapabilityDetail};
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance, UsageRecordInput};
use codexlag_lib::providers::official::OfficialBalanceCapability;
use codexlag_lib::providers::relay::{RelayBalanceAdapter, RelayBalanceCapability};
use codexlag_lib::{
    bootstrap::bootstrap_runtime_for_test, routing::policy::RoutingMode, secret_store::SecretKey,
};
use tower::ServiceExt;

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
        history[0].request_id.contains(":relay-default:"),
        "successful request ids should preserve endpoint segment compatibility"
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
