use codexlag_lib::bootstrap::bootstrap_state_for_test;
use codexlag_lib::models::{ImportedOfficialAccount, OfficialSession};
use codexlag_lib::providers::inventory::project_provider_inventory_summary;
use codexlag_lib::secret_store::SecretKey;

#[tokio::test]
async fn provider_inventory_v12_projects_official_and_generic_accounts_into_one_summary() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    seed_account(
        &mut state,
        "official-primary",
        "OpenAI Primary",
        "openai_official",
        Some("session-secret"),
        r#"{"api_key":"openai-key"}"#,
    );
    seed_account(
        &mut state,
        "claude-direct",
        "Claude Direct",
        "claude_official",
        None,
        r#"{"api_key":"claude-key"}"#,
    );
    seed_account(
        &mut state,
        "gemini-direct",
        "Gemini Direct",
        "gemini_official",
        None,
        r#"{"api_key":"gemini-key"}"#,
    );
    seed_account(
        &mut state,
        "generic-alpha",
        "Generic Alpha",
        "generic_openai_compatible",
        None,
        r#"{"api_key":"generic-key","base_url":"https://gateway.example.test","manual_models":["gpt-4o-mini"]}"#,
    );

    let inventory = project_provider_inventory_summary(&state);

    assert!(
        inventory
            .accounts
            .iter()
            .all(|account| !account.provider_id.is_empty()),
        "inventory should preserve stable provider ids"
    );
    assert!(
        inventory
            .accounts
            .iter()
            .any(|account| account.provider_id == "claude_official" && account.available),
        "claude api-key account should project as available"
    );
    assert!(
        inventory
            .accounts
            .iter()
            .any(|account| account.provider_id == "gemini_official" && account.available),
        "gemini api-key account should project as available"
    );
    assert!(
        inventory
            .models
            .iter()
            .any(|model| model.provider_id == "claude_official" && model.model_id == "claude-3-7-sonnet")
    );
    assert!(
        inventory
            .models
            .iter()
            .any(|model| model.provider_id == "gemini_official" && model.model_id == "gemini-2.5-flash")
    );
}

#[tokio::test]
async fn provider_inventory_v12_reports_auth_profile_and_capability_support() {
    let mut state = bootstrap_state_for_test().await.expect("bootstrap state");

    seed_account(
        &mut state,
        "claude-direct",
        "Claude Direct",
        "claude_official",
        None,
        r#"{"api_key":"claude-key"}"#,
    );

    let inventory = project_provider_inventory_summary(&state);

    assert!(
        inventory.models.iter().all(|model| !model.provider_id.is_empty()),
        "capability rows should stay tied to stable provider ids"
    );

    let claude = inventory
        .models
        .iter()
        .find(|model| model.provider_id == "claude_official")
        .expect("claude model should be present");
    assert!(claude.supports_tools);
    assert!(claude.supports_streaming);
    assert!(claude.supports_reasoning);
}

fn seed_account(
    state: &mut codexlag_lib::state::AppState,
    account_id: &str,
    name: &str,
    provider: &str,
    session_secret: Option<&str>,
    token_secret: &str,
) {
    let session_ref = format!("credential://official/session/{account_id}");
    let token_ref = format!("credential://official/token/{account_id}");

    state
        .save_imported_official_account(ImportedOfficialAccount {
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
            session_credential_ref: session_ref.clone(),
            token_credential_ref: token_ref.clone(),
        })
        .expect("save account");

    if let Some(session_secret) = session_secret {
        state
            .store_secret(&SecretKey::new(session_ref), session_secret.to_string())
            .expect("store session secret");
    }

    state
        .store_secret(&SecretKey::new(token_ref), token_secret.to_string())
        .expect("store token secret");
}
