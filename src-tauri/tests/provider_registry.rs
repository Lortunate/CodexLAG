use codexlag_lib::providers::registry::default_provider_registry;

#[test]
fn provider_registry_registers_openai_official_and_generic_provider_ids() {
    let registry = default_provider_registry();

    assert_eq!(registry.provider_ids(), vec!["generic_openai", "openai"]);

    let official = registry
        .adapter("openai")
        .expect("openai provider should be registered");
    assert_eq!(official.provider_id, "openai");

    let generic = registry
        .adapter("generic_openai")
        .expect("generic openai provider should be registered");
    assert_eq!(generic.provider_id, "generic_openai");
}
