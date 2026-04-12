use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCategory {
    CredentialError,
    QuotaError,
    RoutingError,
    UpstreamError,
    ConfigError,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialErrorKind {
    MissingCredential,
    ExpiredCredential,
    ProviderAuthFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaErrorKind {
    Exhausted,
    ProviderRateLimited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingErrorKind {
    InvalidMode,
    NoAvailableEndpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamErrorKind {
    ProviderTimeout,
    ProviderHttpFailure,
    RelayPayloadInvalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigErrorKind {
    PolicyMissing,
    ProviderRejectedRequest,
    UnsupportedMode,
    InvalidPayload,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexLagErrorKind {
    Credential(CredentialErrorKind),
    Quota(QuotaErrorKind),
    Routing(RoutingErrorKind),
    Upstream(UpstreamErrorKind),
    Config(ConfigErrorKind),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodexLagErrorPayload {
    pub code: String,
    pub category: ErrorCategory,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_context: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexLagError {
    kind: CodexLagErrorKind,
    user_message: String,
    internal_context: Option<String>,
}

impl CodexLagError {
    pub fn new(message: impl Into<String>) -> Self {
        Self::config(ConfigErrorKind::Unknown, message)
    }

    pub fn credential(kind: CredentialErrorKind, message: impl Into<String>) -> Self {
        Self::from_kind(CodexLagErrorKind::Credential(kind), message)
    }

    pub fn quota(kind: QuotaErrorKind, message: impl Into<String>) -> Self {
        Self::from_kind(CodexLagErrorKind::Quota(kind), message)
    }

    pub fn routing(kind: RoutingErrorKind, message: impl Into<String>) -> Self {
        Self::from_kind(CodexLagErrorKind::Routing(kind), message)
    }

    pub fn upstream(kind: UpstreamErrorKind, message: impl Into<String>) -> Self {
        Self::from_kind(CodexLagErrorKind::Upstream(kind), message)
    }

    pub fn config(kind: ConfigErrorKind, message: impl Into<String>) -> Self {
        Self::from_kind(CodexLagErrorKind::Config(kind), message)
    }

    pub fn from_kind(kind: CodexLagErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            user_message: message.into(),
            internal_context: None,
        }
    }

    pub fn with_internal_context(mut self, context: impl Into<String>) -> Self {
        self.internal_context = Some(context.into());
        self
    }

    pub fn code(&self) -> &'static str {
        match self.kind {
            CodexLagErrorKind::Credential(CredentialErrorKind::MissingCredential) => {
                "credential.missing"
            }
            CodexLagErrorKind::Credential(CredentialErrorKind::ExpiredCredential) => {
                "credential.expired"
            }
            CodexLagErrorKind::Credential(CredentialErrorKind::ProviderAuthFailed) => {
                "credential.provider_auth_failed"
            }
            CodexLagErrorKind::Quota(QuotaErrorKind::Exhausted) => "quota.exhausted",
            CodexLagErrorKind::Quota(QuotaErrorKind::ProviderRateLimited) => {
                "quota.provider_rate_limited"
            }
            CodexLagErrorKind::Routing(RoutingErrorKind::InvalidMode) => "routing.invalid_mode",
            CodexLagErrorKind::Routing(RoutingErrorKind::NoAvailableEndpoint) => {
                "routing.no_available_endpoint"
            }
            CodexLagErrorKind::Upstream(UpstreamErrorKind::ProviderTimeout) => {
                "upstream.provider_timeout"
            }
            CodexLagErrorKind::Upstream(UpstreamErrorKind::ProviderHttpFailure) => {
                "upstream.provider_http_failure"
            }
            CodexLagErrorKind::Upstream(UpstreamErrorKind::RelayPayloadInvalid) => {
                "upstream.relay_payload_invalid"
            }
            CodexLagErrorKind::Config(ConfigErrorKind::PolicyMissing) => "config.policy_missing",
            CodexLagErrorKind::Config(ConfigErrorKind::ProviderRejectedRequest) => {
                "config.provider_rejected_request"
            }
            CodexLagErrorKind::Config(ConfigErrorKind::UnsupportedMode) => {
                "config.unsupported_mode"
            }
            CodexLagErrorKind::Config(ConfigErrorKind::InvalidPayload) => "config.invalid_payload",
            CodexLagErrorKind::Config(ConfigErrorKind::Unknown) => "config.unknown",
        }
    }

    pub fn category(&self) -> ErrorCategory {
        match self.kind {
            CodexLagErrorKind::Credential(_) => ErrorCategory::CredentialError,
            CodexLagErrorKind::Quota(_) => ErrorCategory::QuotaError,
            CodexLagErrorKind::Routing(_) => ErrorCategory::RoutingError,
            CodexLagErrorKind::Upstream(_) => ErrorCategory::UpstreamError,
            CodexLagErrorKind::Config(_) => ErrorCategory::ConfigError,
        }
    }

    pub fn message(&self) -> &str {
        self.user_message.as_str()
    }

    pub fn internal_context(&self) -> Option<&str> {
        self.internal_context.as_deref()
    }

    pub fn to_payload(&self) -> CodexLagErrorPayload {
        CodexLagErrorPayload {
            code: self.code().to_string(),
            category: self.category(),
            message: self.user_message.clone(),
            internal_context: self.internal_context.clone(),
        }
    }

    pub fn into_payload(self) -> CodexLagErrorPayload {
        CodexLagErrorPayload {
            code: self.code().to_string(),
            category: self.category(),
            message: self.user_message,
            internal_context: self.internal_context,
        }
    }
}

impl Serialize for CodexLagError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_payload().serialize(serializer)
    }
}

impl fmt::Display for CodexLagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CodexLAG error: {}", self.user_message)
    }
}

impl std::error::Error for CodexLagError {}

impl From<String> for CodexLagError {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for CodexLagError {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

pub type Result<T> = std::result::Result<T, CodexLagError>;
