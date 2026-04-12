use serde::Serialize;
use tauri::{Emitter, Runtime, State};

use crate::{
    error::{CodexLagError, Result},
    routing::policy::RoutingMode,
    state::{AppState, RuntimeState},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
    pub unavailable_reason: Option<String>,
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
    let mode = RoutingMode::parse(mode)
        .ok_or_else(|| CodexLagError::new(format!("unsupported default key mode '{}'", mode)))?;

    runtime.set_current_mode(mode)?;
    default_key_summary_from_runtime(runtime)
}

pub fn default_key_summary_from_runtime(runtime: &RuntimeState) -> Result<DefaultKeySummary> {
    let mut summary = default_key_summary_from_state(&runtime.app_state())?;
    if !runtime
        .loopback_gateway()
        .state()
        .has_available_endpoint_for_mode(summary.allowed_mode.as_str())
    {
        summary.unavailable_reason = Some(format!(
            "no available endpoint for mode '{}'",
            summary.allowed_mode
        ));
    }
    Ok(summary)
}

pub fn emit_default_key_summary_changed<R: Runtime>(
    app: &tauri::AppHandle<R>,
    summary: &DefaultKeySummary,
) -> tauri::Result<()> {
    app.emit(DEFAULT_KEY_SUMMARY_CHANGED_EVENT, summary)
}

#[tauri::command]
pub fn get_default_key_summary(
    state: State<'_, RuntimeState>,
) -> std::result::Result<DefaultKeySummary, String> {
    default_key_summary_from_runtime(&state).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn set_default_key_mode(
    app: tauri::AppHandle,
    mode: String,
    state: State<'_, RuntimeState>,
) -> std::result::Result<DefaultKeySummary, String> {
    let summary = set_default_key_mode_from_runtime(&state, &mode).map_err(|error| error.to_string())?;
    emit_default_key_summary_changed(&app, &summary).map_err(|error| error.to_string())?;
    Ok(summary)
}
