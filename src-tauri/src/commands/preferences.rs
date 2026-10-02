use std::path::Path;

use tauri::{AppHandle, Manager};

use crate::models::{AppError, AppSettings, Preset, SavePresetRequest};
use crate::state::AppState;
use crate::store;

#[tauri::command]
pub async fn list_presets(app: AppHandle) -> Result<Vec<Preset>, AppError> {
    run_storage(app, store::list_presets).await
}

#[tauri::command]
pub async fn save_preset(app: AppHandle, request: SavePresetRequest) -> Result<Preset, AppError> {
    run_storage(app, move |dir| store::save_preset(dir, request)).await
}

#[tauri::command]
pub async fn delete_preset(app: AppHandle, id: String) -> Result<(), AppError> {
    run_storage(app, move |dir| store::delete_preset(dir, &id)).await
}

#[tauri::command]
pub async fn get_settings(app: AppHandle) -> Result<AppSettings, AppError> {
    let state_app = app.clone();
    let settings = run_storage(app, store::get_settings).await?;
    state_app
        .state::<AppState>()
        .heavy
        .set_limit(usize::from(settings.worker_count))?;
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, AppError> {
    let state_app = app.clone();
    let settings = run_storage(app, move |dir| {
        let mut incoming = settings;
        incoming.last_out_dir = store::get_settings(dir)?.last_out_dir;
        store::save_settings(dir, incoming)
    })
    .await?;
    state_app
        .state::<AppState>()
        .heavy
        .set_limit(usize::from(settings.worker_count))?;
    Ok(settings)
}

pub(crate) async fn run_storage<T, F>(app: AppHandle, task: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&Path) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let app_data_dir = app.path().app_data_dir().map_err(|error| {
            AppError::invalid_state(
                "Folder data aplikasi tidak dapat diresolve.",
                error.to_string(),
            )
        })?;

        let state = app.state::<AppState>();
        let _guard = state.storage.lock().map_err(|error| {
            AppError::invalid_state("Storage lock tidak dapat diperoleh.", error.to_string())
        })?;

        task(&app_data_dir)
    })
    .await
    .map_err(|error| AppError::invalid_state("Proses storage internal gagal.", error.to_string()))?
}
