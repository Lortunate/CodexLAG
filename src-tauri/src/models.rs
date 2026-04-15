use serde::{Deserialize, Serialize};

pub use crate::logging::usage::{
    UsageCost, UsageLedger, UsageLedgerQuery, UsageProvenance, UsageRecord, UsageRecordInput,
    UsageRequestDetail,
};
pub use crate::providers::capabilities::{FeatureCapability, FeatureCapabilityPatch};
pub use crate::providers::official::{
    OfficialAuthMode, OfficialBalanceCapability, OfficialSession,
};
pub use crate::providers::relay::{NormalizedBalance, RelayBalanceAdapter, RelayBalanceCapability};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
    pub created_at_ms: i64,
    pub last_used_at_ms: Option<i64>,
}

impl PlatformKey {
    pub fn allowed_mode(&self) -> &str {
        self.allowed_mode.as_str()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub selection_order: Vec<String>,
    #[serde(default = "default_cross_pool_fallback")]
    pub cross_pool_fallback: bool,
    #[serde(default)]
    pub retry_budget: u32,
    #[serde(default)]
    pub failure_rules: FailureRules,
    #[serde(default)]
    pub recovery_rules: RecoveryRules,
}

fn default_cross_pool_fallback() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderEndpointKind {
    OfficialAccount,
    RelayEndpoint,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderEndpoint {
    pub id: String,
    pub name: String,
    pub kind: ProviderEndpointKind,
    pub enabled: bool,
    pub priority: i64,
    pub pool_tags: Vec<String>,
    pub health_status: EndpointHealthState,
    pub last_health_check_at_ms: Option<i64>,
    pub supports_balance_query: bool,
    pub last_balance_snapshot_at_ms: Option<i64>,
    pub pricing_profile_id: Option<String>,
    pub credential_ref_id: Option<String>,
    pub feature_capabilities: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialKind {
    PlatformKeySecret,
    OfficialSession,
    RelayApiKey,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialRef {
    pub id: String,
    pub target_name: String,
    pub version: i64,
    pub credential_kind: CredentialKind,
    pub last_verified_at_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportedOfficialAccount {
    pub account_id: String,
    pub name: String,
    pub provider: String,
    pub session: OfficialSession,
    pub session_credential_ref: String,
    pub token_credential_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSessionSummary {
    pub provider_id: String,
    pub account_id: String,
    pub display_name: String,
    pub auth_state: String,
    pub expires_at_ms: Option<i64>,
    pub last_refresh_at_ms: Option<i64>,
    pub last_refresh_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManagedRelay {
    pub relay_id: String,
    pub name: String,
    pub endpoint: String,
    pub adapter: RelayBalanceAdapter,
    pub api_key_credential_ref: String,
}

pub fn relay_api_key_credential_ref(relay_id: &str) -> String {
    format!("credential://relay/api-key/{relay_id}")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricingProfile {
    pub id: String,
    pub model: String,
    pub input_price_per_1k_micros: i64,
    pub output_price_per_1k_micros: i64,
    pub cache_read_price_per_1k_micros: i64,
    pub currency: String,
    pub effective_from_ms: i64,
    pub effective_to_ms: Option<i64>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestLog {
    pub request_id: String,
    pub platform_key_id: String,
    pub request_type: String,
    pub model: String,
    pub selected_endpoint_id: Option<String>,
    pub attempt_count: u32,
    pub final_status: String,
    pub http_status: Option<i64>,
    pub started_at_ms: i64,
    pub finished_at_ms: Option<i64>,
    pub latency_ms: Option<i64>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub requested_context_window: Option<i64>,
    pub requested_context_compression: Option<String>,
    pub effective_context_window: Option<i64>,
    pub effective_context_compression: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestAttemptLog {
    pub attempt_id: String,
    pub request_id: String,
    pub attempt_index: u32,
    pub endpoint_id: String,
    pub pool_type: String,
    pub trigger_reason: String,
    pub upstream_status: Option<i64>,
    pub timeout_ms: Option<i64>,
    pub latency_ms: Option<i64>,
    pub token_usage_snapshot: Option<String>,
    pub estimated_cost_snapshot: Option<String>,
    pub balance_snapshot_id: Option<String>,
    pub feature_resolution_snapshot: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointFailure {
    Timeout,
    HttpStatus(u16),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureClass {
    Timeout,
    RateLimited,
    ServerError,
    Ignored,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndpointHealthState {
    Healthy,
    Degraded,
    OpenCircuit,
    HalfOpen,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointHealth {
    pub state: EndpointHealthState,
    pub open_until_ms: Option<u64>,
    pub consecutive_timeouts: u32,
    pub consecutive_server_errors: u32,
    pub last_failure: Option<FailureClass>,
}

impl Default for EndpointHealth {
    fn default() -> Self {
        Self {
            state: EndpointHealthState::Healthy,
            open_until_ms: None,
            consecutive_timeouts: 0,
            consecutive_server_errors: 0,
            last_failure: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailureRules {
    pub cooldown_ms: u64,
    pub timeout_open_after: u32,
    pub server_error_open_after: u32,
}

impl Default for FailureRules {
    fn default() -> Self {
        Self {
            cooldown_ms: 30_000,
            timeout_open_after: 3,
            server_error_open_after: 3,
        }
    }
}

impl FailureRules {
    pub fn classify_failure(&self, failure: &EndpointFailure) -> FailureClass {
        match failure {
            EndpointFailure::Timeout => FailureClass::Timeout,
            EndpointFailure::HttpStatus(429) => FailureClass::RateLimited,
            EndpointFailure::HttpStatus(status) if (500..=599).contains(status) => {
                FailureClass::ServerError
            }
            EndpointFailure::HttpStatus(_) => FailureClass::Ignored,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecoveryRules {
    pub half_open_after_ms: u64,
    pub success_close_after: u32,
}

impl Default for RecoveryRules {
    fn default() -> Self {
        Self {
            half_open_after_ms: 15_000,
            success_close_after: 1,
        }
    }
}
