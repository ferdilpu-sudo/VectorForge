use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppError, ErrorCode};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TraceMode {
    Spline,
    Polygon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HierarchicalMode {
    Stacked,
    Cutout,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceParams {
    pub color_precision: u8,
    pub filter_speckle: u16,
    pub layer_difference: u16,
    pub corner_threshold: u16,
    pub length_threshold: f64,
    pub mode: TraceMode,
    pub hierarchical: HierarchicalMode,
}

impl TraceParams {
    pub fn validate(&self) -> Result<(), AppError> {
        let valid = (1..=8).contains(&self.color_precision)
            && self.filter_speckle <= 128
            && self.layer_difference <= 128
            && self.corner_threshold <= 180
            && self.length_threshold.is_finite()
            && (3.5..=10.0).contains(&self.length_threshold);

        if valid {
            Ok(())
        } else {
            Err(AppError::new(
                ErrorCode::InvalidParams,
                "Parameter tracing tidak valid.",
            ))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRequest {
    pub file_id: String,
    pub params: TraceParams,
    pub max_side: u32,
    pub request_id: String,
}

impl PreviewRequest {
    pub fn validate(&self) -> Result<(), AppError> {
        validate_uuid(&self.file_id, "ID file")?;
        validate_uuid(&self.request_id, "ID request")?;
        self.params.validate()?;

        if !(512..=2048).contains(&self.max_side) {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Ukuran maksimum preview harus 512–2048 piksel.",
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceStats {
    pub path_count: u64,
    pub color_count: u64,
    pub svg_bytes: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewResult {
    pub request_id: String,
    pub file_id: String,
    pub svg: String,
    pub stats: TraceStats,
    pub elapsed_ms: u64,
}

pub fn validate_request_id(request_id: &str) -> Result<(), AppError> {
    validate_uuid(request_id, "ID request")
}

fn validate_uuid(value: &str, label: &str) -> Result<(), AppError> {
    Uuid::parse_str(value).map(|_| ()).map_err(|_| {
        AppError::new(
            ErrorCode::InvalidParams,
            format!("{label} tidak valid."),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{HierarchicalMode, PreviewRequest, TraceMode, TraceParams};

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
    fn preview_request_deserializes_camel_case() -> Result<(), String> {
        let json = r#"{
            "fileId":"00000000-0000-4000-8000-000000000001",
            "requestId":"00000000-0000-4000-8000-000000000002",
            "maxSide":1024,
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
        let request: PreviewRequest =
            serde_json::from_str(json).map_err(|error| error.to_string())?;
        request.validate().map_err(|error| error.message)
    }

    #[test]
    fn preview_request_rejects_invalid_boundary_values() {
        let mut request = PreviewRequest {
            file_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            request_id: "00000000-0000-4000-8000-000000000002".to_owned(),
            max_side: 511,
            params: params(),
        };
        assert!(request.validate().is_err());

        request.max_side = 1024;
        request.params.length_threshold = f64::NAN;
        assert!(request.validate().is_err());
    }
}
