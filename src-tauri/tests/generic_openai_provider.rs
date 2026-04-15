use axum::{routing::get, Json, Router};
use codexlag_lib::providers::generic_openai::{
    generic_openai_inventory_models, parse_generic_openai_config,
};
use serde_json::json;

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

#[tokio::test]
async fn generic_openai_provider_discovers_remote_models_when_manual_models_are_missing() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind generic provider test server");
    let address = listener
        .local_addr()
        .expect("generic provider test server should expose an address");
    let router = Router::new().route(
        "/v1/models",
        get(|| async {
            Json(json!({
                "data": [
                    { "id": " gpt-4.1-mini " },
                    { "id": "gpt-4o-mini" },
                    { "id": "gpt-4o-mini" }
                ]
            }))
        }),
    );

    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve generic provider test server");
    });

    let config = parse_generic_openai_config(
        format!(r#"{{"api_key":"test-key","base_url":"http://{address}"}}"#).as_str(),
    )
    .expect("generic provider config should parse");

    let models = tokio::task::spawn_blocking(move || generic_openai_inventory_models(&config))
        .await
        .expect("join blocking model discovery");

    assert_eq!(
        models,
        vec!["gpt-4.1-mini".to_string(), "gpt-4o-mini".to_string()]
    );
}
