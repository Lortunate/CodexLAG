use codexlag_lib::providers::registry::default_provider_registry;

#[test]
fn provider_registry_v12_registers_bounded_official_provider_ids() {
    let registry = default_provider_registry();

    assert!(registry.adapter("openai_official").is_some());
    assert!(registry.adapter("claude_official").is_some());
    assert!(registry.adapter("gemini_official").is_some());
}
