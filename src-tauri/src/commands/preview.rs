use tauri::State;

use crate::engine::PreviewWork;
use crate::models::{AppError, PreviewRequest, PreviewResult, validate_request_id};
use crate::state::AppState;

#[tauri::command]
pub async fn generate_preview(
    state: State<'_, AppState>,
    request: PreviewRequest,
) -> Result<PreviewResult, AppError> {
    request.validate()?;

    let source = {
        let registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state("Registry file tidak dapat dikunci.", error.to_string())
        })?;
        registry.resolve_source(&request.file_id)?
    };

    let receiver = state.preview.submit(PreviewWork { source, request })?;
    tauri::async_runtime::spawn_blocking(move || {
        receiver.recv().map_err(|error| {
            AppError::invalid_state(
                "Worker preview berhenti sebelum memberi hasil.",
                error.to_string(),
            )
        })
    })
    .await
    .map_err(|error| {
        AppError::invalid_state("Penunggu preview internal gagal.", error.to_string())
    })??
}

#[tauri::command]
pub fn cancel_preview(state: State<'_, AppState>, request_id: String) -> Result<(), AppError> {
    validate_request_id(&request_id)?;
    state.preview.cancel(&request_id)
}

#[cfg(test)]
mod tests {
    use crate::models::validate_request_id;

    #[test]
    fn cancel_request_id_requires_uuid_shape() {
        assert!(validate_request_id("not-a-uuid").is_err());
        assert!(validate_request_id("00000000-0000-4000-8000-000000000002").is_ok());
    }
}
