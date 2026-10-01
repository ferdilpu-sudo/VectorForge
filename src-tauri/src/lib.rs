mod commands;
mod files;
mod models;
mod state;

use commands::{choose_destination, import_files, release_files};
use state::AppState;

pub fn run() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            import_files,
            release_files,
            choose_destination
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("VectorForge failed to start: {error}");
        std::process::exit(1);
    }
}
