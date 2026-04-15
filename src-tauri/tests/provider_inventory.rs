use codexlag_lib::bootstrap::bootstrap_state_for_test;
use codexlag_lib::auth::session_store::ProviderSessionStore;
use codexlag_lib::models::{ImportedOfficialAccount, OfficialSession, ProviderSessionSummary};
use codexlag_lib::providers::inventory::project_provider_inventory_summary;
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn provider_inventory_projects_registered_models_for_official_and_generic_accounts() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    state
        .save_imported_official_account(account(
            "official-zeta",
            "Zeta Official",
            "openai_official",
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
            "generic_openai_compatible",
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
        .accounts
        .iter()
        .map(|provider| provider.account_id.as_str())
        .collect();
    assert_eq!(
        endpoint_ids,
        vec!["generic-alpha", "official-zeta"],
        "provider inventory should be sorted deterministically by display name"
    );

    let generic = &summary.accounts[0];
    assert_eq!(generic.provider_id, "generic_openai_compatible");
    assert!(generic.registered);
    assert!(
        generic.available,
        "generic provider should only require a token secret"
    );
    assert_eq!(
        generic.base_url.as_deref(),
        Some("https://gateway.example.test/v1")
    );
    let generic_models = summary
        .models
        .iter()
        .filter(|model| model.account_id == "generic-alpha")
        .collect::<Vec<_>>();
    assert_eq!(
        generic_models
            .iter()
            .map(|model| model.model_id.clone())
            .collect::<Vec<_>>(),
        vec!["gpt-4.1-mini".to_string(), "gpt-4o-mini".to_string()]
    );
    assert!(generic_models.iter().all(|model| model.source == "manual"));

    let official = &summary.accounts[1];
    assert_eq!(official.provider_id, "openai_official");
    assert!(official.registered);
    assert!(official.available);
    assert_eq!(official.base_url, None);
    let official_models = summary
        .models
        .iter()
        .filter(|model| model.account_id == "official-zeta")
        .collect::<Vec<_>>();
    assert_eq!(
        official_models
            .iter()
            .map(|model| model.model_id.clone())
            .collect::<Vec<_>>(),
        vec!["gpt-5-mini".to_string()]
    );
    assert!(official_models.iter().all(|model| model.source == "default"));
}

#[tokio::test]
async fn provider_inventory_projects_stored_openai_provider_sessions() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    ProviderSessionStore::save(
        &mut state,
        ProviderSessionSummary {
            provider_id: "openai_official".into(),
            account_id: "openai-primary".into(),
            display_name: "OpenAI Primary".into(),
            auth_state: "active".into(),
            expires_at_ms: Some(1_731_111_111_000),
            last_refresh_at_ms: Some(1_731_111_000_500),
            last_refresh_error: None,
        },
        "session-cookie".into(),
        serde_json::json!({
            "access_token": "access-token",
            "refresh_token": "refresh-token",
        })
        .to_string(),
    )
    .expect("save openai provider session");

    let summary = project_provider_inventory_summary(&state);
    let provider = summary
        .accounts
        .iter()
        .find(|provider| provider.account_id == "openai-primary")
        .expect("stored openai provider session should project into inventory");

    assert_eq!(provider.provider_id, "openai_official");
    assert!(provider.registered);
    assert!(provider.available);
    assert_eq!(provider.base_url, None);
    let models = summary
        .models
        .iter()
        .filter(|model| model.account_id == "openai-primary")
        .collect::<Vec<_>>();
    assert_eq!(
        models
            .iter()
            .map(|model| model.model_id.clone())
            .collect::<Vec<_>>(),
        vec!["gpt-5-mini".to_string()]
    );
    assert!(models.iter().all(|model| model.source == "session"));
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
        .accounts
        .iter()
        .find(|provider| provider.account_id == "unsupported-provider")
        .expect("unsupported provider should still be projected");

    assert!(!unsupported.registered);
    assert!(
        !unsupported.available,
        "unregistered providers should not project as available"
    );
    assert!(
        summary
            .models
            .iter()
            .filter(|model| model.account_id == "unsupported-provider")
            .collect::<Vec<_>>()
            .is_empty()
    );
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
