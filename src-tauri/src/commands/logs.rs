use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tauri::State;

use crate::error::{CodexLagError, ConfigErrorKind, Result};
use crate::logging::diagnostics::build_provider_diagnostics_summary;
pub use crate::logging::diagnostics::{
    DiagnosticsDetail, DiagnosticsRow, DiagnosticsSection, ProviderDiagnosticsSummary,
};
use crate::logging::redaction::redact_sensitive_value;
use crate::logging::usage::{
    UsageCost, UsageLedger, UsageLedgerQuery, UsageProvenance, UsageRequestDetail,
};
use crate::state::{RuntimeLogFileMetadata as RuntimeLogFileMetadataState, RuntimeState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogSummary {
    pub last_event: String,
    pub level: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeLogMetadata {
    pub log_dir: String,
    pub files: Vec<RuntimeLogFileMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeLogFileMetadata {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub mtime: u64,
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
    let files = runtime
        .runtime_log()
        .recent_log_files(MAX_FILES)
        .map_err(|error| {
            CodexLagError::config(
                ConfigErrorKind::Unknown,
                "Failed to read runtime log metadata.",
            )
            .with_internal_context(format!(
                "command=get_runtime_log_metadata;log_dir={};cause={error}",
                log_dir.display()
            ))
        })?;
    let log_dir_display = sanitize_log_dir_for_display(&log_dir);

    Ok(RuntimeLogMetadata {
        log_dir: log_dir_display.clone(),
        files: files
            .into_iter()
            .map(|file| runtime_log_file_metadata_for_display(&log_dir, &log_dir_display, file))
            .collect(),
    })
}

#[tauri::command]
pub fn get_runtime_log_metadata(state: State<'_, RuntimeState>) -> Result<RuntimeLogMetadata> {
    runtime_log_metadata_from_runtime(&state)
}

#[tauri::command]
pub fn get_provider_diagnostics(
    state: State<'_, RuntimeState>,
) -> Result<ProviderDiagnosticsSummary> {
    provider_diagnostics_from_runtime(&state)
}

#[tauri::command]
pub fn export_runtime_diagnostics(state: State<'_, RuntimeState>) -> Result<String> {
    export_runtime_diagnostics_from_runtime(&state)
}

pub fn provider_diagnostics_from_runtime(
    runtime: &RuntimeState,
) -> Result<ProviderDiagnosticsSummary> {
    Ok(build_provider_diagnostics_summary(runtime))
}

pub fn export_runtime_diagnostics_from_runtime(runtime: &RuntimeState) -> Result<String> {
    let metadata = runtime_log_metadata_from_runtime(runtime)?;
    let diagnostics = provider_diagnostics_from_runtime(runtime)?;
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
    let provider_diagnostics_payload = serde_json::to_string(&diagnostics).map_err(|error| {
        CodexLagError::config(
            ConfigErrorKind::Unknown,
            "Failed to serialize provider diagnostics summary.",
        )
        .with_internal_context(format!(
            "command=export_runtime_diagnostics;operation=serialize_provider_diagnostics;cause={error}"
        ))
    })?;
    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    let manifest = redact_sensitive_value(&format!(
        "generated_at_unix={generated_at_unix}\nlog_dir={}\nfiles_count={}\nfiles={files_payload}\nprovider_diagnostics={provider_diagnostics_payload}\n",
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

fn diagnostics_manifest_display_path(log_dir: &Path) -> String {
    format!(
        "{}/diagnostics/diagnostics-manifest.txt",
        sanitize_log_dir_for_display(log_dir)
    )
}

fn runtime_log_file_metadata_for_display(
    log_dir: &Path,
    log_dir_display: &str,
    file: RuntimeLogFileMetadataState,
) -> RuntimeLogFileMetadata {
    RuntimeLogFileMetadata {
        name: file.name.clone(),
        path: runtime_log_file_display_path(
            log_dir,
            log_dir_display,
            &file.path,
            file.name.as_str(),
        ),
        size: file.size,
        mtime: file.mtime,
    }
}

fn runtime_log_file_display_path(
    log_dir: &Path,
    log_dir_display: &str,
    file_path: &Path,
    file_name: &str,
) -> String {
    let relative_path = file_path
        .strip_prefix(log_dir)
        .ok()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(file_name));
    let relative_display = relative_path.to_string_lossy().replace('\\', "/");
    format!("{log_dir_display}/{relative_display}")
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
    runtime
        .app_state()
        .repositories()
        .request_detail(request_id)
        .expect("request detail")
}

pub fn usage_request_history_from_runtime(
    runtime: &RuntimeState,
    limit: Option<usize>,
) -> Vec<UsageRequestDetail> {
    runtime
        .app_state()
        .repositories()
        .recent_request_details(limit)
        .expect("request history")
}

pub fn usage_ledger_from_runtime(
    runtime: &RuntimeState,
    query: Option<UsageLedgerQuery>,
) -> UsageLedger {
    let query = query.unwrap_or_default();
    let mut entries = runtime
        .app_state()
        .repositories()
        .recent_request_details(None)
        .expect("request history")
        .into_iter()
        .filter(|entry| {
            query
                .endpoint_id
                .as_ref()
                .map(|endpoint_id| entry.endpoint_id == *endpoint_id)
                .unwrap_or(true)
        })
        .filter(|entry| {
            query
                .request_id_prefix
                .as_ref()
                .map(|prefix| entry.request_id.starts_with(prefix))
                .unwrap_or(true)
        })
        .collect::<Vec<_>>();

    if let Some(limit) = query.limit {
        entries.truncate(limit);
    }

    UsageLedger {
        total_tokens: entries.iter().map(|entry| entry.total_tokens).sum(),
        total_cost: aggregate_total_cost(&entries),
        entries,
    }
}

fn aggregate_total_cost(entries: &[UsageRequestDetail]) -> UsageCost {
    if entries.is_empty() {
        return UsageCost {
            amount: None,
            provenance: UsageProvenance::Unknown,
            is_estimated: false,
        };
    }

    let mut running_total = 0.0_f64;
    let mut saw_amount = false;
    let mut saw_unknown = false;
    let mut saw_estimated = false;
    let mut saw_actual = false;

    for entry in entries {
        let cost = &entry.cost;
        if let Some(amount) = cost.amount.as_deref() {
            if let Ok(parsed) = amount.parse::<f64>() {
                saw_amount = true;
                running_total += parsed;
            }
        }

        match cost.provenance {
            UsageProvenance::Actual => saw_actual = true,
            UsageProvenance::Estimated => saw_estimated = true,
            UsageProvenance::Unknown => saw_unknown = true,
        }
    }

    let provenance = if saw_unknown || (saw_actual && saw_estimated) {
        UsageProvenance::Unknown
    } else if saw_actual {
        UsageProvenance::Actual
    } else if saw_estimated {
        UsageProvenance::Estimated
    } else {
        UsageProvenance::Unknown
    };

    let amount = if saw_amount && provenance != UsageProvenance::Unknown {
        Some(format!("{running_total:.4}"))
    } else {
        None
    };

    UsageCost {
        amount,
        provenance: provenance.clone(),
        is_estimated: provenance == UsageProvenance::Estimated,
    }
}
