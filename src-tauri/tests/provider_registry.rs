use codexlag_lib::providers::registry::default_provider_registry;

#[test]
fn provider_registry_registers_openai_official_and_generic_provider_ids() {
    let registry = default_provider_registry();

    assert_eq!(
        registry.provider_ids(),
        vec!["generic_openai_compatible", "openai_official"]
    );

    let official = registry
        .adapter("openai_official")
        .expect("openai provider should be registered");
    assert_eq!(official.provider_id, "openai_official");

    let generic = registry
        .adapter("generic_openai_compatible")
        .expect("generic openai provider should be registered");
    assert_eq!(generic.provider_id, "generic_openai_compatible");
}
