use std::fs;
use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager, State, WebviewWindow};
use tauri_plugin_dialog::{DialogExt, FilePath};

use crate::files::probe_source;
use crate::models::{
    AppError, Destination, DestinationKind, DestinationRequest, ErrorCode, ImportRejection,
    ImportRequest, ImportResult, SourceFile,
};
use crate::state::AppState;

#[tauri::command]
pub async fn import_files(
    app: AppHandle,
    window: WebviewWindow,
    request: ImportRequest,
) -> Result<ImportResult, AppError> {
    let app_for_task = app.clone();
    let window_for_task = window.clone();

    tauri::async_runtime::spawn_blocking(move || {
        import_files_blocking(&app_for_task, &window_for_task, request)
    })
    .await
    .map_err(|error| {
        AppError::invalid_state(
            "Proses import internal gagal.",
            error.to_string(),
        )
    })
}

fn import_files_blocking(
    app: &AppHandle,
    window: &WebviewWindow,
    request: ImportRequest,
) -> ImportResult {
    let mut files = Vec::new();
    let mut rejected = Vec::new();

    for raw_path in request.paths {
        let name = rejection_name(&raw_path);
        match import_one(app, window, &raw_path) {
            Ok(source) => files.push(source),
            Err(error) => rejected.push(ImportRejection { name, error }),
        }
    }

    ImportResult { files, rejected }
}

fn import_one(
    app: &AppHandle,
    window: &WebviewWindow,
    raw_path: &str,
) -> Result<SourceFile, AppError> {
    let requested = PathBuf::from(raw_path);
    let scope = app.asset_protocol_scope();

    if !scope.is_allowed(&requested) {
        return Err(AppError::new(
            ErrorCode::AccessDenied,
            "File belum diotorisasi oleh dialog atau drag-and-drop native.",
        ));
    }

    let canonical = fs::canonicalize(&requested).map_err(source_path_error)?;
    if !scope.is_allowed(&canonical) {
        return Err(AppError::new(
            ErrorCode::AccessDenied,
            "Akses file tidak termasuk dalam scope aplikasi.",
        ));
    }

    let probe = probe_source(&canonical)?;
    let name = canonical
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            AppError::new(
                ErrorCode::InvalidParams,
                "Nama file tidak dapat direpresentasikan sebagai teks.",
            )
        })?
        .to_owned();

    let preview_url = window
        .convert_file_src(&canonical, None)
        .map_err(|error| {
            AppError::invalid_state(
                "URL preview lokal gagal dibuat.",
                error.to_string(),
            )
        })?
        .to_string();

    let state = app.state::<AppState>();
    let id = {
        let mut registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state(
                "Registry file tidak dapat dikunci.",
                error.to_string(),
            )
        })?;
        registry.register_source(canonical, probe.fingerprint.clone())
    };

    Ok(SourceFile {
        id,
        name,
        width: probe.width,
        height: probe.height,
        bytes: probe.bytes,
        format: probe.format,
        has_alpha: probe.has_alpha,
        fingerprint: probe.fingerprint,
        preview_url,
    })
}

#[tauri::command]
pub fn release_files(
    state: State<'_, AppState>,
    file_ids: Vec<String>,
) -> Result<(), AppError> {
    let mut registry = state.registry.lock().map_err(|error| {
        AppError::invalid_state(
            "Registry file tidak dapat dikunci.",
            error.to_string(),
        )
    })?;

    registry.release_sources(&file_ids)
}

