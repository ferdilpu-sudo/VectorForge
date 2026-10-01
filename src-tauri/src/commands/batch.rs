use std::sync::Arc;

use tauri::{AppHandle, Manager, State};

use crate::batch::BatchItemWork;
use crate::models::{
    AppError, BatchHandle, BatchProgress, BatchRequest, DestinationKind, ErrorCode,
};
use crate::state::AppState;
use crate::store;

#[tauri::command]
pub async fn start_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    request: BatchRequest,
) -> Result<BatchHandle, AppError> {
    request.validate()?;

    let works = {
        let registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state("Registry file tidak dapat dikunci.", error.to_string())
        })?;
        let destination = registry.resolve_destination(&request.destination_id)?;
        if destination.kind != DestinationKind::Directory {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Batch membutuhkan tujuan folder.",
            ));
        }

        let mut works = Vec::with_capacity(request.file_ids.len());
        for file_id in &request.file_ids {
            let source = registry.resolve_source(file_id)?;
            let name = source
                .path
                .file_name()
                .and_then(|value| value.to_str())
                .ok_or_else(|| {
                    AppError::new(
                        ErrorCode::InvalidParams,
                        "Nama file sumber batch tidak valid.",
                    )
                })?
                .to_owned();

            works.push(BatchItemWork {
                file_id: file_id.clone(),
                name,
                source,
                params: request.params,
                formats: request.formats.clone(),
                output_dir: destination.path.clone(),
                output_stem: String::new(),
                overwrite: request.overwrite,
            });
        }
        works
    };

    let app_data_dir = app.path().app_data_dir().map_err(|error| {
        AppError::invalid_state(
            "Folder data aplikasi tidak dapat diresolve.",
            error.to_string(),
        )
    })?;
    let storage = Arc::clone(&state.storage);
    let worker_count = tauri::async_runtime::spawn_blocking(move || {
        let _guard = storage.lock().map_err(|error| {
            AppError::invalid_state("Storage lock tidak dapat diperoleh.", error.to_string())
        })?;
        store::get_settings(&app_data_dir).map(|settings| usize::from(settings.worker_count))
    })
    .await
    .map_err(|error| {
        AppError::invalid_state("Pengaturan worker gagal dibaca.", error.to_string())
    })??;

    state.heavy.set_limit(worker_count)?;
    state.batch.start(app, works, worker_count)
}

#[tauri::command]
pub fn get_batch(state: State<'_, AppState>, batch_id: String) -> Result<BatchProgress, AppError> {
    state.batch.get(&batch_id)
}

#[tauri::command]
pub fn cancel_batch(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    run_id: String,
) -> Result<(), AppError> {
    state.batch.cancel(&app, &batch_id, &run_id)
}

#[tauri::command]
pub fn retry_batch_item(
    app: AppHandle,
    state: State<'_, AppState>,
    batch_id: String,
    item_id: String,
) -> Result<BatchHandle, AppError> {
    state.batch.retry_item(app, &batch_id, &item_id)
}
