use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DefaultKeySummary {
    pub name: String,
    pub allowed_mode: String,
}

#[tauri::command]
pub fn get_default_key_summary() -> DefaultKeySummary {
    DefaultKeySummary {
        name: "default".into(),
        allowed_mode: "hybrid".into(),
    }
}
