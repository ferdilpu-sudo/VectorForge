use serde::{Deserialize, Serialize};

use super::{AppError, ErrorCode, TraceParams};

pub const SETTINGS_VERSION: u32 = 1;
pub const PRESETS_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Id,
    En,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Dark,
    Light,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub language: Language,
    pub theme: Theme,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_out_dir: Option<String>,
    pub worker_count: u8,
    pub auto_preview: bool,
    pub preview_max_side: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_preset_id: Option<String>,
}

impl AppSettings {
    pub fn defaults() -> Self {
        let workers = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
            .min(4) as u8;

        Self {
            language: Language::Id,
            theme: Theme::Dark,
            last_out_dir: None,
            worker_count: workers,
            auto_preview: true,
            preview_max_side: 1024,
            last_preset_id: None,
        }
    }

    pub fn validate(&self) -> Result<(), AppError> {
        if !(1..=4).contains(&self.worker_count)
            || !(512..=2048).contains(&self.preview_max_side)
        {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Pengaturan aplikasi tidak valid.",
            ));
        }

        if self
            .last_out_dir
            .as_deref()
            .is_some_and(|value| value.is_empty())
            || self
                .last_preset_id
                .as_deref()
                .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Pengaturan aplikasi tidak valid.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub params: TraceParams,
    pub created_at: String,
    pub built_in: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePresetRequest {
    pub name: String,
    pub params: TraceParams,
}

impl SavePresetRequest {
    pub fn validate(&self) -> Result<String, AppError> {
        self.params.validate()?;
        let name = self.name.trim();
        let length = name.chars().count();
        if !(1..=40).contains(&length) {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Nama preset harus 1–40 karakter.",
            ));
        }
        Ok(name.to_owned())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsFile {
    pub version: u32,
    pub settings: AppSettings,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetsFile {
    pub version: u32,
    pub presets: Vec<Preset>,
}

#[cfg(test)]
mod tests {
    use super::{AppSettings, Language, SavePresetRequest, Theme};
    use crate::models::{HierarchicalMode, TraceMode, TraceParams};

    fn params() -> TraceParams {
        TraceParams {
            color_precision: 6,
            filter_speckle: 4,
            layer_difference: 16,
            corner_threshold: 60,
            length_threshold: 4.0,
            mode: TraceMode::Spline,
            hierarchical: HierarchicalMode::Stacked,
        }
    }

    #[test]
    fn settings_serialize_with_canonical_camel_case() -> Result<(), String> {
        let settings = AppSettings {
            language: Language::Id,
            theme: Theme::Dark,
            last_out_dir: Some("C:\\Vector".to_owned()),
            worker_count: 2,
            auto_preview: true,
            preview_max_side: 1024,
            last_preset_id: Some("builtin-balanced".to_owned()),
        };

        let json = serde_json::to_value(settings).map_err(|error| error.to_string())?;
        assert_eq!(json["language"], "id");
        assert_eq!(json["theme"], "dark");
        assert_eq!(json["lastOutDir"], "C:\\Vector");
        assert_eq!(json["workerCount"], 2);
        assert_eq!(json["autoPreview"], true);
        assert_eq!(json["previewMaxSide"], 1024);
        assert_eq!(json["lastPresetId"], "builtin-balanced");
        Ok(())
    }

    #[test]
    fn preset_name_is_trimmed_and_unicode_counted() -> Result<(), String> {
        let request = SavePresetRequest {
            name: "  Preset Saya  ".to_owned(),
            params: params(),
        };
        assert_eq!(
            request.validate().map_err(|error| error.message)?,
            "Preset Saya"
        );

        let invalid = SavePresetRequest {
            name: "😀".repeat(41),
            params: params(),
        };
        assert!(invalid.validate().is_err());
        Ok(())
    }

    #[test]
    fn settings_bounds_are_rejected_not_clamped() {
        let mut settings = AppSettings::defaults();
        settings.worker_count = 0;
        assert!(settings.validate().is_err());

        settings.worker_count = 2;
        settings.preview_max_side = 4096;
        assert!(settings.validate().is_err());
    }
}
