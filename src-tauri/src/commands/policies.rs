use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PolicySummary {
    pub name: String,
    pub status: String,
}

#[tauri::command]
pub fn list_policies() -> Vec<PolicySummary> {
    vec![PolicySummary {
        name: "Default Gateway Policy".into(),
        status: "active".into(),
    }]
}
