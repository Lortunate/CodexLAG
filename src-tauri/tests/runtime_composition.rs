use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, bootstrap_state_for_test_at, runtime_database_path},
    commands::{
        keys::{default_key_summary_from_state, set_default_key_mode_from_runtime},
        logs::{
            export_runtime_diagnostics_from_runtime, log_summary_from_runtime,
            runtime_log_metadata_from_runtime,
        },
        policies::policy_summaries_from_state,
    },
    routing::policy::{RoutingMode, HYBRID, RELAY_ONLY},
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
};

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
}

#[tokio::test]
async fn set_default_key_mode_rejects_invalid_mode_strings() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let error = set_default_key_mode_from_runtime(&runtime, "invalid-mode")
        .expect_err("invalid mode should fail");

    assert!(error.to_string().contains("unsupported default key mode"));
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
    assert!(metadata.files.iter().all(|file_name| !file_name.ends_with(".txt")));
    assert!(!metadata.files.iter().any(|file_name| file_name == "notes.txt"));
    assert!(!metadata.files.iter().any(|file_name| file_name == "gateway.backup"));
    assert!(!metadata.files.iter().any(|file_name| file_name == "gateway-snapshot"));
    assert!(
        metadata
            .files
            .iter()
            .all(|file_name| file_name.ends_with(".log") || file_name.contains(".log."))
    );
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
    std::fs::write(log_dir.join("gateway-export.log"), "entry-export").expect("write export log file");

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
    assert!(manifest_contents.contains("files=[\"gateway-export.log\"]"));
    assert!(!manifest_contents.contains("ck_local_"));
    assert!(!manifest_contents.contains("bearer "));

    let diagnostics_entries = std::fs::read_dir(log_dir.join("diagnostics"))
        .expect("read diagnostics directory entries");
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
    std::fs::write(log_dir.join("gateway-export.log"), "entry-export").expect("write export log file");

    let diagnostics_dir = log_dir.join("diagnostics");
    std::fs::create_dir_all(&diagnostics_dir).expect("create diagnostics dir");
    let manifest_path = diagnostics_dir.join("diagnostics-manifest.txt");
    if manifest_path.exists() {
        if manifest_path.is_dir() {
            std::fs::remove_dir_all(&manifest_path).expect("remove stale conflicting manifest directory");
        } else {
            std::fs::remove_file(&manifest_path).expect("remove stale manifest file");
        }
    }
    std::fs::create_dir_all(&manifest_path).expect("create conflicting manifest directory");

    let error = export_runtime_diagnostics_from_runtime(&runtime).expect_err("export should fail");
    assert!(error.contains("failed to atomically replace diagnostics manifest"));
    assert!(manifest_path.is_dir());

    let diagnostics_entries = std::fs::read_dir(&diagnostics_dir).expect("read diagnostics directory entries");
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
