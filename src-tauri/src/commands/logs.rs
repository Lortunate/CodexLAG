use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
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
    const MAX_FILES: usize = 20;

    let log_dir = runtime.runtime_log().log_dir.clone();
    let files = match std::fs::read_dir(&log_dir) {
        Ok(entries) => {
            let mut recent_files: BinaryHeap<Reverse<(SystemTime, String)>> = BinaryHeap::new();

            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                let file_type = match entry.file_type() {
                    Ok(file_type) => file_type,
                    Err(_) => continue,
                };
                if !file_type.is_file() {
                    continue;
                }

                let file_name = entry.file_name().to_string_lossy().to_string();
                if !is_runtime_log_file_name(&file_name) {
                    continue;
                }
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                recent_files.push(Reverse((modified, file_name)));
                if recent_files.len() > MAX_FILES {
                    recent_files.pop();
                }
            }

            let mut files = recent_files
                .into_iter()
                .map(|Reverse((modified, file_name))| (file_name, modified))
                .collect::<Vec<_>>();
            files.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
            files
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(error) => return Err(format!("failed to read runtime log directory: {error}")),
    };

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

#[tauri::command]
pub fn export_runtime_diagnostics(state: State<'_, RuntimeState>) -> Result<String, String> {
    export_runtime_diagnostics_from_runtime(&state)
}

pub fn export_runtime_diagnostics_from_runtime(runtime: &RuntimeState) -> Result<String, String> {
    let metadata = runtime_log_metadata_from_runtime(runtime)?;
    let log_dir = runtime.runtime_log().log_dir.clone();
    let diagnostics_dir = log_dir.join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir)
        .map_err(|error| format!("failed to create diagnostics directory: {error}"))?;

    let generated_at_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| format!("failed to derive diagnostics timestamp: {error}"))?
        .as_secs();

    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    let mut manifest = format!(
        "generated_at_unix={generated_at_unix}\nlog_dir={}\nfiles_count={}\nfiles:\n",
        metadata.log_dir,
        metadata.files.len()
    );
    for file_name in metadata.files {
        manifest.push_str("- ");
        manifest.push_str(&file_name);
        manifest.push('\n');
    }

    write_file_atomically(&manifest_path, &manifest)?;

    Ok(diagnostics_manifest_display_path(&log_dir))
}

fn sanitize_log_dir_for_display(path: &Path) -> String {
    match path.file_name().and_then(|name| name.to_str()) {
        Some("logs") => "<app-local-data>/logs".into(),
        Some(tail) => format!("<app-local-data>/{tail}"),
        None => "<app-local-data>/logs".into(),
    }
}

fn is_runtime_log_file_name(file_name: &str) -> bool {
    let file_name = file_name.to_ascii_lowercase();
    file_name.ends_with(".log")
        || file_name.contains(".log.")
        || file_name == "gateway"
        || file_name.starts_with("gateway.")
        || file_name.starts_with("gateway-")
}

fn diagnostics_manifest_display_path(log_dir: &Path) -> String {
    format!(
        "{}/diagnostics/diagnostics-manifest.txt",
        sanitize_log_dir_for_display(log_dir)
    )
}

fn write_file_atomically(target_path: &Path, content: &str) -> Result<(), String> {
    let target_parent = target_path
        .parent()
        .ok_or_else(|| "failed to derive diagnostics manifest directory".to_string())?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| format!("failed to derive diagnostics temp-file nonce: {error}"))?
        .as_nanos();
    let temp_path = target_parent.join(format!(
        ".diagnostics-manifest.tmp-{}-{nonce}",
        std::process::id()
    ));

    std::fs::write(&temp_path, content)
        .map_err(|error| format!("failed to write diagnostics temp manifest: {error}"))?;

    match std::fs::rename(&temp_path, target_path) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            if target_path.exists() {
                std::fs::remove_file(target_path)
                    .map_err(|error| format!("failed to replace diagnostics manifest: {error}"))?;
                std::fs::rename(&temp_path, target_path)
                    .map_err(|error| format!("failed to finalize diagnostics manifest: {error}"))?;
                return Ok(());
            }

            let _ = std::fs::remove_file(&temp_path);
            Err(format!(
                "failed to atomically replace diagnostics manifest: {rename_error}"
            ))
        }
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
