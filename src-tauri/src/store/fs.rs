use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::models::{AppError, ErrorCode};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
unsafe extern "system" {
    fn ReplaceFileW(
        replaced_file_name: *const u16,
        replacement_file_name: *const u16,
        backup_file_name: *const u16,
        replace_flags: u32,
        exclude: *mut core::ffi::c_void,
        reserved: *mut core::ffi::c_void,
    ) -> i32;
}

pub fn read_versioned<T: DeserializeOwned>(
    path: &Path,
    expected_version: u32,
    label: &str,
) -> Result<Option<T>, AppError> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(read_error(label, error)),
    };

    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| corrupt_error(label, error.to_string()))?;
    let version = value
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| corrupt_error(label, "field version tidak valid"))?;

    if version != u64::from(expected_version) {
        return Err(AppError::with_details(
            ErrorCode::DataVersionUnsupported,
            format!("Versi {label} tidak didukung."),
            format!("expected {expected_version}, found {version}"),
        ));
    }

    let parsed = serde_json::from_value(value)
        .map_err(|error| corrupt_error(label, error.to_string()))?;
    Ok(Some(parsed))
}

pub fn write_json_atomic<T: Serialize>(
    path: &Path,
    value: &T,
    label: &str,
) -> Result<(), AppError> {
    let parent = path.parent().ok_or_else(|| {
        AppError::new(
            ErrorCode::WriteFailed,
            format!("Folder {label} tidak valid."),
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| write_error(&format!("Folder {label} gagal dibuat."), error))?;

    let bytes = serde_json::to_vec_pretty(value).map_err(|error| {
        AppError::invalid_state(
            format!("{label} gagal diserialisasi."),
            error.to_string(),
        )
    })?;

    let (mut temp, temp_path) = create_temp(parent, label)?;
    let write_result = temp
        .write_all(&bytes)
        .and_then(|_| temp.write_all(b"\n"))
        .and_then(|_| temp.sync_all());
    drop(temp);

    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(write_error(&format!("{label} gagal ditulis."), error));
    }

    let commit_result = if path.exists() {
        replace_existing_file(path, &temp_path)
    } else {
        fs::rename(&temp_path, path)
    };

    if let Err(error) = commit_result {
        let _ = fs::remove_file(&temp_path);
        return Err(write_error(&format!("{label} gagal dikomit."), error));
    }

    Ok(())
}

fn create_temp(parent: &Path, label: &str) -> Result<(File, PathBuf), AppError> {
    for _ in 0..32 {
        let path = parent.join(format!(".vectorforge-store-{}.tmp", Uuid::new_v4()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(write_error(
                    &format!("Temporary {label} gagal dibuat."),
                    error,
                ));
            }
        }
    }

    Err(AppError::new(
        ErrorCode::WriteFailed,
        format!("Temporary {label} tidak dapat direservasi."),
    ))
}

#[cfg(windows)]
fn wide_path(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn replace_existing_file(target: &Path, replacement: &Path) -> io::Result<()> {
    let target = wide_path(target);
    let replacement = wide_path(replacement);
    let result = unsafe {
        ReplaceFileW(
            target.as_ptr(),
            replacement.as_ptr(),
            std::ptr::null(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };

    if result == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_existing_file(target: &Path, replacement: &Path) -> io::Result<()> {
    fs::rename(replacement, target)
}

fn read_error(label: &str, error: io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => AppError::with_details(
            ErrorCode::AccessDenied,
            format!("{label} tidak dapat dibaca."),
            error.to_string(),
        ),
        _ => AppError::with_details(
            ErrorCode::DataCorrupt,
            format!("{label} gagal dibaca."),
            error.to_string(),
        ),
    }
}

fn write_error(message: &str, error: io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => {
            AppError::with_details(ErrorCode::AccessDenied, message, error.to_string())
        }
        _ => AppError::with_details(ErrorCode::WriteFailed, message, error.to_string()),
    }
}

fn corrupt_error(label: &str, details: impl Into<String>) -> AppError {
    AppError::with_details(
        ErrorCode::DataCorrupt,
        format!("{label} rusak atau tidak valid."),
        details,
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use serde::{Deserialize, Serialize};
    use uuid::Uuid;

    use super::{read_versioned, write_json_atomic};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Fixture {
        version: u32,
        value: String,
    }

    struct TempDir(PathBuf);

    impl TempDir {
        fn create() -> Result<Self, String> {
            let path =
                std::env::temp_dir().join(format!("vectorforge-b05-store-{}", Uuid::new_v4()));
            fs::create_dir(&path).map_err(|error| error.to_string())?;
            Ok(Self(path))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn atomic_json_round_trip_replaces_existing_file() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = dir.0.join("fixture.json");
        write_json_atomic(
            &path,
            &Fixture {
                version: 1,
                value: "first".to_owned(),
            },
            "fixture",
        )
        .map_err(|error| error.message)?;
        write_json_atomic(
            &path,
            &Fixture {
                version: 1,
                value: "second".to_owned(),
            },
            "fixture",
        )
        .map_err(|error| error.message)?;

        let loaded: Fixture = read_versioned(&path, 1, "fixture")
            .map_err(|error| error.message)?
            .ok_or_else(|| "fixture disappeared".to_owned())?;
        assert_eq!(loaded.value, "second");
        Ok(())
    }
}
