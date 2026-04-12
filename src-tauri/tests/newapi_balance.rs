use codexlag_lib::providers::relay::{
    normalize_relay_balance_response, relay_balance_capability, RelayBalanceAdapter,
    RelayBalanceCapability,
};

#[test]
fn normalize_relay_balance_response_maps_newapi_amounts() {
    let normalized = normalize_relay_balance_response(
        RelayBalanceAdapter::NewApi,
        r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#,
    )
    .expect("normalized")
    .expect("newapi balance");

    assert_eq!(normalized.total, "25.00");
    assert_eq!(normalized.used, "7.50");
}

#[test]
fn normalize_relay_balance_response_rejects_malformed_newapi_payload() {
    let error = normalize_relay_balance_response(
        RelayBalanceAdapter::NewApi,
        r#"{"data":{"total_balance":"25.00"}}"#,
    )
    .expect_err("missing used_balance should fail");

    assert!(error.is_payload_error());
}

#[test]
fn normalize_relay_balance_response_returns_none_for_relays_without_balance_support() {
    let normalized =
        normalize_relay_balance_response(RelayBalanceAdapter::NoBalance, r#"{"ignored":true}"#)
            .expect("unsupported relay type should be handled");

    assert_eq!(normalized, None);
}

#[test]
fn relay_balance_capability_tracks_supported_adapter_and_no_balance_cases() {
    assert_eq!(
        relay_balance_capability(RelayBalanceAdapter::NewApi),
        RelayBalanceCapability::Queryable {
            adapter: RelayBalanceAdapter::NewApi,
        }
    );
    assert_eq!(
        relay_balance_capability(RelayBalanceAdapter::NoBalance),
        RelayBalanceCapability::Unsupported
    );
}
