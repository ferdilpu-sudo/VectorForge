use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppError, ErrorCode, ExportFormat, TraceParams};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Queued,
    Processing,
    Done,
    Partial,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputStatus {
    Queued,
    Processing,
    Done,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStage {
    Waiting,
    Decoding,
    Tracing,
    Exporting,
    Finished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    Running,
    Cancelling,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub file_ids: Vec<String>,
    pub params: TraceParams,
    pub formats: Vec<ExportFormat>,
    pub destination_id: String,
    pub overwrite: bool,
}

impl BatchRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        self.params.validate()?;
        validate_uuid(&self.destination_id, "ID tujuan")?;

        if self.file_ids.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Batch membutuhkan minimal satu file.",
            ));
        }
        if self.formats.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Batch membutuhkan minimal satu format.",
            ));
        }

        let mut file_ids = HashSet::with_capacity(self.file_ids.len());
        for file_id in &self.file_ids {
            validate_uuid(file_id, "ID file")?;
            if !file_ids.insert(file_id) {
                return Err(AppError::new(
                    ErrorCode::InvalidParams,
                    "Batch tidak boleh berisi ID file duplikat.",
                ));
            }
        }

        for (index, format) in self.formats.iter().enumerate() {
            if self.formats[..index].contains(format) {
                return Err(AppError::new(
                    ErrorCode::InvalidParams,
                    "Format batch harus unik.",
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchHandle {
    pub batch_id: String,
    pub run_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOutput {
    pub format: ExportFormat,
    pub status: OutputStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchItem {
    pub id: String,
    pub file_id: String,
    pub name: String,
    pub status: ItemStatus,
    pub stage: JobStage,
    pub outputs: Vec<BatchOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<AppError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchProgress {
    pub batch_id: String,
    pub run_id: String,
    pub sequence: u64,
    pub status: BatchStatus,
    pub items: Vec<BatchItem>,
    pub completed_count: u32,
    pub done_count: u32,
    pub partial_count: u32,
    pub failed_count: u32,
    pub cancelled_count: u32,
    pub total: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchSummary {
    pub batch_id: String,
    pub run_id: String,
    pub sequence: u64,
    pub succeeded: u32,
    pub partial: u32,
    pub failed: u32,
    pub cancelled_count: u32,
    pub cancel_requested: bool,
    pub total: u32,
    pub total_elapsed_ms: u64,
}

pub fn validate_batch_id(value: &str, label: &str) -> Result<(), AppError> {
    validate_uuid(value, label)
}

fn validate_uuid(value: &str, label: &str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, format!("{label} tidak valid.")))
}

#[cfg(test)]
mod tests {
    use crate::models::{HierarchicalMode, TraceMode};

    use super::{BatchRequest, ExportFormat, TraceParams};

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
    fn batch_request_deserializes_canonical_shape() -> Result<(), String> {
        let json = r#"{
            "fileIds":[
                "00000000-0000-4000-8000-000000000001",
                "00000000-0000-4000-8000-000000000002"
            ],
            "params":{
                "colorPrecision":6,
                "filterSpeckle":4,
                "layerDifference":16,
                "cornerThreshold":60,
                "lengthThreshold":4,
                "mode":"spline",
                "hierarchical":"stacked"
            },
            "formats":["svg","pdf"],
            "destinationId":"00000000-0000-4000-8000-000000000003",
            "overwrite":false
        }"#;

        let request: BatchRequest =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        request.validate().map_err(|error| error.message)
    }

    #[test]
    fn batch_request_rejects_duplicate_files_and_formats() {
        let file_id = "00000000-0000-4000-8000-000000000001".to_owned();
        let destination_id = "00000000-0000-4000-8000-000000000003".to_owned();

        let duplicate_file = BatchRequest {
            file_ids: vec![file_id.clone(), file_id],
            params: params(),
            formats: vec![ExportFormat::Svg],
            destination_id: destination_id.clone(),
            overwrite: false,
        };
        assert!(duplicate_file.validate().is_err());

        let duplicate_format = BatchRequest {
            file_ids: vec!["00000000-0000-4000-8000-000000000001".to_owned()],
            params: params(),
            formats: vec![ExportFormat::Svg, ExportFormat::Svg],
            destination_id,
            overwrite: false,
        };
        assert!(duplicate_format.validate().is_err());
    }
}
