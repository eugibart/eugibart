// Prevents an extra console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;

fn main() {
    tauri::Builder::default()
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::list_definitions,
            commands::list_serial_ports,
            commands::connect,
            commands::disconnect,
            commands::connection_status,
            commands::read_dtcs,
            commands::clear_dtcs,
            commands::poll_live_data,
            commands::enable_service_mode,
            commands::run_routine,
            commands::start_csv_log,
            commands::stop_csv_log,
            commands::troubleshoot_connection,
            commands::export_health_report,
            commands::connect_vacuum,
            commands::vacuum_status,
            commands::poll_vacuum,
            commands::disconnect_vacuum,
        ])
        .run(tauri::generate_context!())
        .expect("error while running MotoDiag");
}
