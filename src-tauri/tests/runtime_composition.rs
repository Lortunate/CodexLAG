use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test_at, runtime_database_path},
    commands::{
        accounts::{import_official_account_login_from_runtime, OfficialAccountImportInput},
        keys::{
            default_key_summary_from_runtime, default_key_summary_from_state,
            set_default_key_mode_from_runtime,
        },
        logs::{
            export_runtime_diagnostics_from_runtime, log_summary_from_runtime,
            runtime_log_metadata_from_runtime,
        },
        policies::policy_summaries_from_state,
    },
    error::ErrorCategory,
    routing::{
        engine::{endpoint_rejection_reason, wall_clock_now_ms, PoolKind},
        policy::{RoutingMode, HYBRID, RELAY_ONLY},
    },
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
    tray::{build_tray_model_for_runtime, TrayItemId},
};
use std::time::Duration;

fn tray_label(model: &codexlag_lib::tray::TrayModel, id: TrayItemId) -> String {
    model
        .items
        .iter()
        .find(|item| item.id == id)
        .map(|item| item.label.text().to_string())
        .unwrap_or_else(|| panic!("missing tray item {:?}", id))
}

fn available_endpoints_label(runtime: &RuntimeState) -> String {
    let mut available_official = 0usize;
    let mut available_relay = 0usize;
    let now_ms = wall_clock_now_ms();
    for candidate in runtime.loopback_gateway().state().current_candidates() {
        if endpoint_rejection_reason(&candidate, now_ms).is_some() {
            continue;
        }
        match candidate.pool {
            PoolKind::Official => available_official += 1,
            PoolKind::Relay => available_relay += 1,
        }
    }

    format!("Available endpoints | official: {available_official}, relay: {available_relay}")
}

async fn post_loopback_request_with_retry(platform_key_secret: &str) -> reqwest::Response {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build loopback request client");

    for attempt in 0..10 {
        match client
            .post("http://127.0.0.1:8787/codex/request")
            .bearer_auth(platform_key_secret)
            .body("")
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => return response,
            Ok(response) if attempt < 9 => {
                tokio::time::sleep(Duration::from_millis(200)).await;
                drop(response);
            }
            Ok(response) => {
                panic!(
                    "gateway request should return 200, got: {}",
                    response.status()
                )
            }
            Err(_) if attempt < 9 => tokio::time::sleep(Duration::from_millis(200)).await,
            Err(error) => panic!("send gateway request: {error}"),
        }
    }

    unreachable!("loopback gateway request should return within retry budget")
}

#[tokio::test]
async fn bootstrapped_runtime_feeds_commands_and_tray_from_shared_state() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let key_summary = default_key_summary_from_state(&runtime.app_state()).expect("key summary");
    let policy_summaries = policy_summaries_from_state(&runtime.app_state());
    let log_summary = log_summary_from_runtime(&runtime);

    assert_eq!(key_summary.name, "default");
    assert_eq!(key_summary.allowed_mode, HYBRID);
    assert_eq!(policy_summaries.len(), 1);
    assert_eq!(policy_summaries[0].name, "default");
    assert_eq!(policy_summaries[0].status, "active");
    assert_eq!(
        runtime.tray_model().current_mode(),
        Some(RoutingMode::Hybrid)
    );
    let tray_model = build_tray_model_for_runtime(&runtime);
    assert_eq!(
        tray_label(&tray_model, TrayItemId::GatewayStatus),
        "Gateway status | ready"
    );
    assert_eq!(
        tray_label(&tray_model, TrayItemId::ListenAddress),
        "Listen address | http://127.0.0.1:8787"
    );
    assert_eq!(
        tray_label(&tray_model, TrayItemId::AvailableEndpoints),
        available_endpoints_label(&runtime)
    );
    assert_eq!(
        tray_label(&tray_model, TrayItemId::LastBalanceRefresh),
        "Last balance refresh | none"
    );
    assert!(runtime.loopback_gateway().is_ready());
    assert_eq!(log_summary.level, "info");
    assert!(log_summary.last_event.contains("default"));
    assert!(log_summary.last_event.contains(HYBRID));
    let default_secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default platform key secret");
    assert!(default_secret.starts_with("ck_local_"));
}

#[tokio::test]
async fn bootstrapped_runtime_uses_inventory_derived_gateway_candidates() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let candidates = runtime.loopback_gateway().state().current_candidates();
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "official-primary"),
        "runtime candidate inventory should include official-primary"
    );
    assert!(
        candidates
            .iter()
            .any(|candidate| candidate.id == "relay-newapi"),
        "runtime candidate inventory should include relay-newapi"
    );
}

#[tokio::test]
async fn runtime_exposes_gateway_host_status_for_the_running_loopback_server() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let status = runtime.gateway_host_status();
    assert!(status.is_running);
    assert_eq!(status.listen_addr.ip().to_string(), "127.0.0.1");
    assert_eq!(status.listen_addr.port(), 8787);
}

