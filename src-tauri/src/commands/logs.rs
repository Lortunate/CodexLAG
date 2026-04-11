use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct LogSummary {
    pub last_event: String,
    pub level: String,
}

#[tauri::command]
pub fn get_log_summary() -> LogSummary {
    LogSummary {
        last_event: "Gateway started".into(),
        level: "info".into(),
    }
}
