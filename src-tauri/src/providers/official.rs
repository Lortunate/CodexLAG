use serde::{Deserialize, Serialize};

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
