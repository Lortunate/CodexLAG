use std::path::{Path, PathBuf};

use codexlag_lib::bootstrap::{runtime_database_path, runtime_log_dir};

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
