mod commands;
mod exporter;
mod models;
mod realm_parser;
mod services;
mod state;

/**
 * 初始化 Tauri 应用、Rust 状态和所有前端可调用命令。
 */
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(state::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::load_database,
            commands::clear_database,
            commands::get_collection_summaries,
            commands::query_beatmaps,
            commands::load_settings,
            commands::save_settings,
            commands::get_default_columns,
            commands::get_cover,
            commands::export_current,
            commands::export_all_modes,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
