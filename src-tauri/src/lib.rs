pub mod bootstrap;
pub mod db;
pub mod error;
pub mod gateway;
pub mod models;
pub mod providers;
pub mod routing;
pub mod secret_store;
pub mod state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .run(tauri::generate_context!())
        .expect("error while running CodexLAG");
}
