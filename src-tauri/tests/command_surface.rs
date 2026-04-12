use codexlag_lib::commands::accounts::get_account_capability_detail;
use codexlag_lib::commands::logs::{
    get_usage_request_detail, list_usage_request_history, query_usage_ledger,
};
use codexlag_lib::commands::relays::{get_relay_capability_detail, RelayCapabilityDetail};
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance};
use codexlag_lib::providers::official::OfficialBalanceCapability;
use codexlag_lib::providers::relay::{RelayBalanceAdapter, RelayBalanceCapability};

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
fn usage_commands_expose_request_detail_history_and_ledger_provenance() {
    let detail = get_usage_request_detail("req-1".to_string()).expect("existing request");
    assert_eq!(detail.request_id, "req-1");
    assert_eq!(detail.cost.provenance, UsageProvenance::Estimated);
    assert_eq!(detail.cost.amount.as_deref(), Some("0.0123"));

    assert!(
        get_usage_request_detail("req-missing".to_string()).is_none(),
        "unknown request should return None"
    );

    let history = list_usage_request_history(Some(1));
    assert_eq!(history.len(), 1);

    let ledger = query_usage_ledger(Some(UsageLedgerQuery {
        endpoint_id: Some("relay-1".to_string()),
        request_id_prefix: None,
        limit: None,
    }));
    assert_eq!(ledger.entries.len(), 1);
    assert_eq!(ledger.total_cost.provenance, UsageProvenance::Unknown);
    assert_eq!(ledger.total_cost.amount, None);
}
