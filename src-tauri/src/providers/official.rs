use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::error::{
    CodexLagError, ConfigErrorKind, CredentialErrorKind, QuotaErrorKind, UpstreamErrorKind,
};
use crate::providers::invocation::{
    InvocationFailure, InvocationFailureClass, InvocationOutcome, InvocationSuccessMetadata,
    InvocationUsageDimensions,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(from = "String", into = "String")]
pub enum OfficialAuthMode {
    DeviceCode,
    ApiKey,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OfficialBalanceCapability {
    NonQueryable,
}

impl From<String> for OfficialAuthMode {
    fn from(value: String) -> Self {
        match value.as_str() {
            "device_code" => Self::DeviceCode,
            "api_key" => Self::ApiKey,
            _ => Self::Unknown(value),
        }
    }
}

impl From<OfficialAuthMode> for String {
    fn from(value: OfficialAuthMode) -> Self {
        match value {
            OfficialAuthMode::DeviceCode => "device_code".to_string(),
            OfficialAuthMode::ApiKey => "api_key".to_string(),
            OfficialAuthMode::Unknown(value) => value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: Option<String>,
    pub auth_mode: Option<OfficialAuthMode>,
    pub refresh_capability: Option<bool>,
    pub quota_capability: Option<bool>,
    pub last_verified_at_ms: Option<i64>,
    #[serde(default = "default_official_session_status")]
    pub status: String,
}

impl OfficialSession {
    pub fn balance_capability(&self) -> OfficialBalanceCapability {
        OfficialBalanceCapability::NonQueryable
    }
}

fn default_official_session_status() -> String {
    "active".to_string()
}

#[derive(Debug, Deserialize)]
struct OfficialCredentialSecret {
    api_key: String,
    #[serde(default)]
    base_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OfficialResponsePayload {
    model: Option<String>,
    #[serde(default)]
    usage: Option<OfficialUsagePayload>,
}

#[derive(Debug, Default, Deserialize)]
struct OfficialUsagePayload {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
    #[serde(default)]
    input_tokens_details: OfficialInputTokenDetails,
    #[serde(default)]
    output_tokens_details: OfficialOutputTokenDetails,
}

#[derive(Debug, Default, Deserialize)]
struct OfficialInputTokenDetails {
    #[serde(default)]
    cached_tokens: u32,
}

#[derive(Debug, Default, Deserialize)]
struct OfficialOutputTokenDetails {
    #[serde(default)]
    reasoning_tokens: u32,
}

pub async fn invoke_official_session(
    session: &OfficialSession,
    session_secret: &str,
    token_secret: &str,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> InvocationOutcome {
    if session_secret.trim().is_empty() {
        return Err(config_failure(request_id, attempt_id, endpoint_id, None));
    }

    let credentials = match parse_official_credentials(token_secret) {
        Ok(credentials) => credentials,
        Err(_) => return Err(config_failure(request_id, attempt_id, endpoint_id, None)),
    };
    let url = format!("{}/responses", credentials.base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = match client
        .post(url)
        .bearer_auth(credentials.api_key)
        .json(&json!({
            "model": "gpt-5-mini",
            "input": "codexlag request",
            "metadata": {
                "session_id": session.session_id,
            }
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            return Err(if error.is_timeout() {
                timeout_failure(request_id, attempt_id, endpoint_id)
            } else {
                http_failure(request_id, attempt_id, endpoint_id, None)
            });
        }
    };
    let status = response.status();
    let body = match response.bytes().await {
        Ok(body) => body,
        Err(_) => return Err(http_failure(request_id, attempt_id, endpoint_id, Some(status))),
    };
    if !status.is_success() {
        return Err(map_http_status_to_failure(
            request_id,
            attempt_id,
            endpoint_id,
            status,
        ));
    }

    let payload: OfficialResponsePayload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(_) => return Err(config_failure(request_id, attempt_id, endpoint_id, Some(status))),
    };
    let usage = payload.usage.unwrap_or_default();

    Ok(InvocationSuccessMetadata {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        model: payload.model.or_else(|| Some("gpt-5-mini".to_string())),
        upstream_status: status.as_u16(),
        usage_dimensions: Some(InvocationUsageDimensions {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cache_read_tokens: usage.input_tokens_details.cached_tokens,
            cache_write_tokens: 0,
            reasoning_tokens: usage.output_tokens_details.reasoning_tokens,
        }),
    })
}

struct ParsedOfficialCredentials {
    api_key: String,
    base_url: String,
}

fn parse_official_credentials(token_secret: &str) -> Result<ParsedOfficialCredentials, ()> {
    if let Ok(secret) = serde_json::from_str::<OfficialCredentialSecret>(token_secret) {
        let api_key = secret.api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(());
        }
        let base_url = secret
            .base_url
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string())
            .trim()
            .to_string();
        return Ok(ParsedOfficialCredentials { api_key, base_url });
    }

    let api_key = token_secret.trim().to_string();
    if api_key.is_empty() {
        return Err(());
    }
    Ok(ParsedOfficialCredentials {
        api_key,
        base_url: "https://api.openai.com/v1".to_string(),
    })
}

fn map_http_status_to_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: StatusCode,
) -> InvocationFailure {
    match status.as_u16() {
        401 | 403 => auth_failure(request_id, attempt_id, endpoint_id, status),
        429 => rate_limit_failure(request_id, attempt_id, endpoint_id, status),
        code if (500..=599).contains(&code) => {
            http_failure(request_id, attempt_id, endpoint_id, Some(status))
        }
        _ => config_failure(request_id, attempt_id, endpoint_id, Some(status)),
    }
}

fn auth_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: StatusCode,
) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Official,
        class: InvocationFailureClass::Auth,
        upstream_status: Some(status.as_u16()),
    }
}

fn rate_limit_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: StatusCode,
) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Official,
        class: InvocationFailureClass::Http429,
        upstream_status: Some(status.as_u16()),
    }
}

fn http_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: Option<StatusCode>,
) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Official,
        class: InvocationFailureClass::Http5xx,
        upstream_status: status.map(|value| value.as_u16()),
    }
}

fn timeout_failure(request_id: &str, attempt_id: &str, endpoint_id: &str) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Official,
        class: InvocationFailureClass::Timeout,
        upstream_status: None,
    }
}

fn config_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: Option<StatusCode>,
) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Official,
        class: InvocationFailureClass::Config,
        upstream_status: status.map(|value| value.as_u16()),
    }
}

pub(crate) fn map_official_invocation_failure(failure: &InvocationFailure) -> CodexLagError {
    let error = match failure.class {
        InvocationFailureClass::Auth => CodexLagError::credential(
            CredentialErrorKind::ProviderAuthFailed,
            "Official account credential was rejected by upstream.",
        ),
        InvocationFailureClass::Http429 => CodexLagError::quota(
            QuotaErrorKind::ProviderRateLimited,
            "Official provider rate limit exceeded. Try again shortly.",
        ),
        InvocationFailureClass::Http5xx => CodexLagError::upstream(
            UpstreamErrorKind::ProviderHttpFailure,
            "Official provider is temporarily unavailable.",
        ),
        InvocationFailureClass::Timeout => CodexLagError::upstream(
            UpstreamErrorKind::ProviderTimeout,
            "Official provider timed out while handling the request.",
        ),
        InvocationFailureClass::Config => CodexLagError::config(
            ConfigErrorKind::ProviderRejectedRequest,
            "Official account configuration is invalid for this request.",
        ),
    };

    error.with_internal_context(format!(
        "provider=official;endpoint_id={};upstream_status={:?}",
        failure.endpoint_id, failure.upstream_status
    ))
}
