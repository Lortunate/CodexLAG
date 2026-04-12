use codexlag_lib::{
    bootstrap::bootstrap_state_for_test_at,
    commands::logs::export_runtime_diagnostics_from_runtime,
    secret_store::SecretKey,
    state::{RuntimeLogConfig, RuntimeState},
};

#[tokio::test]
async fn diagnostics_and_db_artifacts_never_contain_plain_platform_secret() {
    let isolated_root = isolated_test_root("security-regression");
    let database_path = isolated_root.join("state.sqlite3");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let platform_secret = app_state
        .secret(&SecretKey::default_platform_key())
        .expect("default key secret");
    let log_dir = isolated_root.join("logs");
    let runtime = RuntimeState::new(
        app_state,
        RuntimeLogConfig {
            log_dir: log_dir.clone(),
        },
    );

    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    std::fs::write(log_dir.join("gateway-security.log"), "regular-entry")
        .expect("write runtime log file");

    let _ = export_runtime_diagnostics_from_runtime(&runtime).expect("export diagnostics");
    let manifest_path = log_dir.join("diagnostics").join("diagnostics-manifest.txt");
    let manifest_content = std::fs::read_to_string(&manifest_path).expect("read diagnostics manifest");
    let database_bytes = std::fs::read(&database_path).expect("read sqlite file");
    let database_text = String::from_utf8_lossy(&database_bytes);

    assert!(
        !manifest_content.contains(platform_secret.as_str()),
        "diagnostics manifest must not contain plain platform secret"
    );
    assert!(
        !database_text.contains(platform_secret.as_str()),
        "sqlite artifact must not contain plain platform secret"
    );

    let _ = std::fs::remove_dir_all(&isolated_root);
}

fn isolated_test_root(prefix: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("system clock drift before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()))
}
