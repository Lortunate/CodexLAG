use serde::Serialize;
use tauri::State;

use crate::state::{AppState, RuntimeState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicySummary {
    pub name: String,
    pub status: String,
}

pub fn policy_summaries_from_state(state: &AppState) -> Vec<PolicySummary> {
    let mut summaries = state
        .iter_policies()
        .map(|policy| {
            let status = if state
                .iter_platform_keys()
                .any(|key| key.enabled && key.policy_id == policy.id)
            {
                "active"
            } else {
                "inactive"
            };

            PolicySummary {
                name: policy.name.clone(),
                status: status.into(),
            }
        })
        .collect::<Vec<_>>();

    summaries.sort_by(|left, right| left.name.cmp(&right.name));
    summaries
}

#[tauri::command]
pub fn list_policies(state: State<'_, RuntimeState>) -> Vec<PolicySummary> {
    policy_summaries_from_state(&state.app_state())
}
