use serde::Serialize;
use tauri::State;

use crate::state::RuntimeState;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogSummary {
    pub last_event: String,
    pub level: String,
}

pub fn log_summary_from_runtime(runtime: &RuntimeState) -> LogSummary {
    let app_state = runtime.app_state();
    let key_name = app_state
        .default_platform_key()
        .map(|key| key.name.as_str())
        .unwrap_or("missing");
    let gateway_ready = runtime.loopback_gateway().is_ready();
    let level = if gateway_ready { "info" } else { "warn" };
    let last_event = format!(
        "Loopback gateway ready for key '{}' in {} mode",
        key_name,
        runtime.current_mode().as_str()
    );

    LogSummary {
        last_event,
        level: level.into(),
    }
}

#[tauri::command]
pub fn get_log_summary(state: State<'_, RuntimeState>) -> LogSummary {
    log_summary_from_runtime(&state)
}
