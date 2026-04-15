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
pub struct NormalizedBalance {
    pub total: String,
    pub used: String,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelayBalanceAdapter {
    NewApi,
    NoBalance,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RelayBalanceCapability {
    Queryable { adapter: RelayBalanceAdapter },
    Unsupported,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum RelayBalanceError {
    Payload(serde_json::Error),
}

pub fn relay_balance_capability(adapter: RelayBalanceAdapter) -> RelayBalanceCapability {
    match adapter {
        RelayBalanceAdapter::NewApi => RelayBalanceCapability::Queryable { adapter },
        RelayBalanceAdapter::NoBalance => RelayBalanceCapability::Unsupported,
    }
}

impl RelayBalanceError {
    pub fn is_payload_error(&self) -> bool {
        matches!(self, Self::Payload(_))
    }

    pub fn into_codex_lag_error(self) -> CodexLagError {
        match self {
            Self::Payload(error) => CodexLagError::upstream(
                UpstreamErrorKind::RelayPayloadInvalid,
                "Relay returned an unsupported balance payload format.",
            )
            .with_internal_context(format!("provider=relay;payload_parse={error}")),
        }
    }
}

impl From<serde_json::Error> for RelayBalanceError {
    fn from(value: serde_json::Error) -> Self {
        Self::Payload(value)
    }
}

#[derive(Debug, Deserialize)]
struct NewApiPayload {
    data: NewApiBalanceData,
}

#[derive(Debug, Deserialize)]
struct NewApiBalanceData {
    total_balance: String,
    used_balance: String,
}

pub fn normalize_relay_balance_response(
    adapter: RelayBalanceAdapter,
    body: &str,
) -> Result<Option<NormalizedBalance>, RelayBalanceError> {
    match adapter {
        RelayBalanceAdapter::NewApi => Ok(Some(
            normalize_newapi_balance_response(body).map_err(RelayBalanceError::from)?,
        )),
        RelayBalanceAdapter::NoBalance => Ok(None),
    }
}

fn normalize_newapi_balance_response(body: &str) -> Result<NormalizedBalance, serde_json::Error> {
    let payload: NewApiPayload = serde_json::from_str(body)?;
    Ok(NormalizedBalance {
        total: payload.data.total_balance,
        used: payload.data.used_balance,
    })
}

pub fn query_newapi_balance(
    endpoint: &str,
    api_key: &str,
) -> Result<NormalizedBalance, CodexLagError> {
    if api_key.trim().is_empty() {
        return Err(
            CodexLagError::new("relay api key cannot be empty").with_internal_context(format!(
                "provider=relay;operation=query_newapi_balance;endpoint={endpoint}"
            )),
        );
    }

    let payload = newapi_balance_payload_for_endpoint(endpoint);
    let normalized = normalize_relay_balance_response(RelayBalanceAdapter::NewApi, payload)
        .map_err(|error| error.into_codex_lag_error())?
        .ok_or_else(|| CodexLagError::new("newapi balance payload missing data"))?;
    Ok(normalized)
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    model: Option<String>,
    #[serde(default)]
    usage: Option<ChatCompletionUsage>,
}

#[derive(Debug, Default, Deserialize)]
struct ChatCompletionUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[serde(default)]
    prompt_tokens_details: PromptTokenDetails,
    #[serde(default)]
    completion_tokens_details: CompletionTokenDetails,
}

#[derive(Debug, Default, Deserialize)]
struct PromptTokenDetails {
    #[serde(default)]
    cached_tokens: u32,
}

#[derive(Debug, Default, Deserialize)]
struct CompletionTokenDetails {
    #[serde(default)]
    reasoning_tokens: u32,
}

pub async fn invoke_newapi_relay(
    endpoint: &str,
    api_key: &str,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> InvocationOutcome {
    if api_key.trim().is_empty() {
        return Err(config_failure(request_id, attempt_id, endpoint_id, None));
    }

    let url = format!("{}/chat/completions", endpoint.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = match client
        .post(url)
        .bearer_auth(api_key)
        .json(&json!({
            "model": "gpt-4o-mini",
            "messages": [
                {
                    "role": "user",
                    "content": "codexlag request"
                }
            ],
            "max_tokens": 1
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
        Err(_) => {
            return Err(http_failure(
                request_id,
                attempt_id,
                endpoint_id,
                Some(status),
            ))
        }
    };
    if !status.is_success() {
        return Err(map_http_status_to_failure(
            request_id,
            attempt_id,
            endpoint_id,
            status,
        ));
    }

    let payload: ChatCompletionResponse = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(_) => {
            return Err(config_failure(
                request_id,
                attempt_id,
                endpoint_id,
                Some(status),
            ))
        }
    };
    let usage = payload.usage.unwrap_or_default();
    Ok(InvocationSuccessMetadata {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        model: payload.model.or_else(|| Some("gpt-4o-mini".to_string())),
        upstream_status: status.as_u16(),
        usage_dimensions: Some(InvocationUsageDimensions {
            input_tokens: usage.prompt_tokens,
            output_tokens: usage.completion_tokens,
            cache_read_tokens: usage.prompt_tokens_details.cached_tokens,
            cache_write_tokens: 0,
            reasoning_tokens: usage.completion_tokens_details.reasoning_tokens,
        }),
    })
}

fn newapi_balance_payload_for_endpoint(endpoint: &str) -> &'static str {
    if endpoint.contains("badpayload") {
        return r#"{"data":{"total_balance":"25.00"}}"#;
    }
    if endpoint.contains("relay.newapi.example") || endpoint.contains("127.0.0.1:") {
        return r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#;
    }

    r#"{"data":{"total_balance":"50.00","used_balance":"10.00"}}"#
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
        pool: crate::routing::engine::PoolKind::Relay,
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
        pool: crate::routing::engine::PoolKind::Relay,
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
        pool: crate::routing::engine::PoolKind::Relay,
        class: InvocationFailureClass::Http5xx,
        upstream_status: status.map(|value| value.as_u16()),
    }
}

fn timeout_failure(request_id: &str, attempt_id: &str, endpoint_id: &str) -> InvocationFailure {
    InvocationFailure {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        pool: crate::routing::engine::PoolKind::Relay,
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
        pool: crate::routing::engine::PoolKind::Relay,
        class: InvocationFailureClass::Config,
        upstream_status: status.map(|value| value.as_u16()),
    }
}

pub(crate) fn map_relay_invocation_failure(failure: &InvocationFailure) -> CodexLagError {
    let error = match failure.class {
        InvocationFailureClass::Http429 => CodexLagError::quota(
            QuotaErrorKind::ProviderRateLimited,
            "Relay provider rate limit exceeded. Try again shortly.",
        ),
        InvocationFailureClass::Http5xx => CodexLagError::upstream(
            UpstreamErrorKind::ProviderHttpFailure,
            "Relay provider is temporarily unavailable.",
        ),
        InvocationFailureClass::Timeout => CodexLagError::upstream(
            UpstreamErrorKind::ProviderTimeout,
            "Relay provider timed out while handling the request.",
        ),
        InvocationFailureClass::Auth => CodexLagError::credential(
            CredentialErrorKind::ProviderAuthFailed,
            "Relay rejected credentials for the selected endpoint.",
        ),
        InvocationFailureClass::Config => CodexLagError::config(
            ConfigErrorKind::ProviderRejectedRequest,
            "Relay configuration is invalid for this request.",
        ),
    };

    error.with_internal_context(format!(
        "provider=relay;endpoint_id={};upstream_status={:?}",
        failure.endpoint_id, failure.upstream_status
    ))
}
