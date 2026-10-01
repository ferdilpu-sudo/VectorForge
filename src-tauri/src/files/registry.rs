use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use crate::models::{AppError, DestinationKind, ErrorCode, ExportFormat};

#[derive(Debug, Clone)]
struct SourceEntry {
    path: PathBuf,
    fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct SourceSnapshot {
    pub path: PathBuf,
    pub fingerprint: String,
}

impl SourceEntry {
    fn validate(&self) -> Result<(), AppError> {
        if self.path.as_os_str().is_empty() || self.fingerprint.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidState,
                "Registry sumber menerima data yang tidak valid.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct DestinationEntry {
    path: PathBuf,
    kind: DestinationKind,
    format: Option<ExportFormat>,
    overwrite_confirmed: bool,
}

#[derive(Debug, Clone)]
pub struct DestinationSnapshot {
    pub path: PathBuf,
    pub kind: DestinationKind,
    pub format: Option<ExportFormat>,
    pub overwrite_confirmed: bool,
}

impl DestinationEntry {
    fn validate(&self) -> Result<(), AppError> {
        if self.path.as_os_str().is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidState,
                "Registry tujuan menerima path kosong.",
            ));
        }

        match self.kind {
            DestinationKind::File if self.format.is_none() => Err(AppError::new(
                ErrorCode::InvalidState,
                "Tujuan file wajib menyimpan format output.",
            )),
            DestinationKind::Directory if self.format.is_some() || self.overwrite_confirmed => {
                Err(AppError::new(
                    ErrorCode::InvalidState,
                    "Tujuan folder memiliki state file yang tidak valid.",
                ))
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Default)]
pub struct FileRegistry {
    sources: HashMap<String, SourceEntry>,
    destinations: HashMap<String, DestinationEntry>,
    outputs: HashMap<String, PathBuf>,
}

impl FileRegistry {
    pub fn register_source(
        &mut self,
        path: PathBuf,
        fingerprint: String,
    ) -> Result<String, AppError> {
        let entry = SourceEntry { path, fingerprint };
        entry.validate()?;

        let id = Uuid::new_v4().to_string();
        self.sources.insert(id.clone(), entry);
        Ok(id)
    }

    pub fn resolve_source(&self, file_id: &str) -> Result<SourceSnapshot, AppError> {
        validate_uuid(file_id)?;
        let entry = self
            .sources
            .get(file_id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "File sumber tidak terdaftar."))?;

        Ok(SourceSnapshot {
            path: entry.path.clone(),
            fingerprint: entry.fingerprint.clone(),
        })
    }

    pub fn release_sources(&mut self, file_ids: &[String]) -> Result<(), AppError> {
        for file_id in file_ids {
            validate_uuid(file_id)?;
        }
        for file_id in file_ids {
            self.sources.remove(file_id);
        }
        Ok(())
    }

    pub fn resolve_destination(
        &self,
        destination_id: &str,
    ) -> Result<DestinationSnapshot, AppError> {
        validate_uuid(destination_id)?;
        let entry = self
            .destinations
            .get(destination_id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Tujuan output tidak terdaftar."))?;

        Ok(DestinationSnapshot {
            path: entry.path.clone(),
            kind: entry.kind,
            format: entry.format,
            overwrite_confirmed: entry.overwrite_confirmed,
        })
    }

    pub fn register_destination(
        &mut self,
        path: PathBuf,
        kind: DestinationKind,
        format: Option<ExportFormat>,
        overwrite_confirmed: bool,
    ) -> Result<String, AppError> {
        let entry = DestinationEntry {
            path,
            kind,
            format,
            overwrite_confirmed,
        };
        entry.validate()?;

        let id = Uuid::new_v4().to_string();
        self.destinations.insert(id.clone(), entry);
        Ok(id)
    }

    pub fn unregister_output(&mut self, output_id: &str) {
        self.outputs.remove(output_id);
    }

    pub fn register_output(&mut self, path: PathBuf) -> Result<String, AppError> {
        if path.as_os_str().is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidState,
                "Registry output menerima path kosong.",
            ));
        }

        let id = Uuid::new_v4().to_string();
        self.outputs.insert(id.clone(), path);
        Ok(id)
    }
}

fn validate_uuid(value: &str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, "ID file tidak valid."))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::models::{DestinationKind, ExportFormat};

    use super::FileRegistry;

    #[test]
    fn source_ids_are_uuid_and_release_is_idempotent_for_known_shape() -> Result<(), String> {
        let mut registry = FileRegistry::default();
        let id = registry
            .register_source(PathBuf::from("fixture.png"), "fingerprint".to_owned())
            .map_err(|error| error.message)?;

        uuid::Uuid::parse_str(&id).map_err(|error| error.to_string())?;
        let entry = registry
            .resolve_source(&id)
            .map_err(|error| error.message)?;
        if entry.path.as_path() != Path::new("fixture.png") || entry.fingerprint != "fingerprint" {
            return Err("registered source changed".to_owned());
        }

        registry
            .release_sources(std::slice::from_ref(&id))
            .map_err(|error| error.message)?;
        registry
            .release_sources(std::slice::from_ref(&id))
            .map_err(|error| error.message)?;

        if registry.sources.contains_key(&id) {
            return Err("source survived release".to_owned());
        }
        Ok(())
    }

    #[test]
    fn malformed_release_id_is_rejected() {
        let mut registry = FileRegistry::default();
        assert!(
            registry
                .release_sources(&["not-a-uuid".to_owned()])
                .is_err()
        );
    }

    #[test]
    fn destination_snapshot_preserves_write_authority() -> Result<(), String> {
        let mut registry = FileRegistry::default();
        let id = registry
            .register_destination(
                PathBuf::from("output.svg"),
                DestinationKind::File,
                Some(ExportFormat::Svg),
                true,
            )
            .map_err(|error| error.message)?;

        let snapshot = registry
            .resolve_destination(&id)
            .map_err(|error| error.message)?;
        if snapshot.path.as_path() != Path::new("output.svg")
            || snapshot.kind != DestinationKind::File
            || snapshot.format != Some(ExportFormat::Svg)
            || !snapshot.overwrite_confirmed
        {
            return Err("destination snapshot changed".to_owned());
        }

        Ok(())
    }

    #[test]
    fn output_id_registers_opaque_output_path() -> Result<(), String> {
        let mut registry = FileRegistry::default();
        let id = registry
            .register_output(PathBuf::from("result.svg"))
            .map_err(|error| error.message)?;

        uuid::Uuid::parse_str(&id).map_err(|error| error.to_string())?;
        assert_eq!(
            registry.outputs.get(&id).map(PathBuf::as_path),
            Some(Path::new("result.svg"))
        );
        Ok(())
    }

    #[test]
    fn directory_destination_rejects_file_only_state() {
        let mut registry = FileRegistry::default();
        assert!(
            registry
                .register_destination(
                    PathBuf::from("output"),
                    DestinationKind::Directory,
                    Some(ExportFormat::Svg),
                    true,
                )
                .is_err()
        );
    }
}
