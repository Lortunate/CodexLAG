use codexlag_lib::providers::registry::default_provider_registry;

#[test]
fn provider_registry_registers_openai_official_and_generic_provider_ids() {
    let registry = default_provider_registry();

    let provider_ids = registry.provider_ids();
    assert!(
        provider_ids.contains(&"generic_openai_compatible"),
        "generic provider should remain registered"
    );
    assert!(
        provider_ids.contains(&"openai_official"),
        "openai official provider should remain registered"
    );

    let official = registry
        .adapter("openai_official")
        .expect("openai provider should be registered");
    assert_eq!(official.provider_id(), "openai_official");
    assert!(official.supports_browser_login());
    assert!(!official.supports_balance());

    let generic = registry
        .adapter("generic_openai_compatible")
        .expect("generic openai provider should be registered");
    assert_eq!(generic.provider_id(), "generic_openai_compatible");
    assert!(!generic.supports_browser_login());
    assert!(!generic.supports_balance());
}
