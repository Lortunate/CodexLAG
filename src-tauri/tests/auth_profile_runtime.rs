use codexlag_lib::auth::openai::{OpenAiAuthRuntime, OpenAiBrowserLoginRequest};
use codexlag_lib::bootstrap::bootstrap_state_for_test;
use codexlag_lib::models::ProviderAuthProfile;

#[tokio::test]
async fn auth_profile_runtime_describes_browser_oauth_and_static_api_key_profiles() {
    let state = bootstrap_state_for_test().await.expect("bootstrap state");

    let descriptors = state.list_provider_descriptors();

    let official = descriptors
        .iter()
        .find(|descriptor| descriptor.provider_id == "openai_official")
        .expect("openai official descriptor");
    assert_eq!(official.auth_profile, ProviderAuthProfile::BrowserOauthPkce);
    assert!(!official.supports_model_discovery);

    let generic = descriptors
        .iter()
        .find(|descriptor| descriptor.provider_id == "generic_openai_compatible")
        .expect("generic openai descriptor");
    assert_eq!(generic.auth_profile, ProviderAuthProfile::StaticApiKey);
    assert!(generic.supports_model_discovery);
    assert!(!generic.supports_capability_probe);
}

#[tokio::test]
async fn auth_profile_runtime_marks_pending_browser_login_as_browser_oauth_pkce() {
    let state = bootstrap_state_for_test().await.expect("bootstrap state");
    let mut runtime = OpenAiAuthRuntime::new(state);

    let pending = runtime
        .start_browser_login(OpenAiBrowserLoginRequest {
            account_id: "openai-primary".into(),
            display_name: "OpenAI Primary".into(),
            client_id: "codexlag-desktop".into(),
            issuer_url: "https://auth.openai.example".into(),
            authorization_endpoint: "https://auth.openai.example/oauth2/v1/authorize".into(),
            token_endpoint: "https://auth.openai.example/oauth2/v1/token".into(),
            scopes: vec!["openid".into(), "profile".into(), "offline_access".into()],
        })
        .expect("start browser login");

    assert_eq!(pending.auth_profile, ProviderAuthProfile::BrowserOauthPkce);
}
