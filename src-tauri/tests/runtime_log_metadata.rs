use codexlag_lib::{
    bootstrap::bootstrap_state_for_test_at,
    commands::logs::runtime_log_metadata_from_runtime,
    state::{RuntimeLogConfig, RuntimeState},
};

#[tokio::test]
async fn runtime_log_metadata_returns_bounded_recent_file_entries() {
    let isolated_root = isolated_test_root("runtime-log-metadata");
    std::fs::create_dir_all(&isolated_root).expect("create isolated root");
    let database_path = isolated_root.join("state.sqlite3");
    let log_dir = isolated_root.join("logs");
    let app_state = bootstrap_state_for_test_at(&database_path)
        .await
        .expect("bootstrap isolated app state");
    let runtime = RuntimeState::new(
        app_state,
        RuntimeLogConfig {
            log_dir: log_dir.clone(),
        },
    );

    std::fs::create_dir_all(&log_dir).expect("create runtime log directory");
    for index in 0..30 {
        let file_name = format!("gateway-{index:02}.log");
        std::fs::write(log_dir.join(file_name), format!("entry-{index}"))
            .expect("write runtime log file");
    }
    std::fs::write(log_dir.join("ignored.txt"), "ignore").expect("write non-log file");

    let metadata = runtime_log_metadata_from_runtime(&runtime).expect("runtime log metadata");

    assert_eq!(metadata.log_dir, "<app-local-data>/logs");
    assert_eq!(
        metadata.files.len(),
        20,
        "metadata must be bounded to 20 files"
    );
    assert!(metadata.files.iter().all(|file| !file.name.is_empty()));
    assert!(metadata.files.iter().all(|file| !file.path.is_empty()));
    assert!(metadata.files.iter().all(|file| file.size > 0));
    assert!(metadata.files.iter().all(|file| file.mtime > 0));
    assert!(
        metadata
            .files
            .iter()
            .all(|file| file.path.starts_with("<app-local-data>/logs/")),
        "paths should be sanitized and rooted under <app-local-data>/logs"
    );
    assert!(
        metadata
            .files
            .iter()
            .all(|file| file.path.ends_with(file.name.as_str())),
        "display path should contain the file name"
    );
    assert!(metadata
        .files
        .iter()
        .all(|file| file.name.ends_with(".log")));

    let _ = std::fs::remove_dir_all(&isolated_root);
}

fn isolated_test_root(prefix: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .expect("system clock drift before unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{prefix}-{}-{now}", std::process::id()))
}
