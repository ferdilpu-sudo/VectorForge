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
        let current = store::get_settings(dir)?;
        store::save_settings(dir, preserve_native_settings(settings, &current))
    })
    .await?;
    state_app
        .state::<AppState>()
        .heavy
        .set_limit(usize::from(settings.worker_count))?;
    Ok(settings)
}

pub(crate) fn preserve_native_settings(mut incoming: AppSettings, current: &AppSettings) -> AppSettings {
    incoming.last_out_dir = current.last_out_dir.clone();
    incoming
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

#[cfg(test)]
mod tests {
    use crate::models::AppSettings;
    use crate::models::preferences::Theme;

    use super::preserve_native_settings;

    #[test]
    fn frontend_settings_save_preserves_native_last_output_directory() {
        let mut current = AppSettings::defaults();
        current.last_out_dir = Some(r"C:\Users\Tester\Documents\VectorForge".to_owned());

        let mut incoming = AppSettings::defaults();
        incoming.theme = Theme::Light;
        incoming.last_out_dir = None;

        let merged = preserve_native_settings(incoming, &current);
        assert_eq!(merged.theme, Theme::Light);
        assert_eq!(merged.last_out_dir, current.last_out_dir);
    }
}
