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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformKey {
    pub id: String,
    pub name: String,
    pub allowed_mode: String,
    pub policy_id: String,
    pub enabled: bool,
}

impl PlatformKey {
    pub fn allowed_mode(&self) -> &str {
        self.allowed_mode.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingPolicy {
    pub id: String,
    pub name: String,
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
    Open,
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
