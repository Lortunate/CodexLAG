use serde::{Deserialize, Serialize};

use crate::error::{
    CodexLagError, ConfigErrorKind, CredentialErrorKind, QuotaErrorKind, UpstreamErrorKind,
};
use crate::providers::invocation::{InvocationFailure, InvocationFailureClass};

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
