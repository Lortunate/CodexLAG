use codexlag_lib::bootstrap::{
    bootstrap_state_for_test, bootstrap_state_with_provider_inventory_for_test,
};
use codexlag_lib::models::{
    relay_api_key_credential_ref, ImportedOfficialAccount, ManagedRelay, OfficialAuthMode,
    OfficialSession, RelayBalanceAdapter,
};
use codexlag_lib::routing::candidates::build_runtime_candidates;

#[tokio::test]
async fn runtime_candidates_are_built_from_persisted_accounts_and_relays() {
    let state = bootstrap_state_with_provider_inventory_for_test()
        .await
        .expect("bootstrap state");

    let candidates = build_runtime_candidates(&state);
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "official-primary"),
        "official inventory should produce candidate official-primary"
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "relay-newapi"),
        "relay inventory should produce candidate relay-newapi"
    );
}

#[tokio::test]
async fn runtime_candidates_drop_default_entries_once_persisted_inventory_is_deleted() {
    let mut state = bootstrap_state_with_provider_inventory_for_test()
        .await
        .expect("bootstrap state");

    let removed_account = state
        .repositories_mut()
        .delete_imported_official_account("official-primary")
        .expect("delete default official account");
    assert!(
        removed_account,
        "default official account should be persisted"
    );

    let removed_relay = state
        .delete_managed_relay("relay-newapi")
        .expect("delete default relay");
    assert!(removed_relay, "default relay should be persisted");

    let candidates = build_runtime_candidates(&state);
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.id != "official-primary"),
        "deleted official inventory should no longer inject official-primary"
    );
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.id != "relay-newapi"),
        "deleted relay inventory should no longer inject relay-newapi"
    );
}

#[tokio::test]
async fn runtime_candidates_reflect_missing_provider_credentials_as_unavailable() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    state
        .save_imported_official_account(ImportedOfficialAccount {
            account_id: "official-missing-secrets".to_string(),
            name: "Official Missing Secrets".to_string(),
            provider: "openai".to_string(),
            session: OfficialSession {
                session_id: "session:official-missing-secrets".to_string(),
                account_identity: Some("missing@example.test".to_string()),
                auth_mode: Some(OfficialAuthMode::ApiKey),
                refresh_capability: Some(true),
                quota_capability: Some(false),
                last_verified_at_ms: None,
                status: "active".to_string(),
            },
            session_credential_ref: "credential://official/session/official-missing-secrets"
                .to_string(),
            token_credential_ref: "credential://official/token/official-missing-secrets"
                .to_string(),
        })
        .expect("save official account");
    state
        .save_managed_relay(ManagedRelay {
            relay_id: "relay-missing-secret".to_string(),
            name: "Relay Missing Secret".to_string(),
            endpoint: "https://relay.example.test".to_string(),
            adapter: RelayBalanceAdapter::NewApi,
            api_key_credential_ref: relay_api_key_credential_ref("relay-missing-secret"),
        })
        .expect("save relay");

    let candidates = build_runtime_candidates(&state);
    let official = candidates
        .iter()
        .find(|candidate| candidate.id == "official-missing-secrets")
        .expect("official candidate should exist");
    assert!(
        !official.available,
        "official candidates without stored session/token secrets should be unavailable"
    );

    let relay = candidates
        .iter()
        .find(|candidate| candidate.id == "relay-missing-secret")
        .expect("relay candidate should exist");
    assert!(
        !relay.available,
        "relay candidates without stored api-key secrets should be unavailable"
    );
}