#[tokio::test]
async fn inventory_changes_restart_gateway_host_with_latest_runtime_state() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    import_official_account_login_from_runtime(
        &runtime,
        OfficialAccountImportInput {
            account_id: "official-sync-check".into(),
            name: "sync-check".into(),
            provider: "openai".into(),
            session_credential_ref: "credential://official/session/official-sync-check".into(),
            token_credential_ref: "credential://official/token/official-sync-check".into(),
            account_identity: Some("sync-check@example.com".into()),
            auth_mode: None,
        },
    )
    .expect("import account should trigger inventory sync");

    let default_secret = runtime
        .app_state()
        .secret(&SecretKey::default_platform_key())
        .expect("default platform key secret");

    let response = post_loopback_request_with_retry(default_secret.as_str()).await;

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        runtime
            .loopback_gateway()
            .state()
            .last_route_debug()
            .is_some(),
        "runtime gateway state should receive route debug updates from live host requests"
    );
}

#[tokio::test]
async fn runtime_mode_switch_updates_default_key_summary_and_tray_model() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("switch to relay-only");

    let key_summary = default_key_summary_from_state(&runtime.app_state()).expect("key summary");

    assert_eq!(key_summary.allowed_mode, RELAY_ONLY);
    assert_eq!(runtime.current_mode(), RoutingMode::RelayOnly);
    assert_eq!(
        runtime.tray_model().current_mode(),
        Some(RoutingMode::RelayOnly)
    );
    let tray_model = build_tray_model_for_runtime(&runtime);
    assert_eq!(
        tray_label(&tray_model, TrayItemId::CurrentMode),
        "Default key state | Current mode: relay_only"
    );
    assert_eq!(
        tray_label(&tray_model, TrayItemId::GatewayStatus),
        "Gateway status | ready"
    );
}

#[tokio::test]
async fn runtime_log_summary_warns_when_current_mode_has_no_available_endpoint() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    runtime
        .set_current_mode(RoutingMode::RelayOnly)
        .expect("switch to relay-only");

    for candidate in runtime.loopback_gateway().state().current_candidates() {
        if candidate.pool == PoolKind::Relay {
            let updated = runtime
                .loopback_gateway()
                .state()
                .set_endpoint_availability(candidate.id.as_str(), false);
            assert!(updated, "relay candidate availability should be mutable");
        }
    }

    let log_summary = log_summary_from_runtime(&runtime);

    assert_eq!(log_summary.level, "warn");
    assert!(log_summary.last_event.contains(RELAY_ONLY));
}

#[tokio::test]
async fn runtime_default_key_summary_marks_unavailable_mode_when_no_candidates_exist() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    for candidate in runtime.loopback_gateway().state().current_candidates() {
        let updated = runtime
            .loopback_gateway()
            .state()
            .set_endpoint_availability(candidate.id.as_str(), false);
        assert!(
            updated,
            "candidate availability should be mutable for runtime updates"
        );
    }

    let summary = default_key_summary_from_runtime(&runtime).expect("default key summary");
    assert_eq!(summary.allowed_mode, HYBRID);
    assert_eq!(
        summary.unavailable_reason,
        Some("no available endpoint for mode 'hybrid'".to_string())
    );
}

#[tokio::test]
async fn set_default_key_mode_rejects_invalid_mode_strings() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let error = set_default_key_mode_from_runtime(&runtime, "invalid-mode")
        .expect_err("invalid mode should fail");
    let payload = error.to_payload();

    assert_eq!(payload.code, "config.unsupported_mode");
    assert_eq!(payload.category, ErrorCategory::ConfigError);
    assert_eq!(
        payload.message,
        "Allowed mode must be one of: hybrid, account_only, relay_only."
    );
    assert!(payload
        .internal_context
        .unwrap_or_default()
        .contains("command=set_default_key_mode;mode=invalid-mode"));
}

#[test]
fn runtime_database_path_uses_app_local_data_dir_with_sqlite_filename() {
    let app_local_data_dir = std::path::Path::new("/tmp/codexlag-app");
    let derived = runtime_database_path(app_local_data_dir);

    assert_eq!(
        derived,
        std::path::PathBuf::from("/tmp/codexlag-app").join("codexlag.sqlite3")
    );
}

#[tokio::test]
async fn runtime_log_metadata_exposes_log_dir_and_existing_files() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let log_dir = runtime.runtime_log().log_dir.clone();
    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    for index in 0..30 {
        let file = log_dir.join(format!("gateway-{index:02}.log"));
        std::fs::write(&file, format!("entry-{index}")).expect("write log file");
    }
    std::fs::write(log_dir.join("gateway.log.2026-04-12"), "rotated-entry")
        .expect("write rotated gateway log file");
    std::fs::write(log_dir.join("gateway.backup"), "non-log gateway backup")
        .expect("write non-log gateway backup");
    std::fs::write(log_dir.join("gateway-snapshot"), "non-log gateway snapshot")
        .expect("write non-log gateway snapshot");
    std::fs::write(log_dir.join("notes.txt"), "non-log").expect("write non-log file");

    let metadata = runtime_log_metadata_from_runtime(&runtime).expect("runtime log metadata");

    assert_eq!(metadata.log_dir, "<app-local-data>/logs");
    assert_ne!(metadata.log_dir, log_dir.to_string_lossy());
    assert!(metadata
        .files
        .iter()
        .all(|file| !file.name.ends_with(".txt")));
    assert!(!metadata.files.iter().any(|file| file.name == "notes.txt"));
    assert!(!metadata
        .files
        .iter()
        .any(|file| file.name == "gateway.backup"));
    assert!(!metadata
        .files
        .iter()
        .any(|file| file.name == "gateway-snapshot"));
    assert!(metadata
        .files
        .iter()
        .all(|file| file.name.ends_with(".log") || file.name.contains(".log.")));
    assert!(metadata.files.iter().all(|file| !file.path.is_empty()));
    assert!(metadata
        .files
        .iter()
        .all(|file| file.path.starts_with("<app-local-data>/logs/")));
    assert!(metadata.files.iter().all(|file| file.size > 0));
    assert!(metadata.files.iter().all(|file| file.mtime > 0));
    assert!(metadata.files.len() <= 20);
}

