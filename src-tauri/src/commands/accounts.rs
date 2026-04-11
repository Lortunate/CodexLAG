use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AccountSummary {
    pub name: String,
    pub provider: String,
}

#[tauri::command]
pub fn list_accounts() -> Vec<AccountSummary> {
    vec![AccountSummary {
        name: "Primary Publisher".into(),
        provider: "OpenAI".into(),
    }]
}
