use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

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

pub struct OutputReservation {
    path: PathBuf,
    owns_placeholder: bool,
    committed: bool,
}

impl OutputReservation {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn commit(mut self, bytes: &[u8]) -> Result<(PathBuf, u64), AppError> {
        let written = write_atomic(&self.path, bytes, true)?;
        self.committed = true;
        Ok((self.path.clone(), written))
    }
}

impl Drop for OutputReservation {
    fn drop(&mut self) {
        if self.owns_placeholder && !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

pub fn reserve_batch_output(
    directory: &Path,
    stem: &str,
    extension: &str,
    overwrite: bool,
) -> Result<OutputReservation, AppError> {
    if !directory.is_dir() || stem.is_empty() || extension.is_empty() {
        return Err(AppError::new(
            ErrorCode::InvalidParams,
            "Tujuan batch atau nama output tidak valid.",
        ));
    }

    if overwrite {
        return Ok(OutputReservation {
            path: directory.join(format!("{stem}.{extension}")),
            owns_placeholder: false,
            committed: false,
        });
    }

    for number in 1..=10_000u32 {
        let file_name = if number == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({number}).{extension}")
        };
        let path = directory.join(file_name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                drop(file);
                return Ok(OutputReservation {
                    path,
                    owns_placeholder: true,
                    committed: false,
                });
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(write_error(
                    "Nama output batch gagal direservasi.",
                    error,
                ));
            }
        }
    }

    Err(AppError::new(
        ErrorCode::WriteFailed,
        "Nama output batch unik tidak tersedia.",
    ))
}

pub fn write_atomic(
    target: &Path,
    bytes: &[u8],
    overwrite_confirmed: bool,
) -> Result<u64, AppError> {
    let parent = target
        .parent()
        .ok_or_else(|| AppError::new(ErrorCode::WriteFailed, "Folder output tidak valid."))?;
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

    u64::try_from(bytes.len())
        .map_err(|error| AppError::invalid_state("Ukuran output tidak valid.", error.to_string()))
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

fn commit_temp(target: &Path, temp: &Path, overwrite_confirmed: bool) -> Result<(), AppError> {
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

    commit_new_file(target, temp).map_err(|error| write_error("Output baru gagal dikomit.", error))
}

#[cfg(windows)]
fn commit_new_file(target: &Path, temp: &Path) -> io::Result<()> {
    fs::rename(temp, target)
}

#[cfg(not(windows))]
fn commit_new_file(target: &Path, temp: &Path) -> io::Result<()> {
    if target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "target appeared before commit",
        ));
    }
    fs::rename(temp, target)
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

    use super::{reserve_batch_output, write_atomic};

    struct TempDir {
        path: PathBuf,
    }

    impl TempDir {
        fn create() -> Result<Self, String> {
            let path = std::env::temp_dir().join(format!("vectorforge-b04-fs-{}", Uuid::new_v4()));
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
        assert_eq!(
            fs::read(target).map_err(|error| error.to_string())?,
            b"new-vector"
        );
        Ok(())
    }

    #[test]
    fn unconfirmed_collision_preserves_existing_output() -> Result<(), String> {
        let dir = TempDir::create()?;
        let target = dir.path.join("existing.svg");
        fs::write(&target, b"keep").map_err(|error| error.to_string())?;
        assert!(write_atomic(&target, b"replace", false).is_err());
        assert_eq!(
            fs::read(target).map_err(|error| error.to_string())?,
            b"keep"
        );
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

    #[test]
    fn batch_reservation_uses_suffix_without_racing_existing_file() -> Result<(), String> {
        let dir = TempDir::create()?;
        fs::write(dir.path.join("logo.svg"), b"existing")
            .map_err(|error| error.to_string())?;

        let reservation = reserve_batch_output(&dir.path, "logo", "svg", false)
            .map_err(|error| error.message)?;
        assert_eq!(
            reservation
                .path()
                .file_name()
                .and_then(|value| value.to_str()),
            Some("logo (2).svg")
        );

        let (path, bytes) = reservation
            .commit(b"vector")
            .map_err(|error| error.message)?;
        assert_eq!(bytes, 6);
        assert_eq!(
            fs::read(path).map_err(|error| error.to_string())?,
            b"vector"
        );
        assert_eq!(
            fs::read(dir.path.join("logo.svg")).map_err(|error| error.to_string())?,
            b"existing"
        );
        Ok(())
    }

    #[test]
    fn dropped_batch_reservation_cleans_placeholder() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = {
            let reservation = reserve_batch_output(&dir.path, "logo", "pdf", false)
                .map_err(|error| error.message)?;
            reservation.path().to_path_buf()
        };
        assert!(!path.exists());
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
