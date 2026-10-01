use std::sync::Arc;

use tauri::State;

use crate::export::{ExportWork, export_work};
use crate::models::{AppError, ExportRequest, ExportResult};
use crate::state::AppState;

#[tauri::command]
pub async fn export_file(
    state: State<'_, AppState>,
    request: ExportRequest,
) -> Result<ExportResult, AppError> {
    request.validate()?;

    let (source, destination) = {
        let registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state("Registry file tidak dapat dikunci.", error.to_string())
        })?;
        (
            registry.resolve_source(&request.file_id)?,
            registry.resolve_destination(&request.destination_id)?,
        )
    };

    let heavy = Arc::clone(&state.heavy);
    let committed = tauri::async_runtime::spawn_blocking(move || {
        let _permit = heavy.acquire_normal()?;
        export_work(ExportWork {
            source,
            destination,
            request,
        })
    })
    .await
    .map_err(|error| {
        AppError::invalid_state("Proses export internal gagal.", error.to_string())
    })??;

    let output_id = {
        let mut registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state("Registry output tidak dapat dikunci.", error.to_string())
        })?;
        registry.register_output(committed.path.clone())?
    };

    Ok(ExportResult {
        output_id,
        out_path: committed.path.to_string_lossy().into_owned(),
        format: committed.format,
        bytes: committed.bytes,
        stats: committed.stats,
        elapsed_ms: committed.elapsed_ms,
    })
}
