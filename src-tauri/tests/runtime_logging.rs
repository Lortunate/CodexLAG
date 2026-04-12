use std::path::{Path, PathBuf};

use codexlag_lib::bootstrap::{bootstrap_runtime_for_test, runtime_database_path, runtime_log_dir};

#[test]
fn runtime_log_dir_uses_app_local_data_logs_subfolder() {
    let app_local_data_dir = Path::new("/tmp/codexlag-app");

    assert_eq!(
        runtime_log_dir(app_local_data_dir),
        PathBuf::from("/tmp/codexlag-app").join("logs")
    );
    assert_eq!(
        runtime_database_path(app_local_data_dir),
        PathBuf::from("/tmp/codexlag-app").join("codexlag.sqlite3")
    );
}

#[tokio::test]
async fn bootstrapped_runtime_exposes_runtime_log_dir_metadata() {
    let runtime = bootstrap_runtime_for_test()
        .await
        .expect("bootstrap runtime");
    let log_dir = &runtime.runtime_log().log_dir;

    assert!(!log_dir.as_os_str().is_empty());
    assert!(log_dir.ends_with("logs"));
}
