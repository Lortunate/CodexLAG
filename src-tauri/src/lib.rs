pub mod bootstrap;
pub mod commands;
pub mod db;
pub mod error;
pub mod gateway;
pub mod logging;
pub mod models;
pub mod providers;
pub mod routing;
pub mod secret_store;
pub mod state;
pub mod tray;

use std::error::Error;

use tauri::Manager;
use tauri_plugin_log::{RotationStrategy, Target, TargetKind, TimezoneStrategy};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir {
                        file_name: Some("gateway".to_string()),
                    }),
                ])
                .max_file_size(10_000_000)
                .rotation_strategy(RotationStrategy::KeepSome(10))
                .timezone_strategy(TimezoneStrategy::UseLocal)
                .build(),
        )
        .setup(|app| -> Result<(), Box<dyn Error>> {
            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
            let app_log_dir = app
                .path()
                .app_log_dir()
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
            let database_path = bootstrap::runtime_database_path(app_local_data_dir);

            let runtime = bootstrap::bootstrap_runtime_at_with_log_dir(database_path, app_log_dir)
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;

            tray::install_runtime_tray(app, &runtime.tray_model())
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
            app.manage(runtime);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::accounts::import_official_account_login,
            commands::accounts::refresh_account_balance,
            commands::accounts::get_account_capability_detail,
            commands::relays::list_relays,
            commands::relays::add_relay,
            commands::relays::update_relay,
            commands::relays::delete_relay,
            commands::relays::test_relay_connection,
            commands::relays::refresh_relay_balance,
            commands::relays::get_relay_capability_detail,
            commands::keys::get_default_key_summary,
            commands::keys::set_default_key_mode,
            commands::keys::create_platform_key,
            commands::keys::list_platform_keys,
            commands::keys::disable_platform_key,
            commands::keys::enable_platform_key,
            commands::policies::list_policies,
            commands::policies::update_policy,
            commands::logs::get_log_summary,
            commands::logs::get_runtime_log_metadata,
            commands::logs::export_runtime_diagnostics,
            commands::logs::get_usage_request_detail,
            commands::logs::list_usage_request_history,
            commands::logs::query_usage_ledger
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