#[tokio::test]
async fn diagnostics_export_returns_manifest_path() {
    let isolated_root = isolated_test_root("diagnostics-export-success");
    let database_path = isolated_root.join("state.sqlite3");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let log_dir = isolated_root.join("logs");
    let runtime = RuntimeState::new(
        app_state,
        RuntimeLogConfig {
            log_dir: log_dir.clone(),
        },
    );

    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    std::fs::write(log_dir.join("gateway-export.log"), "entry-export")
        .expect("write export log file");
    std::fs::write(log_dir.join("ck_local_abc123xyz.log"), "tokenized filename")
        .expect("write token-like log file");
    std::fs::write(log_dir.join("Bearer demo-token.log"), "bearer filename")
        .expect("write bearer-like log file");

    let manifest_display_path =
        export_runtime_diagnostics_from_runtime(&runtime).expect("export runtime diagnostics");
    assert_eq!(
        manifest_display_path,
        "<app-local-data>/logs/diagnostics/diagnostics-manifest.txt"
    );

    let manifest_path = log_dir.join("diagnostics").join("diagnostics-manifest.txt");
    assert!(manifest_path.exists());

    let manifest_contents =
        std::fs::read_to_string(&manifest_path).expect("read diagnostics manifest content");
    assert!(manifest_contents.contains("generated_at_unix="));
    assert!(manifest_contents.contains("log_dir=<app-local-data>/logs"));
    assert!(manifest_contents.contains("files_count="));
    assert!(manifest_contents.contains("gateway-export.log"));
    assert!(manifest_contents.contains("ck_local_[redacted]"));
    assert!(manifest_contents.contains("bearer [redacted]"));
    assert!(!manifest_contents.contains("ck_local_abc123xyz"));
    assert!(!manifest_contents.contains("Bearer demo-token"));
    assert!(!manifest_contents.contains("bearer demo-token"));

    let diagnostics_entries =
        std::fs::read_dir(log_dir.join("diagnostics")).expect("read diagnostics directory entries");
    for entry in diagnostics_entries {
        let entry = entry.expect("diagnostics directory entry");
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        assert!(!file_name.starts_with(".diagnostics-manifest.tmp-"));
    }

    let _ = std::fs::remove_dir_all(&isolated_root);
}

#[tokio::test]
async fn diagnostics_export_preserves_existing_target_on_replace_failure() {
    let isolated_root = isolated_test_root("diagnostics-export-failure");
    let database_path = isolated_root.join("state.sqlite3");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let log_dir = isolated_root.join("logs");
    let runtime = RuntimeState::new(
        app_state,
        RuntimeLogConfig {
            log_dir: log_dir.clone(),
        },
    );

    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    std::fs::write(log_dir.join("gateway-export.log"), "entry-export")
        .expect("write export log file");

    let diagnostics_dir = log_dir.join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).expect("create diagnostics dir");
    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    if manifest_path.exists() {
        if manifest_path.is_dir() {
            std::fs::remove_dir_all(&manifest_path)
                .expect("remove stale conflicting manifest directory");
        } else {
            std::fs::remove_file(&manifest_path).expect("remove stale manifest file");
        }
    }
    std::fs::create_dir_all(&manifest_path).expect("create conflicting manifest directory");

    let error = export_runtime_diagnostics_from_runtime(&runtime).expect_err("export should fail");
    let payload = error.to_payload();
    assert_eq!(payload.code, "config.unknown");
    assert_eq!(payload.category, ErrorCategory::ConfigError);
    assert_eq!(
        payload.message,
        "Failed to atomically replace diagnostics manifest."
    );
    assert!(payload
        .internal_context
        .unwrap_or_default()
        .contains("command=export_runtime_diagnostics;operation=rename_manifest"));
    assert!(manifest_path.is_dir());

    let diagnostics_entries =
        std::fs::read_dir(&diagnostics_dir).expect("read diagnostics directory entries");
    for entry in diagnostics_entries {
        let entry = entry.expect("diagnostics directory entry");
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        assert!(!file_name.starts_with(".diagnostics-manifest.tmp-"));
    }

    let _ = std::fs::remove_dir_all(&isolated_root);
}

fn isolated_test_root(prefix: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("system clock drift before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()))
}
