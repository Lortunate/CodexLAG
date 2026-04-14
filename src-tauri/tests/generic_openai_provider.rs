use codexlag_lib::providers::generic_openai::{
    generic_openai_inventory_models, parse_generic_openai_config,
};

#[test]
fn generic_openai_provider_normalizes_base_url_and_manual_models() {
    let config = parse_generic_openai_config(
        r#"{
            "api_key": "test-key",
            "base_url": "https://gateway.example.test/",
            "manual_models": [" gpt-4o-mini ", "", "gpt-4.1-mini", "gpt-4o-mini"]
        }"#,
    )
    .expect("generic provider config should parse");

    assert_eq!(config.base_url, "https://gateway.example.test/v1");
    assert_eq!(
        config.manual_models,
        vec!["gpt-4o-mini".to_string(), "gpt-4.1-mini".to_string()]
    );
    assert_eq!(
        generic_openai_inventory_models(&config),
        vec!["gpt-4o-mini".to_string(), "gpt-4.1-mini".to_string()]
    );
}
