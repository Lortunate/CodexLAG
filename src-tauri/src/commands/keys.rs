use serde::{Deserialize, Serialize};
use tauri::{Emitter, Runtime, State};

use crate::{
    error::{CodexLagError, ConfigErrorKind, Result},
    models::PlatformKey,
    routing::policy::RoutingMode,
    state::{AppState, RuntimeState},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlatformKeyInventoryEntry {
    pub id: String,
    pub name: String,
    pub policy_id: String,
    pub allowed_mode: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePlatformKeyInput {
    pub key_id: String,
    pub name: String,
    pub policy_id: String,
    pub allowed_mode: String,
}

pub const DEFAULT_KEY_SUMMARY_CHANGED_EVENT: &str = "default-key-summary-changed";

pub fn default_key_summary_from_state(state: &AppState) -> Result<DefaultKeySummary> {
    let key = state
        .default_platform_key()
        .ok_or_else(|| CodexLagError::new("default platform key missing from app state"))?;

    Ok(DefaultKeySummary {
        name: key.name.clone(),
        allowed_mode: key.allowed_mode.clone(),
        unavailable_reason: None,
    })
}

pub fn set_default_key_mode_from_runtime(
    runtime: &RuntimeState,
    mode: &str,
) -> Result<DefaultKeySummary> {
    let mode = RoutingMode::parse(mode).ok_or_else(|| {
        CodexLagError::config(
            ConfigErrorKind::UnsupportedMode,
            "Allowed mode must be one of: hybrid, account_only, relay_only.",
        )
        .with_internal_context(format!("command=set_default_key_mode;mode={mode}"))
    })?;

    runtime.set_current_mode(mode)?;
    default_key_summary_from_runtime(runtime)
}

pub fn default_key_summary_from_runtime(runtime: &RuntimeState) -> Result<DefaultKeySummary> {
    let mut summary = default_key_summary_from_state(&runtime.app_state())?;
    summary.unavailable_reason = runtime
        .loopback_gateway()
        .state()
        .unavailable_reason_for_mode(summary.allowed_mode.as_str());
    Ok(summary)
}

pub fn emit_default_key_summary_changed<R: Runtime>(
    app: &tauri::AppHandle<R>,
    summary: &DefaultKeySummary,
) -> tauri::Result<()> {
    app.emit(DEFAULT_KEY_SUMMARY_CHANGED_EVENT, summary)
}

#[tauri::command]
pub fn get_default_key_summary(state: State<'_, RuntimeState>) -> Result<DefaultKeySummary> {
    default_key_summary_from_runtime(&state)
}

#[tauri::command]
pub fn set_default_key_mode(
    app: tauri::AppHandle,
    mode: String,
    state: State<'_, RuntimeState>,
) -> Result<DefaultKeySummary> {
    let summary = set_default_key_mode_from_runtime(&state, &mode)?;
    emit_default_key_summary_changed(&app, &summary).map_err(|error| {
        CodexLagError::new("Failed to emit default key summary update event.")
            .with_internal_context(format!(
            "command=set_default_key_mode;event={DEFAULT_KEY_SUMMARY_CHANGED_EVENT};cause={error}"
        ))
    })?;
    Ok(summary)
}

#[tauri::command]
pub fn create_platform_key(
    state: State<'_, RuntimeState>,
    input: CreatePlatformKeyInput,
) -> Result<PlatformKeyInventoryEntry> {
    create_platform_key_from_runtime(&state, input)
}

pub fn create_platform_key_from_runtime(
    runtime: &RuntimeState,
    input: CreatePlatformKeyInput,
) -> Result<PlatformKeyInventoryEntry> {
    let key_id = validate_identifier(input.key_id, "key_id")?;
    let name = validate_non_empty(input.name, "name")?;
    let policy_id = validate_identifier(input.policy_id, "policy_id")?;
    let allowed_mode = validate_allowed_mode(input.allowed_mode)?;

    let mut app_state = runtime.app_state_mut();
    if app_state.get_platform_key_by_id(key_id.as_str()).is_some() {
        return Err(CodexLagError::config(
            ConfigErrorKind::InvalidPayload,
            "Platform key id already exists.",
        )
        .with_internal_context(format!(
            "command=create_platform_key;field=key_id;value={key_id}"
        )));
    }
    if app_state.get_policy_by_id(policy_id.as_str()).is_none() {
        return Err(
            CodexLagError::config(ConfigErrorKind::InvalidPayload, "Unknown policy id.")
                .with_internal_context(format!(
                    "command=create_platform_key;field=policy_id;value={policy_id}"
                )),
        );
    }

    app_state
        .insert_platform_key(PlatformKey {
            id: key_id.clone(),
            name: name.clone(),
            policy_id: policy_id.clone(),
            allowed_mode: allowed_mode.clone(),
            enabled: true,
        })
        .map_err(|error| {
            CodexLagError::new("Failed to persist platform key.")
                .with_internal_context(format!(
                    "command=create_platform_key;operation=insert_platform_key;key_id={key_id};cause={error}"
                ))
        })?;

    let created = PlatformKeyInventoryEntry {
        id: key_id,
        name,
        policy_id,
        allowed_mode,
        enabled: true,
    };
    Ok(created)
}

#[tauri::command]
pub fn list_platform_keys(state: State<'_, RuntimeState>) -> Vec<PlatformKeyInventoryEntry> {
    list_platform_keys_from_runtime(&state)
}

pub fn list_platform_keys_from_runtime(runtime: &RuntimeState) -> Vec<PlatformKeyInventoryEntry> {
    let mut entries = runtime
        .app_state()
        .iter_platform_keys()
        .map(|key| PlatformKeyInventoryEntry {
            id: key.id.clone(),
            name: key.name.clone(),
            policy_id: key.policy_id.clone(),
            allowed_mode: key.allowed_mode.clone(),
            enabled: key.enabled,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.id.cmp(&right.id));
    entries
}

#[tauri::command]
pub fn disable_platform_key(
    state: State<'_, RuntimeState>,
    key_id: String,
) -> Result<PlatformKeyInventoryEntry> {
    disable_platform_key_from_runtime(&state, key_id)
}

pub fn disable_platform_key_from_runtime(
    runtime: &RuntimeState,
    key_id: String,
) -> Result<PlatformKeyInventoryEntry> {
    set_platform_key_enabled_from_runtime(runtime, key_id, false)
}

#[tauri::command]
pub fn enable_platform_key(
    state: State<'_, RuntimeState>,
    key_id: String,
) -> Result<PlatformKeyInventoryEntry> {
    enable_platform_key_from_runtime(&state, key_id)
}

pub fn enable_platform_key_from_runtime(
    runtime: &RuntimeState,
    key_id: String,
) -> Result<PlatformKeyInventoryEntry> {
    set_platform_key_enabled_from_runtime(runtime, key_id, true)
}

fn set_platform_key_enabled_from_runtime(
    runtime: &RuntimeState,
    key_id: String,
    enabled: bool,
) -> Result<PlatformKeyInventoryEntry> {
    let key_id = validate_identifier(key_id, "key_id")?;
    let mut app_state = runtime.app_state_mut();
    let existing = app_state
        .get_platform_key_by_id(key_id.as_str())
        .cloned()
        .ok_or_else(|| {
            CodexLagError::config(ConfigErrorKind::InvalidPayload, "Unknown platform key id.")
                .with_internal_context(format!(
                    "command=set_platform_key_enabled;field=key_id;value={key_id}"
                ))
        })?;
    app_state
        .set_platform_key_enabled_by_id(key_id.as_str(), enabled)
        .map_err(|error| {
            CodexLagError::new("Failed to persist platform key enabled state.")
                .with_internal_context(format!(
                    "command=set_platform_key_enabled;operation=set_platform_key_enabled_by_id;key_id={key_id};enabled={enabled};cause={error}"
                ))
        })?;

    Ok(PlatformKeyInventoryEntry {
        id: existing.id,
        name: existing.name,
        policy_id: existing.policy_id,
        allowed_mode: existing.allowed_mode,
        enabled,
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
        Err(CodexLagError::config(
            ConfigErrorKind::InvalidPayload,
            format!("{field_name} must use only ascii letters, numbers, '-' or '_'"),
        )
        .with_internal_context(format!(
            "command=keys_validation;field={field_name};value={value}"
        )))
    }
}

fn validate_non_empty(raw: String, field_name: &str) -> Result<String> {
    let value = raw.trim().to_string();
    if value.is_empty() {
        Err(CodexLagError::config(
            ConfigErrorKind::InvalidPayload,
            format!("{field_name} must not be empty"),
        )
        .with_internal_context(format!(
            "command=keys_validation;field={field_name};value=empty"
        )))
    } else {
        Ok(value)
    }
}

fn validate_allowed_mode(raw: String) -> Result<String> {
    let value = validate_non_empty(raw, "allowed_mode")?;
    if RoutingMode::parse(value.as_str()).is_none() {
        return Err(CodexLagError::config(
            ConfigErrorKind::UnsupportedMode,
            "Allowed mode must be one of: hybrid, account_only, relay_only.",
        )
        .with_internal_context(format!(
            "command=keys_validation;field=allowed_mode;value={value}"
        )));
    }
    Ok(value)
}
