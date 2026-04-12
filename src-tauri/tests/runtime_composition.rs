use codexlag_lib::{
    bootstrap::{bootstrap_runtime_for_test, runtime_database_path},
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

    let metadata = runtime_log_metadata_from_runtime(&runtime).expect("runtime log metadata");

    assert_eq!(metadata.log_dir, "<app-local-data>/logs");
    assert_ne!(metadata.log_dir, log_dir.to_string_lossy());
    assert!(metadata.files.iter().all(|file_name| file_name.ends_with(".log")));
    assert!(metadata.files.len() <= 20);
}

#[tokio::test]
async fn diagnostics_export_returns_manifest_path() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");

    let log_dir = runtime.runtime_log().log_dir.clone();
    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    std::fs::write(log_dir.join("gateway-export.log"), "entry-export").expect("write export log file");

    let manifest_path =
        export_runtime_diagnostics_from_runtime(&runtime).expect("export runtime diagnostics");
    let manifest_path = std::path::PathBuf::from(manifest_path);

    assert_eq!(manifest_path.file_name().and_then(|name| name.to_str()), Some("diagnostics-manifest.txt"));
    assert!(manifest_path.ends_with("diagnostics/diagnostics-manifest.txt"));
    assert!(manifest_path.exists());

    let manifest_contents =
        std::fs::read_to_string(&manifest_path).expect("read diagnostics manifest content");
    assert!(manifest_contents.contains("generated_at_unix="));
    assert!(manifest_contents.contains("log_dir=<app-local-data>/logs"));
    assert!(manifest_contents.contains("files_count="));
    assert!(manifest_contents.contains("- gateway-export.log"));
}
