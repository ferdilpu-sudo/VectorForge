use std::collections::HashSet;
use std::path::Path;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

use crate::models::{
    AppError, ErrorCode, HierarchicalMode, PRESETS_VERSION, Preset, PresetsFile, SavePresetRequest,
    SegmentationMode,
    TraceMode, TraceParams,
};

use super::fs::{read_versioned, write_json_atomic};

const FILE_NAME: &str = "presets.json";
const BUILTIN_CREATED_AT: &str = "2026-09-30T00:00:00Z";

pub fn list_presets(app_data_dir: &Path) -> Result<Vec<Preset>, AppError> {
    let mut presets = built_in_presets();
    presets.extend(load_user_presets(app_data_dir)?);
    Ok(presets)
}

pub fn save_preset(app_data_dir: &Path, request: SavePresetRequest) -> Result<Preset, AppError> {
    let name = request.validate()?;
    let mut users = load_user_presets(app_data_dir)?;
    ensure_unique_name(&name, &users)?;

    let created_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| {
            AppError::invalid_state("Waktu preset gagal dibuat.", error.to_string())
        })?;
    let preset = Preset {
        id: Uuid::new_v4().to_string(),
        name,
        params: request.params,
        created_at,
        built_in: false,
    };

    users.push(preset.clone());
    write_user_presets(app_data_dir, users)?;
    Ok(preset)
}

pub fn delete_preset(app_data_dir: &Path, id: &str) -> Result<(), AppError> {
    if built_in_presets().iter().any(|preset| preset.id == id) {
        return Err(AppError::new(
            ErrorCode::PresetReadOnly,
            "Preset bawaan tidak dapat dihapus.",
        ));
    }
    Uuid::parse_str(id)
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, "ID preset tidak valid."))?;

    let mut users = load_user_presets(app_data_dir)?;
    let before = users.len();
    users.retain(|preset| preset.id != id);
    if users.len() == before {
        return Err(AppError::new(
            ErrorCode::NotFound,
            "Preset tidak ditemukan.",
        ));
    }

    write_user_presets(app_data_dir, users)
}

