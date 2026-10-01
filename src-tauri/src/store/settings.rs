use std::path::Path;

use crate::models::{AppError, AppSettings, ErrorCode, SETTINGS_VERSION, SettingsFile};

use super::fs::{read_versioned, write_json_atomic};

const FILE_NAME: &str = "settings.json";

pub fn get_settings(app_data_dir: &Path) -> Result<AppSettings, AppError> {
    let path = app_data_dir.join(FILE_NAME);
    match read_settings_file(&path)? {
        Some(file) => Ok(file.settings),
        None => Ok(AppSettings::defaults()),
    }
}

pub fn save_settings(app_data_dir: &Path, settings: AppSettings) -> Result<AppSettings, AppError> {
    settings.validate()?;
    let path = app_data_dir.join(FILE_NAME);

    if path.exists() {
        let _ = read_settings_file(&path)?;
    }

    let file = SettingsFile {
        version: SETTINGS_VERSION,
        settings: settings.clone(),
    };
    write_json_atomic(&path, &file, "Pengaturan")?;
    Ok(settings)
}

fn read_settings_file(path: &Path) -> Result<Option<SettingsFile>, AppError> {
    let file: Option<SettingsFile> = read_versioned(path, SETTINGS_VERSION, "Pengaturan")?;

    if let Some(file) = file.as_ref() {
        file.settings.validate().map_err(|error| {
            AppError::with_details(
                ErrorCode::DataCorrupt,
                "Pengaturan rusak atau tidak valid.",
                error.message,
            )
        })?;
    }

    Ok(file)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use crate::models::preferences::{Language, Theme};
    use crate::models::{AppSettings, ErrorCode};

    use super::{get_settings, save_settings};

    struct TempDir(PathBuf);

    impl TempDir {
        fn create() -> Result<Self, String> {
            let path =
                std::env::temp_dir().join(format!("vectorforge-b05-settings-{}", Uuid::new_v4()));
            fs::create_dir(&path).map_err(|error| error.to_string())?;
            Ok(Self(path))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn settings() -> AppSettings {
        AppSettings {
            language: Language::En,
            theme: Theme::System,
            last_out_dir: Some("C:\\VectorForge".to_owned()),
            worker_count: 3,
            auto_preview: false,
            preview_max_side: 2048,
            last_preset_id: Some("builtin-logo".to_owned()),
        }
    }

    #[test]
    fn missing_settings_return_defaults_without_creating_file() -> Result<(), String> {
        let dir = TempDir::create()?;
        let loaded = get_settings(&dir.0).map_err(|error| error.message)?;
        loaded.validate().map_err(|error| error.message)?;
        assert!(!dir.0.join("settings.json").exists());
        Ok(())
    }

    #[test]
    fn settings_save_and_load_round_trip() -> Result<(), String> {
        let dir = TempDir::create()?;
        let expected = settings();
        save_settings(&dir.0, expected.clone()).map_err(|error| error.message)?;
        let actual = get_settings(&dir.0).map_err(|error| error.message)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn corrupt_settings_are_preserved_and_block_overwrite() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = dir.0.join("settings.json");
        let original = b"{ definitely-not-json".to_vec();
        fs::write(&path, &original).map_err(|error| error.to_string())?;

        assert!(matches!(
            get_settings(&dir.0),
            Err(error) if error.code == ErrorCode::DataCorrupt
        ));
        assert!(matches!(
            save_settings(&dir.0, settings()),
            Err(error) if error.code == ErrorCode::DataCorrupt
        ));
        assert_eq!(fs::read(path).map_err(|error| error.to_string())?, original);
        Ok(())
    }

    #[test]
    fn newer_settings_version_is_preserved_and_blocks_write() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = dir.0.join("settings.json");
        let original = br#"{"version":2,"settings":{}}"#.to_vec();
        fs::write(&path, &original).map_err(|error| error.to_string())?;

        assert!(matches!(
            get_settings(&dir.0),
            Err(error) if error.code == ErrorCode::DataVersionUnsupported
        ));
        assert!(matches!(
            save_settings(&dir.0, settings()),
            Err(error) if error.code == ErrorCode::DataVersionUnsupported
        ));
        assert_eq!(fs::read(path).map_err(|error| error.to_string())?, original);
        Ok(())
    }
}
