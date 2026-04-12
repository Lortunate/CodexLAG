use serde::Serialize;
use tauri::State;

use crate::logging::usage::{
    query_usage_ledger as query_usage_ledger_model, request_detail, request_history, UsageLedger,
    UsageLedgerQuery, UsageRequestDetail,
};
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

#[tauri::command]
pub fn get_usage_request_detail(
    state: State<'_, RuntimeState>,
    request_id: String,
) -> Option<UsageRequestDetail> {
    usage_request_detail_from_runtime(&state, request_id.as_str())
}

#[tauri::command]
pub fn list_usage_request_history(
    state: State<'_, RuntimeState>,
    limit: Option<usize>,
) -> Vec<UsageRequestDetail> {
    usage_request_history_from_runtime(&state, limit)
}

#[tauri::command]
pub fn query_usage_ledger(
    state: State<'_, RuntimeState>,
    query: Option<UsageLedgerQuery>,
) -> UsageLedger {
    usage_ledger_from_runtime(&state, query)
}

pub fn usage_request_detail_from_runtime(
    runtime: &RuntimeState,
    request_id: &str,
) -> Option<UsageRequestDetail> {
    let records = runtime.loopback_gateway().state().usage_records();
    request_detail(&records, request_id)
}

pub fn usage_request_history_from_runtime(
    runtime: &RuntimeState,
    limit: Option<usize>,
) -> Vec<UsageRequestDetail> {
    let records = runtime.loopback_gateway().state().usage_records();
    request_history(&records, limit)
}

pub fn usage_ledger_from_runtime(
    runtime: &RuntimeState,
    query: Option<UsageLedgerQuery>,
) -> UsageLedger {
    let records = runtime.loopback_gateway().state().usage_records();
    query_usage_ledger_model(&records, query.unwrap_or_default())
}
