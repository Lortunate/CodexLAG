use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::auth::openai::{PendingOpenAiLoopbackAuthSession, ReqwestOpenAiSessionRefresher};
use crate::error::{CodexLagError, ConfigErrorKind, Result};
use crate::models::{ImportedOfficialAccount, ProviderSessionSummary};
use crate::providers::official::{OfficialAuthMode, OfficialBalanceCapability, OfficialSession};
use crate::state::{AppState, RuntimeState};

const OFFICIAL_PRIMARY_ACCOUNT_ID: &str = "official-primary";
const OPENAI_AUTH_ACCOUNT_ID: &str = "openai-primary";
const OPENAI_AUTH_DISPLAY_NAME: &str = "OpenAI Primary";
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
    pub status: String,
    pub account_identity: Option<String>,
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
    let mut accounts = state
        .iter_imported_official_accounts()
        .map(|account| AccountSummary {
            account_id: account.account_id.clone(),
            name: account.name.clone(),
            provider: account.provider.clone(),
        })
        .collect::<Vec<_>>();
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

pub fn list_provider_sessions_from_runtime(
    runtime: &RuntimeState,
) -> Result<Vec<ProviderSessionSummary>> {
    runtime.list_provider_sessions()
}

#[tauri::command]
pub fn list_provider_sessions(state: State<'_, RuntimeState>) -> Result<Vec<ProviderSessionSummary>> {
    list_provider_sessions_from_runtime(&state)
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
    .inspect(|snapshot| {
        runtime.record_balance_refresh_summary(format!(
            "account:{} @ {} ({})",
            snapshot.account_id,
            snapshot.refreshed_at,
            account_balance_status(snapshot)
        ));
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
    let session = runtime
        .loopback_gateway()
        .state()
        .official_session_for_candidate(account_id.as_str())
        .or_else(|_| official_session_for(&state, account_id.as_str()))
        .map_err(|error| {
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
        status: session.status,
        account_identity: session.account_identity,
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
    {
        let state = runtime.app_state();
        validate_not_conflicting_with_relay_id(&state, account_id.as_str())?;
    }

    let account = ImportedOfficialAccount {
        account_id: account_id.clone(),
        name: name.clone(),
        provider: provider.clone(),
        session: OfficialSession {
            session_id: format!("session:{account_id}"),
            account_identity: input.account_identity.map(|value| value.trim().to_string()),
            auth_mode: parse_auth_mode(input.auth_mode.as_deref()),
            refresh_capability: Some(true),
            quota_capability: Some(false),
            last_verified_at_ms: None,
            status: "active".to_string(),
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
    runtime.on_inventory_changed().map_err(|error| {
        with_command_context(
            error,
            format!(
                "command=import_official_account_login;operation=on_inventory_changed;account_id={account_id}"
            ),
        )
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

pub fn start_openai_browser_login_from_runtime(
    runtime: &RuntimeState,
    app: &AppHandle,
) -> Result<PendingOpenAiLoopbackAuthSession> {
    let pending = runtime
        .openai_auth_mut()
        .start_default_browser_login(
            OPENAI_AUTH_ACCOUNT_ID.to_string(),
            OPENAI_AUTH_DISPLAY_NAME.to_string(),
        )?;

    app.opener()
        .open_url(pending.authorization_url.as_str(), None::<&str>)
        .map_err(|error| {
            CodexLagError::new(format!("Failed to open OpenAI browser login URL: {error}"))
                .with_internal_context("command=start_openai_browser_login;operation=open_url")
        })?;

    Ok(pending)
}

#[tauri::command]
pub fn start_openai_browser_login(
    app: AppHandle,
    state: State<'_, RuntimeState>,
) -> Result<PendingOpenAiLoopbackAuthSession> {
    start_openai_browser_login_from_runtime(&state, &app)
}

pub fn refresh_openai_session_from_runtime(
    runtime: &RuntimeState,
    account_id: String,
) -> Result<ProviderSessionSummary> {
    let refresher = ReqwestOpenAiSessionRefresher::new();
    let session = runtime
        .openai_auth_mut()
        .refresh_session(account_id.as_str(), &refresher)?;
    Ok(session.summary)
}

#[tauri::command]
pub fn refresh_openai_session(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<ProviderSessionSummary> {
    refresh_openai_session_from_runtime(&state, account_id)
}

pub fn logout_openai_session_from_runtime(
    runtime: &RuntimeState,
    account_id: String,
) -> Result<bool> {
    runtime.openai_auth_mut().logout_session(account_id.as_str())
}

#[tauri::command]
pub fn logout_openai_session(
    state: State<'_, RuntimeState>,
    account_id: String,
) -> Result<bool> {
    logout_openai_session_from_runtime(&state, account_id)
}

fn official_primary_session() -> OfficialSession {
    OfficialSession {
        session_id: "official-session-1".into(),
        account_identity: Some("user@example.com".into()),
        auth_mode: None,
        refresh_capability: Some(true),
        quota_capability: Some(false),
        last_verified_at_ms: None,
        status: "active".to_string(),
    }
}

pub(crate) fn default_primary_account() -> ImportedOfficialAccount {
    ImportedOfficialAccount {
        account_id: OFFICIAL_PRIMARY_ACCOUNT_ID.into(),
        name: "Primary Publisher".into(),
        provider: "openai".into(),
        session: official_primary_session(),
        session_credential_ref: "credential://official/session/official-primary".into(),
        token_credential_ref: "credential://official/token/official-primary".into(),
    }
}

fn official_session_for(state: &AppState, account_id: &str) -> Result<OfficialSession> {
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

fn validate_not_conflicting_with_relay_id(state: &AppState, account_id: &str) -> Result<()> {
    if crate::commands::relays::list_relays_from_state(state)
        .iter()
        .any(|relay| relay.relay_id == account_id)
    {
        Err(invalid_payload_error(
            format!("account_id conflicts with existing relay id: {account_id}"),
            format!(
                "command=account_import_validation;field=account_id;value={account_id};reason=relay_conflict"
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

fn account_balance_status(snapshot: &AccountBalanceSnapshot) -> &'static str {
    match snapshot.balance {
        AccountBalanceAvailability::Queryable { .. } => "queryable",
        AccountBalanceAvailability::NonQueryable { .. } => "non_queryable",
    }
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
