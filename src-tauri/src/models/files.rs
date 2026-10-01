use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use super::{AppError, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceFormat {
    Png,
    Jpeg,
    Webp,
    Bmp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceFile {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub bytes: u64,
    pub format: SourceFormat,
    pub has_alpha: bool,
    pub fingerprint: String,
    pub preview_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRequest {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRejection {
    pub name: String,
    pub error: AppError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub files: Vec<SourceFile>,
    pub rejected: Vec<ImportRejection>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Svg,
    Pdf,
    Eps,
}

impl ExportFormat {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Pdf => "pdf",
            Self::Eps => "eps",
        }
    }

    pub fn filter_name(self) -> &'static str {
        match self {
            Self::Svg => "SVG",
            Self::Pdf => "PDF",
            Self::Eps => "EPS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DestinationKind {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationRequest {
    pub kind: DestinationKind,
    pub suggested_name: Option<String>,
    pub format: Option<ExportFormat>,
}

impl DestinationRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.kind == DestinationKind::File && self.format.is_none() {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Format wajib dipilih untuk tujuan file.",
            ));
        }

        if let Some(name) = self.suggested_name.as_deref() {
            let trimmed = name.trim();
            let path = Path::new(trimmed);
            let single_normal_component = !trimmed.is_empty()
                && path.components().count() == 1
                && matches!(path.components().next(), Some(Component::Normal(_)));

            if !single_normal_component {
                return Err(AppError::new(
                    ErrorCode::InvalidParams,
                    "Nama file yang disarankan tidak valid.",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Destination {
    pub id: String,
    pub display_path: String,
    pub kind: DestinationKind,
}

#[cfg(test)]
mod tests {
    use super::{DestinationKind, DestinationRequest, ExportFormat, SourceFile, SourceFormat};

    #[test]
    fn source_file_serializes_canonical_field_names() -> Result<(), String> {
        let source = SourceFile {
            id: "00000000-0000-4000-8000-000000000000".to_owned(),
            name: "sample.jpg".to_owned(),
            width: 320,
            height: 240,
            bytes: 1024,
            format: SourceFormat::Jpeg,
            has_alpha: false,
            fingerprint: "abc".to_owned(),
            preview_url: "asset://sample".to_owned(),
        };
        let json = serde_json::to_value(source).map_err(|error| error.to_string())?;

        if json["format"] != "jpeg" || json["hasAlpha"] != false || json.get("previewUrl").is_none()
        {
            return Err(format!("unexpected SourceFile JSON: {json}"));
        }

        Ok(())
    }

    #[test]
    fn file_destination_requires_format_and_safe_suggested_name() {
        let missing_format = DestinationRequest {
            kind: DestinationKind::File,
            suggested_name: Some("logo.svg".to_owned()),
            format: None,
        };
        assert!(missing_format.validate().is_err());

        let unsafe_name = DestinationRequest {
            kind: DestinationKind::File,
            suggested_name: Some("../logo.svg".to_owned()),
            format: Some(ExportFormat::Svg),
        };
        assert!(unsafe_name.validate().is_err());
    }
}
