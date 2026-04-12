use serde::Serialize;
use tauri::State;

use crate::logging::usage::{
    query_usage_ledger as query_usage_ledger_model, record_request, request_detail,
    request_history, UsageLedger, UsageLedgerQuery, UsageRecord, UsageRecordInput,
    UsageRequestDetail,
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
pub fn get_usage_request_detail(request_id: String) -> Option<UsageRequestDetail> {
    let records = sample_usage_records();
    request_detail(&records, request_id.as_str())
}

#[tauri::command]
pub fn list_usage_request_history(limit: Option<usize>) -> Vec<UsageRequestDetail> {
    let records = sample_usage_records();
    request_history(&records, limit)
}

#[tauri::command]
pub fn query_usage_ledger(query: Option<UsageLedgerQuery>) -> UsageLedger {
    let records = sample_usage_records();
    query_usage_ledger_model(&records, query.unwrap_or_default())
}

fn sample_usage_records() -> Vec<UsageRecord> {
    vec![
        record_request(UsageRecordInput {
            request_id: "req-1".into(),
            endpoint_id: "official-1".into(),
            input_tokens: 120,
            output_tokens: 30,
            cache_read_tokens: 10,
            cache_write_tokens: 0,
            estimated_cost: "0.0123".into(),
        }),
        record_request(UsageRecordInput {
            request_id: "req-2".into(),
            endpoint_id: "relay-1".into(),
            input_tokens: 40,
            output_tokens: 15,
            cache_read_tokens: 5,
            cache_write_tokens: 2,
            estimated_cost: "".into(),
        }),
    ]
}
