mod commands;
mod engine;
mod export;
mod files;
mod models;
mod state;
mod store;

use commands::{
    cancel_preview, choose_destination, delete_preset, export_file, generate_preview, get_settings,
    import_files, list_presets, release_files, save_preset, save_settings,
};
use state::AppState;

pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            import_files,
            release_files,
            choose_destination,
            generate_preview,
            cancel_preview,
            export_file,
            list_presets,
            save_preset,
            delete_preset,
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("VectorForge failed to start: {error}");
        std::process::exit(1);
    }
}
