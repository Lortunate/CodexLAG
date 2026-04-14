use codexlag_lib::bootstrap::bootstrap_state_for_test;
use codexlag_lib::models::{ImportedOfficialAccount, OfficialSession};
use codexlag_lib::providers::inventory::project_provider_inventory_summary;
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn provider_inventory_projects_registered_models_for_official_and_generic_accounts() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    state
        .save_imported_official_account(account(
            "official-zeta",
            "Zeta Official",
            "openai",
            "credential://official/session/official-zeta",
            "credential://official/token/official-zeta",
        ))
        .expect("save official account");
    state
        .store_secret(
            &SecretKey::new("credential://official/session/official-zeta".to_string()),
            "session-secret".to_string(),
        )
        .expect("store official session secret");
    state
        .store_secret(
            &SecretKey::new("credential://official/token/official-zeta".to_string()),
            r#"{"api_key":"official-key"}"#.to_string(),
        )
        .expect("store official token secret");

    state
        .save_imported_official_account(account(
            "generic-alpha",
            "Alpha Generic",
            "generic_openai",
            "credential://official/session/generic-alpha",
            "credential://official/token/generic-alpha",
        ))
        .expect("save generic account");
    state
        .store_secret(
            &SecretKey::new("credential://official/token/generic-alpha".to_string()),
            r#"{
                "api_key":"generic-key",
                "base_url":"https://gateway.example.test/",
                "manual_models":[" gpt-4o-mini ","","gpt-4.1-mini","gpt-4o-mini"]
            }"#
            .to_string(),
        )
        .expect("store generic token secret");

    let summary = project_provider_inventory_summary(&state);
    let endpoint_ids: Vec<&str> = summary
        .providers
        .iter()
        .map(|provider| provider.endpoint_id.as_str())
        .collect();
    assert_eq!(
        endpoint_ids,
        vec!["generic-alpha", "official-zeta"],
        "provider inventory should be sorted deterministically by display name"
    );

    let generic = &summary.providers[0];
    assert_eq!(generic.provider_id, "generic_openai");
    assert!(generic.registered);
    assert!(
        generic.available,
        "generic provider should only require a token secret"
    );
    assert_eq!(
        generic.base_url.as_deref(),
        Some("https://gateway.example.test/v1")
    );
    assert_eq!(
        generic.model_ids,
        vec!["gpt-4o-mini".to_string(), "gpt-4.1-mini".to_string()]
    );
    assert_eq!(generic.feature_capabilities.len(), 2);

    let official = &summary.providers[1];
    assert_eq!(official.provider_id, "openai");
    assert!(official.registered);
    assert!(official.available);
    assert_eq!(official.base_url, None);
    assert_eq!(official.model_ids, vec!["gpt-5-mini".to_string()]);
}

#[tokio::test]
async fn provider_inventory_marks_unregistered_providers_as_unavailable() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    state
        .save_imported_official_account(account(
            "unsupported-provider",
            "Unsupported Provider",
            "custom_provider",
            "credential://official/session/unsupported-provider",
            "credential://official/token/unsupported-provider",
        ))
        .expect("save unsupported account");
    state
        .store_secret(
            &SecretKey::new("credential://official/token/unsupported-provider".to_string()),
            r#"{"api_key":"unsupported-key"}"#.to_string(),
        )
        .expect("store unsupported token secret");

    let summary = project_provider_inventory_summary(&state);
    let unsupported = summary
        .providers
        .iter()
        .find(|provider| provider.endpoint_id == "unsupported-provider")
        .expect("unsupported provider should still be projected");

    assert!(!unsupported.registered);
    assert!(
        !unsupported.available,
        "unregistered providers should not project as available"
    );
    assert!(unsupported.model_ids.is_empty());
}

fn account(
    account_id: &str,
    name: &str,
    provider: &str,
    session_credential_ref: &str,
    token_credential_ref: &str,
) -> ImportedOfficialAccount {
    ImportedOfficialAccount {
        account_id: account_id.to_string(),
        name: name.to_string(),
        provider: provider.to_string(),
        session: OfficialSession {
            session_id: format!("session:{account_id}"),
            account_identity: Some(format!("{account_id}@example.test")),
            auth_mode: None,
            refresh_capability: Some(true),
            quota_capability: Some(false),
            last_verified_at_ms: None,
            status: "active".to_string(),
        },
        session_credential_ref: session_credential_ref.to_string(),
        token_credential_ref: token_credential_ref.to_string(),
    }
}