pub fn built_in_presets() -> Vec<Preset> {
    let defaults = default_params();
    vec![
        Preset {
            id: "builtin-balanced".to_owned(),
            name: "Seimbang".to_owned(),
            params: defaults,
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-logo".to_owned(),
            name: "Logo & Flat".to_owned(),
            params: TraceParams {
                color_precision: 4,
                filter_speckle: 8,
                layer_difference: 32,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-photo".to_owned(),
            name: "Foto Detail".to_owned(),
            params: TraceParams {
                color_precision: 8,
                filter_speckle: 2,
                layer_difference: 8,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-poster".to_owned(),
            name: "Poster Halus".to_owned(),
            params: TraceParams {
                color_precision: 7,
                corner_threshold: 90,
                mode: TraceMode::Spline,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-ai-smooth".to_owned(),
            name: "AI Smooth Illustration".to_owned(),
            params: TraceParams {
                color_precision: 6,
                filter_speckle: 5,
                layer_difference: 20,
                corner_threshold: 75,
                smoothness: 1.5,
                mode: TraceMode::Spline,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-professional-logo".to_owned(),
            name: "Professional Logo".to_owned(),
            params: TraceParams {
                color_precision: 5,
                filter_speckle: 3,
                layer_difference: 20,
                corner_threshold: 45,
                smoothness: 0.0,
                hierarchical: HierarchicalMode::Cutout,
                segmentation_mode: SegmentationMode::ColorCluster,
                palette_limit: 0,
                geometry_mode: crate::models::GeometryMode::LineAware,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
        Preset {
            id: "builtin-professional-illustration".to_owned(),
            name: "Professional Illustration".to_owned(),
            params: TraceParams {
                color_precision: 6,
                filter_speckle: 5,
                layer_difference: 18,
                corner_threshold: 75,
                smoothness: 0.5,
                hierarchical: HierarchicalMode::Cutout,
                segmentation_mode: SegmentationMode::Watershed,
                palette_limit: 24,
                ..defaults
            },
            created_at: BUILTIN_CREATED_AT.to_owned(),
            built_in: true,
        },
    ]
}

fn default_params() -> TraceParams {
    TraceParams {
        color_precision: 6,
        filter_speckle: 4,
        layer_difference: 16,
        corner_threshold: 60,
        length_threshold: 4.0,
        smoothness: 0.0,
        segmentation_mode: crate::models::SegmentationMode::ColorCluster,
        palette_limit: 0,
        geometry_mode: crate::models::GeometryMode::Classic,
        mode: TraceMode::Spline,
        hierarchical: HierarchicalMode::Stacked,
    }
}

fn load_user_presets(app_data_dir: &Path) -> Result<Vec<Preset>, AppError> {
    let path = app_data_dir.join(FILE_NAME);
    let file: Option<PresetsFile> = read_versioned(&path, PRESETS_VERSION, "Preset")?;
    let Some(file) = file else {
        return Ok(Vec::new());
    };

    validate_user_presets(&file.presets)?;
    Ok(file.presets)
}

fn write_user_presets(app_data_dir: &Path, presets: Vec<Preset>) -> Result<(), AppError> {
    let path = app_data_dir.join(FILE_NAME);
    if path.exists() {
        let _ = load_user_presets(app_data_dir)?;
    }

    let file = PresetsFile {
        version: PRESETS_VERSION,
        presets,
    };
    write_json_atomic(&path, &file, "Preset")
}

fn validate_user_presets(presets: &[Preset]) -> Result<(), AppError> {
    let mut ids = HashSet::new();
    let mut names = HashSet::new();

    for preset in presets {
        let valid_name =
            preset.name.trim() == preset.name && (1..=40).contains(&preset.name.chars().count());
        let valid_id = Uuid::parse_str(&preset.id).is_ok();
        let valid_date = OffsetDateTime::parse(&preset.created_at, &Rfc3339).is_ok();

        if preset.built_in
            || !valid_name
            || !valid_id
            || !valid_date
            || preset.params.validate().is_err()
            || !ids.insert(preset.id.clone())
            || !names.insert(preset.name.to_lowercase())
        {
            return Err(AppError::new(
                ErrorCode::DataCorrupt,
                "Preset tersimpan rusak atau tidak valid.",
            ));
        }
    }

    for builtin in built_in_presets() {
        // New built-in names must not invalidate custom presets created
        // before their introduction (presets.json remains version 1).
        if matches!(
            builtin.id.as_str(),
            "builtin-ai-smooth"
                | "builtin-professional-logo"
                | "builtin-professional-illustration"
        ) {
            continue;
        }
        if names.contains(&builtin.name.to_lowercase()) {
            return Err(AppError::new(
                ErrorCode::DataCorrupt,
                "Preset tersimpan menggunakan nama preset bawaan.",
            ));
        }
    }

    Ok(())
}

fn ensure_unique_name(name: &str, users: &[Preset]) -> Result<(), AppError> {
    let key = name.to_lowercase();
    let duplicate = built_in_presets()
        .iter()
        .chain(users.iter())
        .any(|preset| preset.name.to_lowercase() == key);

    if duplicate {
        Err(AppError::new(
            ErrorCode::PresetDuplicate,
            "Nama preset sudah digunakan.",
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use uuid::Uuid;

    use crate::models::{
        ErrorCode, HierarchicalMode, SavePresetRequest, SegmentationMode, TraceMode, TraceParams,
    };

    use super::{built_in_presets, delete_preset, list_presets, save_preset};

    struct TempDir(PathBuf);

    impl TempDir {
        fn create() -> Result<Self, String> {
            let path =
                std::env::temp_dir().join(format!("vectorforge-b05-presets-{}", Uuid::new_v4()));
            fs::create_dir(&path).map_err(|error| error.to_string())?;
            Ok(Self(path))
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn request(name: &str) -> SavePresetRequest {
        SavePresetRequest {
            name: name.to_owned(),
            params: TraceParams {
                color_precision: 6,
                filter_speckle: 4,
                layer_difference: 16,
                corner_threshold: 60,
                length_threshold: 4.0,
                smoothness: 0.0,
                segmentation_mode: crate::models::SegmentationMode::ColorCluster,
                palette_limit: 0,
                geometry_mode: crate::models::GeometryMode::Classic,
                mode: TraceMode::Spline,
                hierarchical: HierarchicalMode::Stacked,
            },
        }
    }

    #[test]
    fn builtins_have_stable_ids_and_are_not_written_on_list() -> Result<(), String> {
        let dir = TempDir::create()?;
        let presets = list_presets(&dir.0).map_err(|error| error.message)?;
        let ids: Vec<&str> = presets.iter().map(|preset| preset.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "builtin-balanced",
                "builtin-logo",
                "builtin-photo",
                "builtin-poster",
                "builtin-ai-smooth",
                "builtin-professional-logo",
                "builtin-professional-illustration"
            ]
        );
        assert!(!dir.0.join("presets.json").exists());
        Ok(())
    }

    #[test]
    fn ai_smooth_preset_is_opt_in_and_old_presets_stay_unchanged() {
        let presets = built_in_presets();
        let balanced = presets
            .iter()
            .find(|preset| preset.id == "builtin-balanced");
        let ai = presets
            .iter()
            .find(|preset| preset.id == "builtin-ai-smooth");
        assert_eq!(balanced.map(|preset| preset.params.smoothness), Some(0.0));
        assert_eq!(ai.map(|preset| preset.params.smoothness), Some(1.5));
    }

    #[test]
    fn existing_v1_user_preset_named_like_new_builtin_remains_readable() -> Result<(), String> {
        let dir = TempDir::create()?;
        let data = serde_json::json!({
            "version": 1,
            "presets": [{
                "id": "00000000-0000-4000-8000-000000000123",
                "name": "AI Smooth Illustration",
                "params": {
                    "colorPrecision": 6,
                    "filterSpeckle": 4,
                    "layerDifference": 16,
                    "cornerThreshold": 60,
                    "lengthThreshold": 4,
                    "mode": "spline",
                    "hierarchical": "stacked"
                },
                "createdAt": "2026-10-01T00:00:00Z",
                "builtIn": false
            }]
        });
        fs::write(dir.0.join("presets.json"), data.to_string())
            .map_err(|error| error.to_string())?;
        let presets = list_presets(&dir.0).map_err(|error| error.message)?;
        let user = presets
            .iter()
            .find(|preset| preset.id == "00000000-0000-4000-8000-000000000123")
            .ok_or_else(|| "legacy user preset not found".to_owned())?;
        assert_eq!(user.params.smoothness, 0.0);
        Ok(())
    }

    #[test]
    fn professional_presets_match_contract() {
        let presets = built_in_presets();
        let logo = presets.iter().find(|preset| preset.id == "builtin-professional-logo");
        let illustration = presets
            .iter()
            .find(|preset| preset.id == "builtin-professional-illustration");
        assert_eq!(logo.map(|preset| preset.params.smoothness), Some(0.0));
        assert_eq!(
            logo.map(|preset| preset.params.geometry_mode),
            Some(crate::models::GeometryMode::LineAware)
        );
        assert_eq!(
            illustration.map(|preset| preset.params.geometry_mode),
            Some(crate::models::GeometryMode::Classic)
        );
        assert_eq!(logo.map(|preset| preset.params.hierarchical), Some(HierarchicalMode::Cutout));
        assert_eq!(
            illustration.map(|preset| preset.params.segmentation_mode),
            Some(SegmentationMode::Watershed)
        );
        assert_eq!(illustration.map(|preset| preset.params.palette_limit), Some(24));
    }

    #[test]
    fn old_user_preset_with_professional_builtin_name_is_preserved() -> Result<(), String> {
        let dir = TempDir::create()?;
        let data = serde_json::json!({
            "version": 1,
            "presets": [{
                "id": "00000000-0000-4000-8000-000000000124",
                "name": "Professional Illustration",
                "params": {
                    "colorPrecision": 6,
                    "filterSpeckle": 4,
                    "layerDifference": 16,
                    "cornerThreshold": 60,
                    "lengthThreshold": 4,
                    "mode": "spline",
                    "hierarchical": "stacked"
                },
                "createdAt": "2026-10-01T00:00:00Z",
                "builtIn": false
            }]
        });
        fs::write(dir.0.join("presets.json"), data.to_string())
            .map_err(|error| error.to_string())?;
        let presets = list_presets(&dir.0).map_err(|error| error.message)?;
        let legacy = presets
            .iter()
            .find(|preset| preset.id == "00000000-0000-4000-8000-000000000124")
            .ok_or_else(|| "legacy custom preset missing".to_owned())?;
        assert_eq!(legacy.params.segmentation_mode, SegmentationMode::ColorCluster);
        assert_eq!(legacy.params.palette_limit, 0);
        assert_eq!(legacy.params.geometry_mode, crate::models::GeometryMode::Classic);
        Ok(())
    }

    #[test]
    fn user_preset_round_trip_and_delete() -> Result<(), String> {
        let dir = TempDir::create()?;
        let saved = save_preset(&dir.0, request("Saya")).map_err(|error| error.message)?;
        assert!(!saved.built_in);
        Uuid::parse_str(&saved.id).map_err(|error| error.to_string())?;

        let listed = list_presets(&dir.0).map_err(|error| error.message)?;
        assert_eq!(listed.len(), built_in_presets().len() + 1);

        delete_preset(&dir.0, &saved.id).map_err(|error| error.message)?;
        let listed = list_presets(&dir.0).map_err(|error| error.message)?;
        assert_eq!(listed.len(), built_in_presets().len());
        Ok(())
    }

    #[test]
    fn duplicate_name_is_case_insensitive_across_builtin_and_user() -> Result<(), String> {
        let dir = TempDir::create()?;
        assert!(matches!(
            save_preset(&dir.0, request("seimbang")),
            Err(error) if error.code == ErrorCode::PresetDuplicate
        ));

        save_preset(&dir.0, request("Custom")).map_err(|error| error.message)?;
        assert!(matches!(
            save_preset(&dir.0, request("custom")),
            Err(error) if error.code == ErrorCode::PresetDuplicate
        ));
        Ok(())
    }

    #[test]
    fn builtin_delete_is_read_only() -> Result<(), String> {
        let dir = TempDir::create()?;
        assert!(matches!(
            delete_preset(&dir.0, "builtin-logo"),
            Err(error) if error.code == ErrorCode::PresetReadOnly
        ));
        Ok(())
    }

    #[test]
    fn corrupt_preset_file_is_preserved_and_blocks_mutation() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = dir.0.join("presets.json");
        let original = b"not-json".to_vec();
        fs::write(&path, &original).map_err(|error| error.to_string())?;

        assert!(matches!(
            list_presets(&dir.0),
            Err(error) if error.code == ErrorCode::DataCorrupt
        ));
        assert!(matches!(
            save_preset(&dir.0, request("New")),
            Err(error) if error.code == ErrorCode::DataCorrupt
        ));
        assert_eq!(fs::read(path).map_err(|error| error.to_string())?, original);
        Ok(())
    }

    #[test]
    fn newer_preset_version_blocks_mutation() -> Result<(), String> {
        let dir = TempDir::create()?;
        let path = dir.0.join("presets.json");
        let original = br#"{"version":2,"presets":[]}"#.to_vec();
        fs::write(&path, &original).map_err(|error| error.to_string())?;

        assert!(matches!(
            list_presets(&dir.0),
            Err(error) if error.code == ErrorCode::DataVersionUnsupported
        ));
        assert!(matches!(
            save_preset(&dir.0, request("New")),
            Err(error) if error.code == ErrorCode::DataVersionUnsupported
        ));
        assert_eq!(fs::read(path).map_err(|error| error.to_string())?, original);
        Ok(())
    }
}
