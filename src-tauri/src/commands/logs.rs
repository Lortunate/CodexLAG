use serde::Serialize;
use std::path::Path;
use std::time::SystemTime;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeLogMetadata {
    pub log_dir: String,
    pub files: Vec<String>,
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

pub fn runtime_log_metadata_from_runtime(
    runtime: &RuntimeState,
) -> Result<RuntimeLogMetadata, String> {
    const MAX_SCAN_ENTRIES: usize = 200;
    const MAX_FILES: usize = 20;

    let log_dir = runtime.runtime_log().log_dir.clone();
    let mut files = match std::fs::read_dir(&log_dir) {
        Ok(entries) => entries
            .take(MAX_SCAN_ENTRIES)
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let file_type = entry.file_type().ok()?;
                if !file_type.is_file() {
                    return None;
                }

                let file_name = entry.file_name().to_string_lossy().to_string();
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                Some((file_name, modified))
            })
            .collect::<Vec<_>>(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("failed to read runtime log directory: {error}")),
    };

    files.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));

    Ok(RuntimeLogMetadata {
        log_dir: sanitize_log_dir_for_display(&log_dir),
        files: files
            .into_iter()
            .take(MAX_FILES)
            .map(|(file_name, _)| file_name)
            .collect(),
    })
}

#[tauri::command]
pub fn get_runtime_log_metadata(
    state: State<'_, RuntimeState>,
) -> Result<RuntimeLogMetadata, String> {
    runtime_log_metadata_from_runtime(&state)
}

fn sanitize_log_dir_for_display(path: &Path) -> String {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("logs") => "<app-local-data>/logs".into(),
        Some(tail) => format!("<app-local-data>/{tail}"),
        None => "<app-local-data>/logs".into(),
    }
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
