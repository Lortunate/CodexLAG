use std::time::Duration;

use serde::Deserialize;
use serde_json::json;

use crate::error::{
    CodexLagError, ConfigErrorKind, CredentialErrorKind, QuotaErrorKind, UpstreamErrorKind,
};
use crate::providers::invocation::{
    InvocationFailure, InvocationFailureClass, InvocationOutcome, InvocationSuccessMetadata,
    InvocationUsageDimensions,
};
use crate::providers::registry::ProviderAdapter;
use reqwest::StatusCode;

pub const GENERIC_OPENAI_PROVIDER_ID: &str = "generic_openai_compatible";
const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";

pub fn provider_adapter() -> ProviderAdapter {
    ProviderAdapter {
        provider_id: GENERIC_OPENAI_PROVIDER_ID,
        display_name: "OpenAI-Compatible",
        default_models: &[DEFAULT_MODEL],
        requires_session_secret: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericOpenAiConfig {
    pub api_key: String,
    pub base_url: String,
    pub manual_models: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GenericOpenAiCredentialSecret {
    api_key: String,
    #[serde(default)]
    base_url: Option<String>,
    #[serde(default)]
    manual_models: Vec<String>,
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

#[derive(Debug, Default, Deserialize)]
struct ModelListResponse {
    #[serde(default)]
    data: Vec<ModelDescriptor>,
}

#[derive(Debug, Deserialize)]
struct ModelDescriptor {
    id: String,
}

pub fn parse_generic_openai_config(secret: &str) -> Result<GenericOpenAiConfig, ()> {
    if let Ok(parsed) = serde_json::from_str::<GenericOpenAiCredentialSecret>(secret) {
        let api_key = parsed.api_key.trim().to_string();
        if api_key.is_empty() {
            return Err(());
        }
        return Ok(GenericOpenAiConfig {
            api_key,
            base_url: normalize_base_url(parsed.base_url.as_deref()),
            manual_models: normalize_manual_models(parsed.manual_models),
        });
    }

    let api_key = secret.trim().to_string();
    if api_key.is_empty() {
        return Err(());
    }

    Ok(GenericOpenAiConfig {
        api_key,
        base_url: DEFAULT_BASE_URL.to_string(),
        manual_models: Vec::new(),
    })
}

pub fn normalize_base_url(base_url: Option<&str>) -> String {
    let raw = base_url.unwrap_or(DEFAULT_BASE_URL).trim();
    let trimmed = raw.trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        trimmed.to_string()
    } else {
        format!("{trimmed}/v1")
    }
}

pub fn normalize_manual_models(models: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();
    for model in models {
        let model = model.trim();
        if model.is_empty() {
            continue;
        }
        if normalized.iter().any(|existing| existing == model) {
            continue;
        }
        normalized.push(model.to_string());
    }
    normalized
}

pub fn generic_openai_inventory_models(config: &GenericOpenAiConfig) -> Vec<String> {
    if !config.manual_models.is_empty() {
        return config.manual_models.clone();
    }

    if let Some(discovered) = discover_generic_openai_models(config) {
        return discovered;
    }

    provider_adapter()
        .default_models
        .iter()
        .map(|model| (*model).to_string())
        .collect()
}

fn discover_generic_openai_models(config: &GenericOpenAiConfig) -> Option<Vec<String>> {
    let response = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .ok()?
        .get(format!("{}/models", config.base_url))
        .bearer_auth(config.api_key.as_str())
        .send()
        .ok()?;
    if !response.status().is_success() {
        return None;
    }

    let payload = response.json::<ModelListResponse>().ok()?;
    let models = normalize_manual_models(payload.data.into_iter().map(|model| model.id).collect());
    (!models.is_empty()).then_some(models)
}

pub async fn invoke_generic_openai(
    secret: &str,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> InvocationOutcome {
    let config = match parse_generic_openai_config(secret) {
        Ok(config) => config,
        Err(_) => return Err(config_failure(request_id, attempt_id, endpoint_id, None)),
    };

    let model = config
        .manual_models
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());
    let url = format!("{}/chat/completions", config.base_url);
    let response = match reqwest::Client::new()
        .post(url)
        .bearer_auth(config.api_key)
        .json(&json!({
            "model": model,
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
        model: payload.model.or(Some(model)),
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

fn map_http_status_to_failure(
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
    status: StatusCode,
) -> InvocationFailure {
    match status.as_u16() {
        401 | 403 => auth_failure(request_id, attempt_id, endpoint_id, status),
        429 => rate_limit_failure(request_id, attempt_id, endpoint_id, status),
        500..=599 => http_failure(request_id, attempt_id, endpoint_id, Some(status)),
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
        upstream_status: status.map(|status| status.as_u16()),
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
        upstream_status: status.map(|status| status.as_u16()),
    }
}

pub fn map_generic_openai_invocation_failure(failure: &InvocationFailure) -> CodexLagError {
    let error = match failure.class {
        InvocationFailureClass::Timeout => CodexLagError::upstream(
            UpstreamErrorKind::ProviderTimeout,
            "The OpenAI-compatible provider timed out before returning a response.",
        ),
        InvocationFailureClass::Http429 => CodexLagError::quota(
            QuotaErrorKind::ProviderRateLimited,
            "The OpenAI-compatible provider rate limited the request.",
        ),
        InvocationFailureClass::Auth => CodexLagError::credential(
            CredentialErrorKind::ProviderAuthFailed,
            "The OpenAI-compatible provider rejected the stored API key.",
        ),
        InvocationFailureClass::Config => CodexLagError::config(
            ConfigErrorKind::ProviderRejectedRequest,
            "The OpenAI-compatible provider configuration is invalid.",
        ),
        InvocationFailureClass::Http5xx => CodexLagError::upstream(
            UpstreamErrorKind::ProviderHttpFailure,
            "The OpenAI-compatible provider is unavailable.",
        ),
    };

    error.with_internal_context(format!(
        "provider=generic_openai;endpoint_id={};upstream_status={:?}",
        failure.endpoint_id, failure.upstream_status
    ))
}
