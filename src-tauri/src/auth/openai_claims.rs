use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::Deserialize;

use crate::error::{CodexLagError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiEntitlementSnapshot {
    pub email: Option<String>,
    pub account_id: Option<String>,
    pub plan_type: Option<String>,
    pub subscription_active_start: Option<String>,
    pub subscription_active_until: Option<String>,
    pub claim_source: String,
}

#[derive(Debug, Deserialize)]
struct JwtClaims {
    #[serde(default)]
    email: Option<String>,
    #[serde(rename = "https://api.openai.com/auth", default)]
    auth: Option<OpenAiAuthClaims>,
}

#[derive(Debug, Deserialize)]
struct OpenAiAuthClaims {
    #[serde(default)]
    chatgpt_account_id: Option<String>,
    #[serde(default)]
    chatgpt_plan_type: Option<String>,
    #[serde(default)]
    chatgpt_subscription_active_start: Option<String>,
    #[serde(default)]
    chatgpt_subscription_active_until: Option<String>,
}

pub fn parse_openai_id_token_claims(id_token: &str) -> Result<OpenAiEntitlementSnapshot> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| CodexLagError::new("openai id_token missing payload segment"))?;
    let decoded = URL_SAFE_NO_PAD.decode(payload).map_err(|error| {
        CodexLagError::new(format!("failed to decode openai id_token: {error}"))
    })?;
    let claims: JwtClaims = serde_json::from_slice(&decoded).map_err(|error| {
        CodexLagError::new(format!("failed to parse openai id_token claims: {error}"))
    })?;
    let auth = claims.auth;

    Ok(OpenAiEntitlementSnapshot {
        email: claims.email,
        account_id: auth
            .as_ref()
            .and_then(|value| value.chatgpt_account_id.clone()),
        plan_type: auth
            .as_ref()
            .and_then(|value| value.chatgpt_plan_type.clone()),
        subscription_active_start: auth
            .as_ref()
            .and_then(|value| value.chatgpt_subscription_active_start.clone()),
        subscription_active_until: auth
            .as_ref()
            .and_then(|value| value.chatgpt_subscription_active_until.clone()),
        claim_source: "id_token_claim".into(),
    })
}
