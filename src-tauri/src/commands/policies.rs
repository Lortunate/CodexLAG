use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};
use tauri::State;

use crate::models::{FailureRules, RecoveryRules, RoutingPolicy};
use crate::state::{AppState, RuntimeState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PolicySummary {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyUpdateInput {
    pub policy_id: String,
    pub name: String,
    pub selection_order: Vec<String>,
    pub cross_pool_fallback: bool,
    pub retry_budget: u32,
    pub timeout_open_after: u32,
    pub server_error_open_after: u32,
    pub cooldown_ms: u64,
    pub half_open_after_ms: u64,
    pub success_close_after: u32,
}

static MANAGED_POLICIES: OnceLock<Mutex<HashMap<String, RoutingPolicy>>> = OnceLock::new();

fn managed_policies() -> &'static Mutex<HashMap<String, RoutingPolicy>> {
    MANAGED_POLICIES.get_or_init(|| {
        let mut seeded = HashMap::new();
        seeded.insert(
            "default".to_string(),
            RoutingPolicy {
                id: "default".to_string(),
                name: "Default Policy".to_string(),
                selection_order: vec!["official-primary".to_string(), "relay-newapi".to_string()],
                cross_pool_fallback: true,
                retry_budget: 1,
                failure_rules: FailureRules::default(),
                recovery_rules: RecoveryRules::default(),
            },
        );
        Mutex::new(seeded)
    })
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

#[tauri::command]
pub fn update_policy(input: PolicyUpdateInput) -> Result<PolicyUpdateInput, String> {
    validate_policy_update_input(&input)?;

    let mut policies = managed_policies()
        .lock()
        .expect("managed policy store lock poisoned");
    if !policies.contains_key(input.policy_id.as_str()) {
        return Err(format!("unknown policy id: {}", input.policy_id));
    }

    let policy = RoutingPolicy {
        id: input.policy_id.clone(),
        name: input.name.clone(),
        selection_order: input.selection_order.clone(),
        cross_pool_fallback: input.cross_pool_fallback,
        retry_budget: input.retry_budget,
        failure_rules: FailureRules {
            cooldown_ms: input.cooldown_ms,
            timeout_open_after: input.timeout_open_after,
            server_error_open_after: input.server_error_open_after,
        },
        recovery_rules: RecoveryRules {
            half_open_after_ms: input.half_open_after_ms,
            success_close_after: input.success_close_after,
        },
    };
    policies.insert(policy.id.clone(), policy);
    Ok(input)
}

fn validate_policy_update_input(input: &PolicyUpdateInput) -> Result<(), String> {
    if input.policy_id.trim().is_empty() {
        return Err("policy_id must not be empty".to_string());
    }
    if input.name.trim().is_empty() {
        return Err("name must not be empty".to_string());
    }
    if input.selection_order.is_empty() {
        return Err("selection_order must contain at least one endpoint id".to_string());
    }

    let mut seen = HashSet::new();
    for endpoint_id in &input.selection_order {
        let endpoint_id = endpoint_id.trim();
        if endpoint_id.is_empty() {
            return Err("selection_order entries must not be empty".to_string());
        }
        if !endpoint_id.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        }) {
            return Err(
                "selection_order entries must use only ascii letters, numbers, '-' or '_'"
                    .to_string(),
            );
        }
        if !seen.insert(endpoint_id.to_string()) {
            return Err(format!(
                "selection_order must not contain duplicate endpoint ids: {endpoint_id}"
            ));
        }
    }

    if input.retry_budget == 0 {
        return Err("retry_budget must be greater than 0".to_string());
    }
    if input.timeout_open_after == 0 {
        return Err("timeout_open_after must be greater than 0".to_string());
    }
    if input.server_error_open_after == 0 {
        return Err("server_error_open_after must be greater than 0".to_string());
    }
    if input.cooldown_ms == 0 {
        return Err("cooldown_ms must be greater than 0".to_string());
    }
    if input.half_open_after_ms == 0 {
        return Err("half_open_after_ms must be greater than 0".to_string());
    }
    if input.success_close_after == 0 {
        return Err("success_close_after must be greater than 0".to_string());
    }

    Ok(())
}
