use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppError, ErrorCode, ExportFormat, TraceParams, TraceStats};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportRequest {
    pub file_id: String,
    pub params: TraceParams,
    pub format: ExportFormat,
    pub destination_id: String,
    pub allow_large_output: bool,
}

impl ExportRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_uuid(&self.file_id, "ID file")?;
        validate_uuid(&self.destination_id, "ID tujuan")?;
        self.params.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub output_id: String,
    pub out_path: String,
    pub format: ExportFormat,
    pub bytes: u64,
    pub stats: TraceStats,
    pub elapsed_ms: u64,
}

fn validate_uuid(value: &str, label: &str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, format!("{label} tidak valid.")))
}

#[cfg(test)]
mod tests {
    use super::ExportRequest;

    #[test]
    fn export_request_deserializes_camel_case_and_validates() -> Result<(), String> {
        let json = r#"{
            "fileId":"00000000-0000-4000-8000-000000000001",
            "destinationId":"00000000-0000-4000-8000-000000000002",
            "format":"svg",
            "allowLargeOutput":false,
            "params":{
                "colorPrecision":6,
                "filterSpeckle":4,
                "layerDifference":16,
                "cornerThreshold":60,
                "lengthThreshold":4,
                "mode":"spline",
                "hierarchical":"stacked"
            }
        }"#;

        let request: ExportRequest =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        request.validate().map_err(|error| error.message)
    }

    #[test]
    fn export_request_rejects_invalid_destination_id() -> Result<(), String> {
        let json = r#"{
            "fileId":"00000000-0000-4000-8000-000000000001",
            "destinationId":"not-a-uuid",
            "format":"pdf",
            "allowLargeOutput":false,
            "params":{
                "colorPrecision":6,
                "filterSpeckle":4,
                "layerDifference":16,
                "cornerThreshold":60,
                "lengthThreshold":4,
                "mode":"spline",
                "hierarchical":"stacked"
            }
        }"#;

        let request: ExportRequest =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        assert!(request.validate().is_err());
        Ok(())
    }
}
