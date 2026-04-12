use codexlag_lib::commands::accounts::{refresh_account_balance, AccountBalanceAvailability};
use codexlag_lib::commands::relays::{refresh_relay_balance, RelayBalanceAvailability};

#[test]
fn refresh_account_balance_marks_official_accounts_as_non_queryable() {
    let snapshot = refresh_account_balance("official-primary".to_string())
        .expect("known official account should succeed");

    assert_eq!(snapshot.account_id, "official-primary");
    assert_eq!(snapshot.provider, "openai");
    assert_eq!(
        snapshot.balance,
        AccountBalanceAvailability::NonQueryable {
            reason: "official accounts do not expose a balance endpoint".to_string(),
        }
    );
    let refreshed_at = snapshot
        .refreshed_at
        .parse::<u64>()
        .expect("refreshed_at should be unix seconds");
    assert!(refreshed_at > 1_700_000_000);
}

#[test]
fn refresh_account_balance_returns_explicit_unknown_id_error() {
    let error = refresh_account_balance("unknown-account".to_string())
        .expect_err("unknown account should be reported");
    assert_eq!(error, "unknown account id: unknown-account");
}

#[test]
fn refresh_relay_balance_handles_supported_and_unsupported_apis() {
    let supported = refresh_relay_balance("relay-newapi".to_string())
        .expect("known relay with balance api should succeed");
    assert_eq!(supported.relay_id, "relay-newapi");
    assert!(matches!(
        supported.balance,
        RelayBalanceAvailability::Queryable {
            ref balance,
            ref adapter
        } if balance.total == "25.00" && balance.used == "7.50" && adapter == "newapi"
    ));

    let unsupported = refresh_relay_balance("relay-nobalance".to_string())
        .expect("known relay without balance api should still succeed");
    assert_eq!(unsupported.relay_id, "relay-nobalance");
    assert_eq!(
        unsupported.balance,
        RelayBalanceAvailability::Unsupported {
            reason: "relay does not provide a balance endpoint".to_string(),
        }
    );
}

#[test]
fn refresh_relay_balance_distinguishes_unknown_id_and_parse_failure() {
    let unknown_error = refresh_relay_balance("relay-missing".to_string())
        .expect_err("unknown relay should be reported");
    assert_eq!(unknown_error, "unknown relay id: relay-missing");

    let parse_error = refresh_relay_balance("relay-badpayload".to_string())
        .expect_err("bad payload relay should report parser failure");
    assert!(
        parse_error.starts_with("relay balance payload parse error:"),
        "unexpected parse error: {parse_error}"
    );
}
