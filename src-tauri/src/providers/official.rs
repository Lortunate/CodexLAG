use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OfficialAuthMode {
    DeviceCode,
    ApiKey,
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: Option<String>,
    pub auth_mode: Option<OfficialAuthMode>,
    pub refresh_capability: Option<bool>,
}
