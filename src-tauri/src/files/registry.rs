use std::collections::HashMap;
use std::path::PathBuf;

use uuid::Uuid;

use crate::models::{AppError, DestinationKind, ErrorCode, ExportFormat};

#[derive(Debug, Clone)]
pub struct SourceEntry {
    pub path: PathBuf,
    pub fingerprint: String,
}

#[derive(Debug, Clone)]
pub struct DestinationEntry {
    pub path: PathBuf,
    pub kind: DestinationKind,
    pub format: Option<ExportFormat>,
    pub overwrite_confirmed: bool,
}

#[derive(Debug, Default)]
pub struct FileRegistry {
    sources: HashMap<String, SourceEntry>,
    destinations: HashMap<String, DestinationEntry>,
}

impl FileRegistry {
    pub fn register_source(&mut self, path: PathBuf, fingerprint: String) -> String {
        let id = Uuid::new_v4().to_string();
        self.sources.insert(id.clone(), SourceEntry { path, fingerprint });
        id
    }

    pub fn resolve_source(&self, file_id: &str) -> Result<&SourceEntry, AppError> {
        validate_uuid(file_id)?;
        self.sources.get(file_id).ok_or_else(|| {
            AppError::new(ErrorCode::NotFound, "File sumber tidak terdaftar.")
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

    pub fn register_destination(
        &mut self,
        path: PathBuf,
        kind: DestinationKind,
        format: Option<ExportFormat>,
        overwrite_confirmed: bool,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        self.destinations.insert(
            id.clone(),
            DestinationEntry {
                path,
                kind,
                format,
                overwrite_confirmed,
            },
        );
        id
    }

    pub fn resolve_destination(
        &self,
        destination_id: &str,
    ) -> Result<&DestinationEntry, AppError> {
        validate_uuid(destination_id)?;
        self.destinations.get(destination_id).ok_or_else(|| {
            AppError::new(ErrorCode::NotFound, "Tujuan output tidak terdaftar.")
        })
    }
}

fn validate_uuid(value: &str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, "ID file tidak valid."))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::FileRegistry;

    #[test]
    fn source_ids_are_uuid_and_release_is_idempotent_for_known_shape() -> Result<(), String> {
        let mut registry = FileRegistry::default();
        let id = registry.register_source(PathBuf::from("fixture.png"), "fingerprint".to_owned());

        uuid::Uuid::parse_str(&id).map_err(|error| error.to_string())?;
        let entry = registry
            .resolve_source(&id)
            .map_err(|error| error.message)?;
        if entry.path != PathBuf::from("fixture.png") || entry.fingerprint != "fingerprint" {
            return Err("registered source changed".to_owned());
        }

        registry
            .release_sources(std::slice::from_ref(&id))
            .map_err(|error| error.message)?;
        registry
            .release_sources(std::slice::from_ref(&id))
            .map_err(|error| error.message)?;

        if registry.resolve_source(&id).is_ok() {
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
}
