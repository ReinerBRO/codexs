mod auth;
mod commands;
mod models;
mod usage;
mod utils;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(commands::GenerationControl::default())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::start_generation,
            commands::stop_generation,
            commands::get_accounts,
            commands::import_accounts,
            commands::list_accounts,
            commands::add_account,
            commands::delete_account,
            commands::switch_account,
            commands::refresh_account_usage,
            commands::get_current_account,
            commands::list_ssh_servers,
            commands::add_ssh_server,
            commands::delete_ssh_server,
            commands::get_ssh_auto_sync,
            commands::set_ssh_auto_sync,
            commands::sync_auth_to_ssh,
            commands::parse_ssh_config,
            commands::find_codex_sessions,
            commands::terminate_and_resume_sessions
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
