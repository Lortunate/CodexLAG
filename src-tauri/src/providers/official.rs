use serde::{Deserialize, Serialize};

use crate::error::{
    CodexLagError, ConfigErrorKind, CredentialErrorKind, QuotaErrorKind, UpstreamErrorKind,
};
use crate::providers::invocation::{InvocationFailure, InvocationFailureClass};

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
}

impl OfficialSession {
    pub fn balance_capability(&self) -> OfficialBalanceCapability {
        OfficialBalanceCapability::NonQueryable
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
