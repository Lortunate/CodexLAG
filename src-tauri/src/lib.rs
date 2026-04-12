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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| -> Result<(), Box<dyn Error>> {
            let app_local_data_dir = app
                .path()
                .app_local_data_dir()
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
            let database_path = bootstrap::runtime_database_path(app_local_data_dir);

            let runtime = bootstrap::bootstrap_runtime_at(database_path)
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;

            tray::install_runtime_tray(app, &runtime.tray_model())
                .map_err(|error| -> Box<dyn Error> { Box::new(error) })?;
            app.manage(runtime);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::accounts::refresh_account_balance,
            commands::accounts::get_account_capability_detail,
            commands::relays::list_relays,
            commands::relays::refresh_relay_balance,
            commands::relays::get_relay_capability_detail,
            commands::keys::get_default_key_summary,
            commands::keys::set_default_key_mode,
            commands::policies::list_policies,
            commands::logs::get_log_summary,
            commands::logs::get_usage_request_detail,
            commands::logs::list_usage_request_history,
            commands::logs::query_usage_ledger
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
