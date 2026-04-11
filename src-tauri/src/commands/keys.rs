use serde::Serialize;
use tauri::State;

use crate::{
    error::{CodexLagError, Result},
    state::{AppState, RuntimeState},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
}

pub fn default_key_summary_from_state(state: &AppState) -> Result<DefaultKeySummary> {
    let key = state
        .default_platform_key()
        .ok_or_else(|| CodexLagError::new("default platform key missing from app state"))?;

    Ok(DefaultKeySummary {
        name: key.name.clone(),
        allowed_mode: key.allowed_mode.clone(),
    })
}

#[tauri::command]
pub fn get_default_key_summary(
    state: State<'_, RuntimeState>,
) -> std::result::Result<DefaultKeySummary, String> {
    default_key_summary_from_state(state.app_state()).map_err(|error| error.to_string())
}
