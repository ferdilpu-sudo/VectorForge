mod batch;
mod commands;
mod engine;
mod export;
mod files;
mod models;
mod source_protocol;
mod state;
mod store;

use commands::{
    cancel_batch, cancel_preview, choose_destination, delete_preset, export_file, generate_preview,
    get_batch, get_settings, import_files, list_presets, release_files, retry_batch_item,
    save_preset, save_settings, start_batch,
};
use state::AppState;

pub fn run() {
    let result = tauri::Builder::default()
        .register_uri_scheme_protocol(source_protocol::SOURCE_PROTOCOL, |context, request| {
            source_protocol::handle(context, request)
        })
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
            save_settings,
            start_batch,
            get_batch,
            cancel_batch,
            retry_batch_item
        ])
        .run(tauri::generate_context!());

    if let Err(error) = result {
        eprintln!("VectorForge failed to start: {error}");
        std::process::exit(1);
    }
}
