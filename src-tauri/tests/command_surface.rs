use codexlag_lib::commands::accounts::get_account_capability_detail;
use codexlag_lib::commands::logs::{
    get_usage_request_detail, list_usage_request_history, query_usage_ledger,
};
use codexlag_lib::commands::relays::get_relay_capability_detail;
use codexlag_lib::logging::usage::{UsageLedgerQuery, UsageProvenance};

#[test]
fn account_and_relay_capability_details_expose_balance_metadata() {
    let account =
        get_account_capability_detail("official-primary".to_string()).expect("official account");
    assert_eq!(account.account_id, "official-primary");
    assert_eq!(account.refresh_capability, Some(true));
    assert!(!account.balance_queryable);

    let relay = get_relay_capability_detail("relay-newapi".to_string()).expect("relay");
    assert_eq!(relay.relay_id, "relay-newapi");
    assert_eq!(relay.balance_adapter.as_deref(), Some("newapi"));
    assert!(relay.balance_queryable);
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
