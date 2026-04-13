use serde::Serialize;
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::path::Path;
use std::time::SystemTime;
use tauri::State;

use crate::error::{CodexLagError, ConfigErrorKind, Result};
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
    let current_mode = runtime.current_mode();
    let mode = current_mode.as_str();
    let gateway_ready = runtime.loopback_gateway().is_ready_for_mode(mode);
    let level = if gateway_ready { "info" } else { "warn" };
    let last_event = if gateway_ready {
        format!(
            "Loopback gateway ready for key '{}' in {} mode",
            key_name, mode
        )
    } else {
        format!(
            "Loopback gateway unavailable for key '{}' in {} mode",
            key_name, mode
        )
    };

    LogSummary {
        last_event,
        level: level.into(),
    }
}

#[tauri::command]
pub fn get_log_summary(state: State<'_, RuntimeState>) -> LogSummary {
    log_summary_from_runtime(&state)
}

pub fn runtime_log_metadata_from_runtime(runtime: &RuntimeState) -> Result<RuntimeLogMetadata> {
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
        Err(error) => {
            return Err(CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to read runtime log metadata.",
            )
            .with_internal_context(format!(
                "command=get_runtime_log_metadata;log_dir={};cause={error}",
                log_dir.display()
            )));
        }
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
pub fn get_runtime_log_metadata(state: State<'_, RuntimeState>) -> Result<RuntimeLogMetadata> {
    runtime_log_metadata_from_runtime(&state)
}

#[tauri::command]
pub fn export_runtime_diagnostics(state: State<'_, RuntimeState>) -> Result<String> {
    export_runtime_diagnostics_from_runtime(&state)
}

pub fn export_runtime_diagnostics_from_runtime(runtime: &RuntimeState) -> Result<String> {
    let metadata = runtime_log_metadata_from_runtime(runtime)?;
    let log_dir = runtime.runtime_log().log_dir.clone();
    let diagnostics_dir = log_dir.join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).map_err(|error| {
        CodexLagError::config(
            ConfigErrorKind::Unknown,
            "Failed to create diagnostics directory.",
        )
        .with_internal_context(format!(
            "command=export_runtime_diagnostics;operation=create_dir_all;diagnostics_dir={};cause={error}",
            diagnostics_dir.display()
        ))
    })?;

    let generated_at_unix = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| {
            CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to derive diagnostics timestamp.",
            )
            .with_internal_context(format!(
                "command=export_runtime_diagnostics;operation=duration_since_unix_epoch;cause={error}"
            ))
        })?
        .as_secs();

    let files_payload = serde_json::to_string(&metadata.files).map_err(|error| {
        CodexLagError::config(
            ConfigErrorKind::Unknown,
            "Failed to serialize diagnostics manifest files.",
        )
        .with_internal_context(format!(
            "command=export_runtime_diagnostics;operation=serialize_manifest_files;cause={error}"
        ))
    })?;
    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    let manifest = redact_token_like_values(&format!(
        "generated_at_unix={generated_at_unix}\nlog_dir={}\nfiles_count={}\nfiles={files_payload}\n",
        metadata.log_dir,
        metadata.files.len()
    ));

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
    if file_name.ends_with(".log") {
        return true;
    }

    if let Some((_, suffix)) = file_name.split_once(".log.") {
        return !suffix.is_empty();
    }

    false
}

fn diagnostics_manifest_display_path(log_dir: &Path) -> String {
    format!(
        "{}/diagnostics/diagnostics-manifest.txt",
        sanitize_log_dir_for_display(log_dir)
    )
}

fn redact_token_like_values(value: &str) -> String {
    let value = redact_prefixed_token(value, "ck_local_");
    redact_bearer_token(&value)
}

fn redact_prefixed_token(value: &str, prefix: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;

    while let Some(relative_start) = value[cursor..].find(prefix) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        output.push_str(prefix);
        output.push_str("[redacted]");

        let mut token_end = start + prefix.len();
        for ch in value[token_end..].chars() {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                token_end += ch.len_utf8();
            } else {
                break;
            }
        }

        cursor = token_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn redact_bearer_token(value: &str) -> String {
    let marker = "bearer ";
    let lowercase = value.to_ascii_lowercase();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;

    while let Some(relative_start) = lowercase[cursor..].find(marker) {
        let start = cursor + relative_start;
        output.push_str(&value[cursor..start]);
        output.push_str(marker);
        output.push_str("[redacted]");

        let mut token_end = start + marker.len();
        for ch in value[token_end..].chars() {
            if ch.is_whitespace() || ch == '"' || ch == '\'' || ch == ',' {
                break;
            }
            token_end += ch.len_utf8();
        }
        cursor = token_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn write_file_atomically(target_path: &Path, content: &str) -> Result<()> {
    let target_parent = target_path.parent().ok_or_else(|| {
        CodexLagError::config(
            ConfigErrorKind::Unknown,
            "Failed to derive diagnostics manifest directory.",
        )
        .with_internal_context(format!(
            "command=export_runtime_diagnostics;operation=derive_manifest_directory;target_path={}",
            target_path.display()
        ))
    })?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|error| {
            CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to derive diagnostics temp-file nonce.",
            )
            .with_internal_context(format!(
                "command=export_runtime_diagnostics;operation=derive_temp_file_nonce;target_path={};cause={error}",
                target_path.display()
            ))
        })?
        .as_nanos();
    let temp_path = target_parent.join(format!(
        ".diagnostics-manifest.tmp-{}-{nonce}",
        std::process::id()
    ));

    std::fs::write(&temp_path, content).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        CodexLagError::config(
            ConfigErrorKind::Unknown,
            "Failed to write diagnostics temp manifest.",
        )
        .with_internal_context(format!(
            "command=export_runtime_diagnostics;operation=write_temp_manifest;temp_path={};cause={error}",
            temp_path.display()
        ))
    })?;

    if let Err(rename_error) = std::fs::rename(&temp_path, target_path) {
        let cleanup_error = std::fs::remove_file(&temp_path).err();
        return Err(match cleanup_error {
            Some(cleanup_error) => CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to atomically replace diagnostics manifest.",
            )
            .with_internal_context(format!(
                "command=export_runtime_diagnostics;operation=rename_manifest;temp_path={};target_path={};cause={rename_error};cleanup_cause={cleanup_error}",
                temp_path.display(),
                target_path.display()
            )),
            None => CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to atomically replace diagnostics manifest.",
            )
            .with_internal_context(format!(
                "command=export_runtime_diagnostics;operation=rename_manifest;temp_path={};target_path={};cause={rename_error}",
                temp_path.display(),
                target_path.display()
            )),
        });
    }

    Ok(())
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
