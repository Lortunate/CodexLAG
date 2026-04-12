use serde::Serialize;

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
    pub balance_queryable: bool,
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
pub fn refresh_account_balance(account_id: String) -> Option<AccountBalanceSnapshot> {
    let summary = list_accounts()
        .into_iter()
        .find(|candidate| candidate.account_id == account_id)?;
    let session = official_primary_session();
    let balance = match session.balance_capability() {
        OfficialBalanceCapability::NonQueryable => AccountBalanceAvailability::NonQueryable {
            reason: "official accounts do not expose a balance endpoint".into(),
        },
    };

    Some(AccountBalanceSnapshot {
        account_id: summary.account_id,
        provider: summary.provider,
        refreshed_at: "2026-01-01T00:00:00Z".into(),
        balance,
    })
}

#[tauri::command]
pub fn get_account_capability_detail(account_id: String) -> Option<AccountCapabilityDetail> {
    let summary = list_accounts()
        .into_iter()
        .find(|candidate| candidate.account_id == account_id)?;
    let session = official_primary_session();
    let balance_queryable = matches!(
        session.balance_capability(),
        OfficialBalanceCapability::NonQueryable
    );

    Some(AccountCapabilityDetail {
        account_id: summary.account_id,
        provider: summary.provider,
        refresh_capability: session.refresh_capability,
        balance_queryable: !balance_queryable,
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
