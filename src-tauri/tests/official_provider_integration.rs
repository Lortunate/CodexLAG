use codexlag_lib::bootstrap::bootstrap_runtime_for_test;
use codexlag_lib::commands::accounts::{
    import_official_account_login_from_runtime, OfficialAccountImportInput,
};
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn imported_official_account_exposes_runtime_status_and_identity() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let detail = codexlag_lib::commands::accounts::get_account_capability_detail_from_runtime(
        &runtime,
        "official-primary".to_string(),
    )
    .expect("official capability detail");

    assert_eq!(detail.provider, "openai");
    assert!(detail.refresh_capability.is_some());
    assert!(!detail.status.is_empty());
    assert!(detail.account_identity.is_some());
}

#[tokio::test]
async fn imported_official_account_runtime_path_requires_stored_session_secret() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let account_id = "official-runtime-secret-check".to_string();
    let session_ref = "credential://official/session/runtime-secret-check".to_string();
    let token_ref = "credential://official/token/runtime-secret-check".to_string();

    import_official_account_login_from_runtime(
        &runtime,
        OfficialAccountImportInput {
            account_id: account_id.clone(),
            name: "Runtime Secret Check".to_string(),
            provider: "openai".to_string(),
            session_credential_ref: session_ref.clone(),
            token_credential_ref: token_ref.clone(),
            account_identity: Some("runtime-secret-check@example.test".to_string()),
            auth_mode: Some("api_key".to_string()),
        },
    )
    .expect("import official account");

    let missing = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate(account_id.as_str());
    assert!(
        missing.is_err(),
        "runtime should require stored session/token credentials before selecting imported official account"
    );

    runtime
        .app_state()
        .store_secret(&SecretKey::new(session_ref), "session-secret".to_string())
        .expect("store official session secret");
    runtime
        .app_state()
        .store_secret(&SecretKey::new(token_ref), "token-secret".to_string())
        .expect("store official token secret");

    let hydrated = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate(account_id.as_str())
        .expect("runtime should hydrate official session when both secrets exist");
    assert_eq!(hydrated.session_id, format!("session:{account_id}"));
}