#[tauri::command]
pub async fn choose_destination(
    window: WebviewWindow,
    state: State<'_, AppState>,
    request: DestinationRequest,
) -> Result<Option<Destination>, AppError> {
    request.validate()?;

    let kind = request.kind;
    let format = request.format;
    let suggested_name = request.suggested_name.clone();
    let dialog = window.dialog().file();

    let selected = tauri::async_runtime::spawn_blocking(move || match kind {
        DestinationKind::Directory => dialog
            .set_title("Pilih folder output")
            .blocking_pick_folder(),
        DestinationKind::File => {
            let selected_format = match format {
                Some(value) => value,
                None => return None,
            };
            let mut builder = dialog
                .set_title("Simpan hasil VectorForge")
                .add_filter(selected_format.filter_name(), &[selected_format.extension()]);
            if let Some(name) = suggested_name {
                builder = builder.set_file_name(name);
            }
            builder.blocking_save_file()
        }
    })
    .await
    .map_err(|error| {
        AppError::invalid_state(
            "Dialog tujuan gagal dijalankan.",
            error.to_string(),
        )
    })?;

    let Some(file_path) = selected else {
        return Ok(None);
    };
    let selected_path = file_path_to_path(file_path)?;
    let normalized = normalize_destination(&selected_path, kind)?;
    let overwrite_confirmed = kind == DestinationKind::File && normalized.exists();
    let display_path = normalized.to_string_lossy().into_owned();

    let id = {
        let mut registry = state.registry.lock().map_err(|error| {
            AppError::invalid_state(
                "Registry tujuan tidak dapat dikunci.",
                error.to_string(),
            )
        })?;
        registry.register_destination(
            normalized,
            kind,
            format,
            overwrite_confirmed,
        )
    };

    Ok(Some(Destination {
        id,
        display_path,
        kind,
    }))
}

fn file_path_to_path(file_path: FilePath) -> Result<PathBuf, AppError> {
    file_path.into_path().map_err(|error| {
        AppError::invalid_state(
            "Path dari dialog native tidak valid.",
            error.to_string(),
        )
    })
}

fn normalize_destination(path: &Path, kind: DestinationKind) -> Result<PathBuf, AppError> {
    match kind {
        DestinationKind::Directory => {
            let canonical = fs::canonicalize(path).map_err(destination_io_error)?;
            if canonical.is_dir() {
                Ok(canonical)
            } else {
                Err(AppError::new(
                    ErrorCode::InvalidParams,
                    "Tujuan yang dipilih bukan folder.",
                ))
            }
        }
        DestinationKind::File => {
            let parent = path.parent().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidParams,
                    "Folder tujuan file tidak valid.",
                )
            })?;
            let canonical_parent = fs::canonicalize(parent).map_err(destination_io_error)?;
            if !canonical_parent.is_dir() {
                return Err(AppError::new(
                    ErrorCode::InvalidParams,
                    "Folder tujuan file tidak valid.",
                ));
            }
            let file_name = path.file_name().ok_or_else(|| {
                AppError::new(
                    ErrorCode::InvalidParams,
                    "Nama file tujuan tidak valid.",
                )
            })?;
            Ok(canonical_parent.join(file_name))
        }
    }
}

fn rejection_name(raw_path: &str) -> String {
    Path::new(raw_path)
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("File")
        .to_owned()
}

fn source_path_error(error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            AppError::new(ErrorCode::FileNotFound, "File sumber tidak ditemukan.")
        }
        std::io::ErrorKind::PermissionDenied => {
            AppError::new(ErrorCode::AccessDenied, "Akses file sumber ditolak.")
        }
        _ => AppError::with_details(
            ErrorCode::ImageCorrupt,
            "Path sumber gagal diverifikasi.",
            error.to_string(),
        ),
    }
}

fn destination_io_error(error: std::io::Error) -> AppError {
    match error.kind() {
        std::io::ErrorKind::NotFound => {
            AppError::new(ErrorCode::FileNotFound, "Folder tujuan tidak ditemukan.")
        }
        std::io::ErrorKind::PermissionDenied => {
            AppError::new(ErrorCode::AccessDenied, "Akses folder tujuan ditolak.")
        }
        _ => AppError::with_details(
            ErrorCode::WriteFailed,
            "Folder tujuan gagal diverifikasi.",
            error.to_string(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::models::DestinationKind;

    use super::{normalize_destination, rejection_name};

    #[test]
    fn rejection_name_does_not_echo_full_path() {
        assert_eq!(
            rejection_name(r"C:\Users\Someone\secret\photo.png"),
            "photo.png"
        );
    }

    #[test]
    fn missing_destination_directory_is_rejected() {
        let path = PathBuf::from("__vectorforge_missing_directory__");
        assert!(normalize_destination(&path, DestinationKind::Directory).is_err());
    }
}
