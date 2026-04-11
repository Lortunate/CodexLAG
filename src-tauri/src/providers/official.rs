use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfficialSession {
    pub session_id: String,
    pub account_identity: String,
    pub auth_mode: String,
    pub refresh_capability: bool,
}
