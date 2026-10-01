use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::models::{AppError, ErrorCode};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

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

pub fn write_atomic(
    target: &Path,
    bytes: &[u8],
    overwrite_confirmed: bool,
) -> Result<u64, AppError> {
    let parent = target.parent().ok_or_else(|| {
        AppError::new(ErrorCode::WriteFailed, "Folder output tidak valid.")
    })?;
    if !parent.is_dir() {
        return Err(AppError::new(
            ErrorCode::WriteFailed,
            "Folder output tidak tersedia.",
        ));
    }

    let (mut temp, temp_path) = create_temp(parent)?;
    let write_result = write_temp(&mut temp, bytes);
    drop(temp);
    if let Err(error) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(error);
    }

    let commit_result = commit_temp(target, &temp_path, overwrite_confirmed);
    if commit_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    commit_result?;

    u64::try_from(bytes.len()).map_err(|error| {
        AppError::invalid_state("Ukuran output tidak valid.", error.to_string())
    })
}

fn create_temp(parent: &Path) -> Result<(File, PathBuf), AppError> {
    for _ in 0..32 {
        let path = parent.join(format!(".vectorforge-{}.tmp", Uuid::new_v4()));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(write_error("Temporary output gagal dibuat.", error)),
        }
    }

    Err(AppError::new(
        ErrorCode::WriteFailed,
        "Nama temporary output tidak dapat direservasi.",
    ))
}

fn write_temp(file: &mut File, bytes: &[u8]) -> Result<(), AppError> {
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| write_error("Temporary output gagal ditulis.", error))
}

fn commit_temp(
    target: &Path,
    temp: &Path,
    overwrite_confirmed: bool,
) -> Result<(), AppError> {
    if target.exists() {
        if !overwrite_confirmed {
            return Err(AppError::new(
                ErrorCode::WriteFailed,
                "File tujuan sudah ada dan belum diizinkan untuk ditimpa.",
            ));
        }
        return replace_existing_file(target, temp)
            .map_err(|error| write_error("File tujuan gagal diganti.", error));
    }

    let reservation = match OpenOptions::new().write(true).create_new(true).open(target) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            return Err(AppError::new(
                ErrorCode::WriteFailed,
                "File tujuan dibuat proses lain sebelum export selesai.",
            ));
        }
        Err(error) => return Err(write_error("Nama output gagal direservasi.", error)),
    };
    drop(reservation);

    if let Err(error) = replace_existing_file(target, temp) {
        let _ = fs::remove_file(target);
        return Err(write_error("Output baru gagal dikomit.", error));
    }

    Ok(())
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

fn write_error(message: &str, error: io::Error) -> AppError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => {
            AppError::with_details(ErrorCode::AccessDenied, message, error.to_string())
        }
        _ => AppError::with_details(ErrorCode::WriteFailed, message, error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    #[cfg(windows)]
    use std::fs::OpenOptions;
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;

    use super::write_atomic;

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn create() -> Result<Self, String> {
            let path = std::env::temp_dir().join(format!(
                "vectorforge-b04-fs-{}",
                Uuid::new_v4()
            ));
            fs::create_dir(&path).map_err(|error| error.to_string())?;
            Ok(Self { path })
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[test]
    fn new_output_commits_exact_bytes() -> Result<(), String> {
        let dir = TempDir::create()?;
        let target = dir.path.join("new.svg");
        write_atomic(&target, b"new-vector", false).map_err(|error| error.message)?;
        assert_eq!(fs::read(target).map_err(|error| error.to_string())?, b"new-vector");
        Ok(())
    }

    #[test]
    fn unconfirmed_collision_preserves_existing_output() -> Result<(), String> {
        let dir = TempDir::create()?;
        let target = dir.path.join("existing.svg");
        fs::write(&target, b"keep").map_err(|error| error.to_string())?;
        assert!(write_atomic(&target, b"replace", false).is_err());
        assert_eq!(fs::read(target).map_err(|error| error.to_string())?, b"keep");
        Ok(())
    }

    #[test]
    fn confirmed_overwrite_replaces_existing_output() -> Result<(), String> {
        let dir = TempDir::create()?;
        let target = dir.path.join("existing.svg");
        fs::write(&target, b"old").map_err(|error| error.to_string())?;
        write_atomic(&target, b"new", true).map_err(|error| error.message)?;
        assert_eq!(fs::read(target).map_err(|error| error.to_string())?, b"new");
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn locked_overwrite_preserves_existing_output() -> Result<(), String> {
        let dir = TempDir::create()?;
        let target = dir.path.join("locked.svg");
        fs::write(&target, b"keep-old").map_err(|error| error.to_string())?;

        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&target)
            .map_err(|error| error.to_string())?;

        assert!(write_atomic(&target, b"should-not-commit", true).is_err());
        drop(locked);

        assert_eq!(
            fs::read(target).map_err(|error| error.to_string())?,
            b"keep-old"
        );
        Ok(())
    }
}
