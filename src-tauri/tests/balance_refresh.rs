use codexlag_lib::commands::accounts::{refresh_account_balance, AccountBalanceAvailability};
use codexlag_lib::commands::relays::refresh_relay_balance;

#[test]
fn refresh_account_balance_marks_official_accounts_as_non_queryable() {
    let snapshot =
        refresh_account_balance("official-primary".to_string()).expect("known official account");

    assert_eq!(snapshot.account_id, "official-primary");
    assert_eq!(snapshot.provider, "openai");
    assert_eq!(
        snapshot.balance,
        AccountBalanceAvailability::NonQueryable {
            reason: "official accounts do not expose a balance endpoint".to_string(),
        }
    );
}

#[test]
fn refresh_relay_balance_handles_supported_and_unsupported_apis() {
    let supported =
        refresh_relay_balance("relay-newapi".to_string()).expect("known relay with balance api");
    assert_eq!(supported.relay_id, "relay-newapi");
    assert_eq!(
        supported.balance.as_ref().map(|value| value.total.as_str()),
        Some("25.00")
    );
    assert_eq!(
        supported.balance.as_ref().map(|value| value.used.as_str()),
        Some("7.50")
    );

    let unsupported = refresh_relay_balance("relay-nobalance".to_string())
        .expect("known relay without balance api");
    assert_eq!(unsupported.relay_id, "relay-nobalance");
    assert_eq!(unsupported.balance, None);
}
