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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::relays::list_relays,
            commands::keys::get_default_key_summary,
            commands::policies::list_policies,
            commands::logs::get_log_summary
        ])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
