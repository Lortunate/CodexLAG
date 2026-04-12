use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::providers::official::{OfficialBalanceCapability, OfficialSession};

#[derive(Debug, Clone, Serialize)]
pub struct AccountSummary {
    pub account_id: String,
    pub name: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum AccountBalanceAvailability {
    Queryable { total: String, used: String },
    NonQueryable { reason: String },
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountBalanceSnapshot {
    pub account_id: String,
    pub provider: String,
    pub refreshed_at: String,
    pub balance: AccountBalanceAvailability,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AccountCapabilityDetail {
    pub account_id: String,
    pub provider: String,
    pub refresh_capability: Option<bool>,
    pub balance_capability: OfficialBalanceCapability,
}

#[tauri::command]
pub fn list_accounts() -> Vec<AccountSummary> {
    vec![AccountSummary {
        account_id: "official-primary".into(),
        name: "Primary Publisher".into(),
        provider: "openai".into(),
    }]
}

#[tauri::command]
pub fn refresh_account_balance(account_id: String) -> Result<AccountBalanceSnapshot, String> {
    let summary = account_summary_by_id(account_id.as_str())?;
    let session = official_primary_session();
    let balance = match session.balance_capability() {
        OfficialBalanceCapability::NonQueryable => AccountBalanceAvailability::NonQueryable {
            reason: "official accounts do not expose a balance endpoint".into(),
        },
    };

    Ok(AccountBalanceSnapshot {
        account_id: summary.account_id,
        provider: summary.provider,
        refreshed_at: current_unix_timestamp_string(),
        balance,
    })
}

#[tauri::command]
pub fn get_account_capability_detail(
    account_id: String,
) -> Result<AccountCapabilityDetail, String> {
    let summary = account_summary_by_id(account_id.as_str())?;
    let session = official_primary_session();

    Ok(AccountCapabilityDetail {
        account_id: summary.account_id,
        provider: summary.provider,
        refresh_capability: session.refresh_capability,
        balance_capability: session.balance_capability(),
    })
}

fn official_primary_session() -> OfficialSession {
    OfficialSession {
        session_id: "official-session-1".into(),
        account_identity: Some("user@example.com".into()),
        auth_mode: None,
        refresh_capability: Some(true),
    }
}

fn account_summary_by_id(account_id: &str) -> Result<AccountSummary, String> {
    list_accounts()
        .into_iter()
        .find(|candidate| candidate.account_id == account_id)
        .ok_or_else(|| format!("unknown account id: {account_id}"))
}

fn current_unix_timestamp_string() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    timestamp.to_string()
}
