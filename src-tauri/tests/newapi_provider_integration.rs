use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::relays::{
    add_relay_from_runtime, refresh_relay_balance_from_runtime, RelayUpsertInput,
};

#[tokio::test]
async fn newapi_relay_balance_refresh_uses_adapter_logic_instead_of_fixtures() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    let snapshot = refresh_relay_balance_from_runtime(&runtime, "relay-newapi".to_string())
        .expect("refresh relay balance");
    assert_eq!(snapshot.relay_id, "relay-newapi");
    assert_eq!(snapshot.endpoint, "http://127.0.0.1:8787");
}

#[tokio::test]
async fn relay_provider_path_can_be_selected_from_runtime_inventory() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");
    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates.iter().any(|candidate| candidate.id == "relay-newapi"),
        "relay runtime inventory should include relay-newapi"
    );
}

#[tokio::test]
async fn managed_newapi_relay_balance_refresh_requires_stored_api_key_secret() {
    let runtime = bootstrap_runtime_for_test().await.expect("bootstrap runtime");

    add_relay_from_runtime(
        &runtime,
        RelayUpsertInput {
            relay_id: "relay-newapi-missing-secret".to_string(),
            name: "Relay Missing Secret".to_string(),
            endpoint: "https://relay.example".to_string(),
            adapter: Some("newapi".to_string()),
            api_key_credential_ref: Some(
                "credential://relay/api-key/relay-newapi-missing-secret".to_string(),
            ),
        },
    )
    .expect("create relay");

    let error =
        refresh_relay_balance_from_runtime(&runtime, "relay-newapi-missing-secret".to_string())
            .expect_err("missing relay api-key secret should fail");
    assert!(
        error
            .to_string()
            .contains("secret 'credential://relay/api-key/relay-newapi-missing-secret' not found")
    );
}
