use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;

use crate::error::{CodexLagError, ConfigErrorKind, Result};
use crate::models::ImportedOfficialAccount;
use crate::providers::official::{OfficialAuthMode, OfficialBalanceCapability, OfficialSession};
use crate::state::{AppState, RuntimeState};

const OFFICIAL_PRIMARY_ACCOUNT_ID: &str = "official-primary";
const RESERVED_BUILTIN_ACCOUNT_IDS: &[&str] = &[OFFICIAL_PRIMARY_ACCOUNT_ID];

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

#[derive(Debug, Clone, Deserialize)]
pub struct OfficialAccountImportInput {
    pub account_id: String,
    pub name: String,
    pub provider: String,
    pub session_credential_ref: String,
    pub token_credential_ref: String,
    pub account_identity: Option<String>,
    pub auth_mode: Option<String>,
}

pub(crate) fn list_accounts_from_state(state: &AppState) -> Vec<AccountSummary> {
    let mut accounts = vec![AccountSummary {
        account_id: OFFICIAL_PRIMARY_ACCOUNT_ID.into(),
        name: "Primary Publisher".into(),
        provider: "openai".into(),
    }];

    accounts.extend(
        state
            .iter_imported_official_accounts()
            .map(|account| AccountSummary {
                account_id: account.account_id.clone(),
                name: account.name.clone(),
                provider: account.provider.clone(),
            }),
    );
    accounts.sort_by(|left, right| left.account_id.cmp(&right.account_id));
    accounts
}

pub fn list_accounts_from_runtime(runtime: &RuntimeState) -> Vec<AccountSummary> {
    list_accounts_from_state(&runtime.app_state())
}

#[tauri::command]
pub fn list_accounts(state: State<'_, RuntimeState>) -> Vec<AccountSummary> {
    list_accounts_from_runtime(&state)
}

pub fn refresh_account_balance_from_runtime(
    runtime: &RuntimeState,
    account_id: String,
) -> Result<AccountBalanceSnapshot> {
    let state = runtime.app_state();
    let summary = account_summary_by_id(&state, account_id.as_str()).map_err(|error| {
        with_command_context(
            error,
            format!("command=refresh_account_balance;account_id={account_id}"),
        )
    })?;
    let session = official_session_for(&state, account_id.as_str()).map_err(|error| {
        with_command_context(
            error,
            format!("command=refresh_account_balance;account_id={account_id}"),
        )
    })?;
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
pub fn refresh_account_balance(
    account_id: String,
    state: State<'_, RuntimeState>,
) -> Result<AccountBalanceSnapshot> {
    refresh_account_balance_from_runtime(&state, account_id)
}

pub fn get_account_capability_detail_from_runtime(
    runtime: &RuntimeState,
    account_id: String,
) -> Result<AccountCapabilityDetail> {
    let state = runtime.app_state();
    let summary = account_summary_by_id(&state, account_id.as_str()).map_err(|error| {
        with_command_context(
            error,
            format!("command=get_account_capability_detail;account_id={account_id}"),
        )
    })?;
    let session = official_session_for(&state, account_id.as_str()).map_err(|error| {
        with_command_context(
            error,
            format!("command=get_account_capability_detail;account_id={account_id}"),
        )
    })?;

    Ok(AccountCapabilityDetail {
        account_id: summary.account_id,
        provider: summary.provider,
        refresh_capability: session.refresh_capability,
        balance_capability: session.balance_capability(),
    })
}

#[tauri::command]
pub fn get_account_capability_detail(
    account_id: String,
    state: State<'_, RuntimeState>,
) -> Result<AccountCapabilityDetail> {
    get_account_capability_detail_from_runtime(&state, account_id)
}

pub fn import_official_account_login_from_runtime(
    runtime: &RuntimeState,
    input: OfficialAccountImportInput,
) -> Result<AccountSummary> {
    let account_id = validate_identifier(input.account_id, "account_id")?;
    validate_not_reserved_account_id(account_id.as_str())?;
    let name = validate_non_empty(input.name, "name")?;
    let provider = validate_non_empty(input.provider, "provider")?;
    if provider != "openai" {
        return Err(invalid_payload_error(
            "provider must be 'openai'",
            "command=account_import_validation;field=provider;value=invalid",
        ));
    }

    validate_credential_ref(
        input.session_credential_ref.as_str(),
        "credential://official/session/",
        "session credential ref",
    )?;
    validate_credential_ref(
        input.token_credential_ref.as_str(),
        "credential://official/token/",
        "token credential ref",
    )?;

    let account = ImportedOfficialAccount {
        account_id: account_id.clone(),
        name: name.clone(),
        provider: provider.clone(),
        session: OfficialSession {
            session_id: format!("session:{account_id}"),
            account_identity: input.account_identity.map(|value| value.trim().to_string()),
            auth_mode: parse_auth_mode(input.auth_mode.as_deref()),
            refresh_capability: Some(true),
        },
        session_credential_ref: input.session_credential_ref.trim().to_string(),
        token_credential_ref: input.token_credential_ref.trim().to_string(),
    };

    runtime
        .app_state_mut()
        .save_imported_official_account(account)
        .map_err(|error| {
            CodexLagError::new("Failed to persist imported official account login.")
                .with_internal_context(format!(
                    "command=import_official_account_login;operation=save_imported_official_account;account_id={account_id};cause={error}"
                ))
        })?;

    Ok(AccountSummary {
        account_id,
        name,
        provider,
    })
}

#[tauri::command]
pub fn import_official_account_login(
    input: OfficialAccountImportInput,
    state: State<'_, RuntimeState>,
) -> Result<AccountSummary> {
    import_official_account_login_from_runtime(&state, input)
}

fn official_primary_session() -> OfficialSession {
    OfficialSession {
        session_id: "official-session-1".into(),
        account_identity: Some("user@example.com".into()),
        auth_mode: None,
        refresh_capability: Some(true),
    }
}

fn official_session_for(state: &AppState, account_id: &str) -> Result<OfficialSession> {
    if account_id == OFFICIAL_PRIMARY_ACCOUNT_ID {
        return Ok(official_primary_session());
    }

    let entry = state.imported_official_account(account_id).ok_or_else(|| {
        invalid_payload_error(
            "Unknown account id.",
            format!("command=account_lookup;field=account_id;value={account_id}"),
        )
    })?;
    Ok(entry.session.clone())
}

fn account_summary_by_id(state: &AppState, account_id: &str) -> Result<AccountSummary> {
    list_accounts_from_state(state)
        .into_iter()
        .find(|candidate| candidate.account_id == account_id)
        .ok_or_else(|| {
            invalid_payload_error(
                "Unknown account id.",
                format!("command=account_lookup;field=account_id;value={account_id}"),
            )
        })
}

fn validate_identifier(raw: String, field_name: &str) -> Result<String> {
    let value = validate_non_empty(raw, field_name)?;
    if value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || character == '-' || character == '_')
    {
        Ok(value)
    } else {
        Err(invalid_payload_error(
            format!("{field_name} must use only ascii letters, numbers, '-' or '_'"),
            format!("command=account_import_validation;field={field_name};value={value}"),
        ))
    }
}

