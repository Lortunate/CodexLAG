use codexlag_lib::auth::list_provider_descriptors;
use codexlag_lib::models::ProviderAuthProfile;

#[test]
fn foundation_alignment_limits_official_desktop_auth_profiles() {
    let descriptors = list_provider_descriptors();
    let provider_ids = descriptors
        .iter()
        .map(|descriptor| descriptor.provider_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        provider_ids,
        vec![
            "claude_official",
            "gemini_official",
            "generic_openai_compatible",
            "openai_official",
        ],
        "provider descriptor set must stay explicit and bounded"
    );

    assert!(
        descriptors.iter().all(|descriptor| matches!(
            descriptor.auth_profile,
            ProviderAuthProfile::BrowserOauthPkce | ProviderAuthProfile::StaticApiKey
        )),
        "desktop provider auth profiles must stay inside the V1.2 baseline"
    );

    assert!(descriptors.iter().any(|descriptor| {
        descriptor.provider_id == "openai_official"
            && descriptor.auth_profile == ProviderAuthProfile::BrowserOauthPkce
    }));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.provider_id == "claude_official"
            && descriptor.auth_profile == ProviderAuthProfile::StaticApiKey
    }));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.provider_id == "gemini_official"
            && descriptor.auth_profile == ProviderAuthProfile::StaticApiKey
    }));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.provider_id == "generic_openai_compatible"
            && descriptor.auth_profile == ProviderAuthProfile::StaticApiKey
    }));
}
