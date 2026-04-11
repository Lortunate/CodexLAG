use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct RelaySummary {
    pub name: String,
    pub endpoint: String,
}

#[tauri::command]
pub fn list_relays() -> Vec<RelaySummary> {
    vec![RelaySummary {
        name: "Local Gateway".into(),
        endpoint: "http://127.0.0.1:8787".into(),
    }]
}