fn validate_not_reserved_account_id(account_id: &str) -> Result<()> {
    if RESERVED_BUILTIN_ACCOUNT_IDS
        .iter()
        .any(|reserved_id| account_id == *reserved_id)
    {
        Err(invalid_payload_error(
            format!("account_id is reserved and cannot be imported: {account_id}"),
            format!(
                "command=account_import_validation;field=account_id;value={account_id};reason=reserved"
            ),
        ))
    } else {
        Ok(())
    }
}

fn validate_non_empty(raw: String, field_name: &str) -> Result<String> {
    let value = raw.trim().to_string();
    if value.is_empty() {
        Err(invalid_payload_error(
            format!("{field_name} must not be empty"),
            format!("command=account_import_validation;field={field_name};value=empty"),
        ))
    } else {
        Ok(value)
    }
}

fn validate_credential_ref(value: &str, prefix: &str, label: &str) -> Result<()> {
    let normalized = value.trim();
    if normalized.starts_with(prefix) {
        Ok(())
    } else {
        Err(invalid_payload_error(
            format!("{label} must start with '{prefix}'"),
            format!("command=account_import_validation;field={label};value={normalized}"),
        ))
    }
}

fn parse_auth_mode(value: Option<&str>) -> Option<OfficialAuthMode> {
    let Some(raw) = value else {
        return None;
    };
    let normalized = raw.trim();
    if normalized.is_empty() {
        return None;
    }
    Some(OfficialAuthMode::from(normalized.to_string()))
}

fn current_unix_timestamp_string() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    timestamp.to_string()
}

fn invalid_payload_error(message: impl Into<String>, context: impl Into<String>) -> CodexLagError {
    CodexLagError::config(ConfigErrorKind::InvalidPayload, message).with_internal_context(context)
}

fn with_command_context(error: CodexLagError, context: String) -> CodexLagError {
    let merged_context = match error.internal_context() {
        Some(existing) => format!("{context};{existing}"),
        None => context,
    };
    error.with_internal_context(merged_context)
}
