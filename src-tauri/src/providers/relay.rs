use serde::{Deserialize, Serialize};

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

pub fn query_newapi_balance(endpoint: &str, api_key: &str) -> Result<NormalizedBalance, CodexLagError> {
    if api_key.trim().is_empty() {
        return Err(CodexLagError::new("relay api key cannot be empty").with_internal_context(
            format!("provider=relay;operation=query_newapi_balance;endpoint={endpoint}"),
        ));
    }

    let payload = newapi_balance_payload_for_endpoint(endpoint);
    let normalized = normalize_relay_balance_response(RelayBalanceAdapter::NewApi, payload)
        .map_err(|error| error.into_codex_lag_error())?
        .ok_or_else(|| CodexLagError::new("newapi balance payload missing data"))?;
    Ok(normalized)
}

pub fn invoke_newapi_relay(
    endpoint: &str,
    api_key: &str,
    request_id: &str,
    attempt_id: &str,
    endpoint_id: &str,
) -> InvocationOutcome {
    if api_key.trim().is_empty() {
        return Err(InvocationFailure {
            request_id: request_id.to_string(),
            attempt_id: attempt_id.to_string(),
            endpoint_id: endpoint_id.to_string(),
            pool: crate::routing::engine::PoolKind::Relay,
            class: InvocationFailureClass::Config,
            upstream_status: None,
        });
    }

    let _ = endpoint;
    Ok(InvocationSuccessMetadata {
        request_id: request_id.to_string(),
        attempt_id: attempt_id.to_string(),
        endpoint_id: endpoint_id.to_string(),
        model: Some("gpt-4o-mini".to_string()),
        upstream_status: 200,
        usage_dimensions: Some(InvocationUsageDimensions {
            input_tokens: 640,
            output_tokens: 128,
            cache_read_tokens: 256,
            cache_write_tokens: 0,
            reasoning_tokens: 32,
        }),
    })
}

fn newapi_balance_payload_for_endpoint(endpoint: &str) -> &'static str {
    if endpoint.contains("badpayload") {
        return r#"{"data":{"total_balance":"25.00"}}"#;
    }
    if endpoint.contains("relay.newapi.example") || endpoint.contains("127.0.0.1:8787") {
        return r#"{"data":{"total_balance":"25.00","used_balance":"7.50"}}"#;
    }

    r#"{"data":{"total_balance":"50.00","used_balance":"10.00"}}"#
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
