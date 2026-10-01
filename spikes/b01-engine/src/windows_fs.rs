use std::fs::{self, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn unique_test_dir() -> Result<PathBuf, String> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "vectorforge-b01-fs-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

#[cfg(windows)]
pub fn run_windows_filesystem_spike() -> Result<Vec<String>, String> {
    let dir = unique_test_dir()?;
    let result = run_in_dir(&dir);
    let cleanup = fs::remove_dir_all(&dir);

    match (result, cleanup) {
        (Ok(messages), Ok(())) => Ok(messages),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(format!("filesystem spike cleanup failed: {error}")),
    }
}

#[cfg(not(windows))]
pub fn run_windows_filesystem_spike() -> Result<Vec<String>, String> {
    Err("Windows filesystem spike must run on Windows".to_owned())
}

#[cfg(windows)]
fn run_in_dir(dir: &Path) -> Result<Vec<String>, String> {
    let mut messages = Vec::new();

    let reserved = dir.join("reserved.svg");
    let first = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&reserved)
        .map_err(|error| format!("first create_new reservation failed: {error}"))?;
    let second = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&reserved);
    if !matches!(second, Err(ref error) if error.kind() == io::ErrorKind::AlreadyExists) {
        return Err("create_new did not reject a duplicate reservation".to_owned());
    }
    drop(first);
    messages.push("[PASS] Windows create_new prevents output-name collision".to_owned());

    let new_temp = dir.join("new-output.tmp");
    let new_target = dir.join("new-output.svg");
    fs::write(&new_temp, b"new-output")
        .map_err(|error| format!("write new temp failed: {error}"))?;
    fs::rename(&new_temp, &new_target)
        .map_err(|error| format!("rename new output failed: {error}"))?;
    if fs::read(&new_target).map_err(|error| error.to_string())? != b"new-output" {
        return Err("new-file commit changed bytes".to_owned());
    }
    messages.push("[PASS] Windows same-directory rename commits a new output".to_owned());

    let replace_target = dir.join("replace.svg");
    let replace_temp = dir.join("replace.tmp");
    fs::write(&replace_target, b"old")
        .map_err(|error| format!("write replace target failed: {error}"))?;
    fs::write(&replace_temp, b"new")
        .map_err(|error| format!("write replace temp failed: {error}"))?;
    replace_existing_file(&replace_target, &replace_temp)
        .map_err(|error| format!("ReplaceFileW failed: {error}"))?;
    if fs::read(&replace_target).map_err(|error| error.to_string())? != b"new" {
        return Err("ReplaceFileW did not expose replacement bytes".to_owned());
    }
    if replace_temp.exists() {
        return Err("ReplaceFileW left the replacement temp behind".to_owned());
    }
    messages.push("[PASS] Windows ReplaceFileW atomically replaces existing output".to_owned());

    let locked_target = dir.join("locked.svg");
    let locked_temp = dir.join("locked.tmp");
    fs::write(&locked_target, b"keep-old")
        .map_err(|error| format!("write locked target failed: {error}"))?;
    fs::write(&locked_temp, b"should-not-commit")
        .map_err(|error| format!("write locked temp failed: {error}"))?;

    let locked_handle = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&locked_target)
        .map_err(|error| format!("open locked target failed: {error}"))?;

    let locked_replace = replace_existing_file(&locked_target, &locked_temp);
    if locked_replace.is_ok() {
        return Err("ReplaceFileW unexpectedly replaced a deny-share target".to_owned());
    }
    if !locked_temp.exists() {
        return Err("failed locked replace lost the replacement temp before cleanup".to_owned());
    }

    drop(locked_handle);

    if fs::read(&locked_target).map_err(|error| {
        format!("read locked target after releasing deny-share handle failed: {error}")
    })? != b"keep-old"
    {
        return Err("locked replace damaged the original target".to_owned());
    }
    fs::remove_file(&locked_temp)
        .map_err(|error| format!("locked temp cleanup failed: {error}"))?;
    messages.push(
        "[PASS] Windows locked target fails safely; original survives and temp is cleanable"
            .to_owned(),
    );

    Ok(messages)
}
