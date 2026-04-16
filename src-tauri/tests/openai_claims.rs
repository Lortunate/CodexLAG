use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use codexlag_lib::auth::openai_claims::{parse_openai_id_token_claims, OpenAiEntitlementSnapshot};

#[test]
fn parses_plan_and_subscription_window_from_openai_id_token_claims() {
    let token = test_openai_jwt(
        r#"{
            "email":"user@example.com",
            "https://api.openai.com/auth":{
                "chatgpt_account_id":"acc_123",
                "chatgpt_plan_type":"pro",
                "chatgpt_subscription_active_start":"2026-04-01T00:00:00Z",
                "chatgpt_subscription_active_until":"2026-05-01T00:00:00Z"
            }
        }"#,
    );

    let claims = parse_openai_id_token_claims(&token).expect("claims should parse");

    assert_eq!(claims.email.as_deref(), Some("user@example.com"));
    assert_eq!(claims.account_id.as_deref(), Some("acc_123"));
    assert_eq!(claims.plan_type.as_deref(), Some("pro"));
    assert_eq!(
        claims.subscription_active_until.as_deref(),
        Some("2026-05-01T00:00:00Z")
    );
}

#[test]
fn returns_empty_snapshot_when_openai_claim_block_is_missing() {
    let token = test_openai_jwt(r#"{"email":"user@example.com"}"#);

    let claims = parse_openai_id_token_claims(&token).expect("claims should parse");

    assert_eq!(
        claims,
        OpenAiEntitlementSnapshot {
            email: Some("user@example.com".into()),
            account_id: None,
            plan_type: None,
            subscription_active_start: None,
            subscription_active_until: None,
            claim_source: "id_token_claim".into(),
        }
    );
}

fn test_openai_jwt(payload_json: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(r#"{"alg":"none","typ":"JWT"}"#);
    let payload = URL_SAFE_NO_PAD.encode(payload_json);

    format!("{header}.{payload}.signature")
}
