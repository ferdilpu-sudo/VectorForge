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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentationMode {
    #[default]
    ColorCluster,
    Watershed,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GeometryMode {
    #[default]
    Classic,
    LineAware,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceParams {
    pub color_precision: u8,
    pub filter_speckle: u16,
    pub layer_difference: u16,
    pub corner_threshold: u16,
    pub length_threshold: f64,
    // Old preset JSON omits this field. Zero leaves VTracer simplification disabled.
    #[serde(default)]
    pub smoothness: f64,
    // Old preset JSON uses the classic segmentation with no palette cap.
    #[serde(default)]
    pub segmentation_mode: SegmentationMode,
    #[serde(default)]
    pub palette_limit: u8,
    #[serde(default)]
    pub geometry_mode: GeometryMode,
    #[serde(default)]
    pub quality_guard: bool,
    #[serde(default)]
    pub raster_feedback: bool,
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
            && (3.5..=10.0).contains(&self.length_threshold)
            && self.smoothness.is_finite()
            && (0.0..=3.0).contains(&self.smoothness)
            && self.palette_limit <= 64;

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
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| AppError::new(ErrorCode::InvalidParams, format!("{label} tidak valid.")))
}

#[cfg(test)]
mod tests {
    use super::{
        GeometryMode, HierarchicalMode, PreviewRequest, SegmentationMode, TraceMode, TraceParams,
    };

    fn params() -> TraceParams {
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
            quality_guard: false,
            raster_feedback: false,
            mode: TraceMode::Spline,
            hierarchical: HierarchicalMode::Stacked,
        }
    }

    #[test]
    fn legacy_trace_params_default_to_no_smoothing() -> Result<(), String> {
        let legacy = r#"{
            "colorPrecision":6,
            "filterSpeckle":4,
            "layerDifference":16,
            "cornerThreshold":60,
            "lengthThreshold":4,
            "mode":"spline",
            "hierarchical":"stacked"
        }"#;
        let parsed: TraceParams =
            serde_json::from_str(legacy).map_err(|error| error.to_string())?;
        assert_eq!(parsed.smoothness, 0.0);
        assert_eq!(parsed.segmentation_mode, SegmentationMode::ColorCluster);
        assert_eq!(parsed.palette_limit, 0);
        assert_eq!(parsed.geometry_mode, GeometryMode::Classic);
        assert!(!parsed.quality_guard);
        parsed.validate().map_err(|error| error.message)?;

        let serialized = serde_json::to_value(parsed).map_err(|error| error.to_string())?;
        assert_eq!(serialized["smoothness"], 0.0);
        assert_eq!(serialized["geometryMode"], "classic");
        assert_eq!(serialized["qualityGuard"], false);
        Ok(())
    }

    #[test]
    fn smoothness_range_rejects_nonfinite_and_out_of_bounds() {
        let mut params = params();
        for invalid in [-0.1, 3.1, f64::NAN, f64::INFINITY] {
            params.smoothness = invalid;
            assert!(params.validate().is_err());
        }
        params.smoothness = 1.5;
        assert!(params.validate().is_ok());
    }

    #[test]
    fn professional_trace_options_deserialize_and_validate() -> Result<(), String> {
        let mut params = params();
        params.segmentation_mode = SegmentationMode::Watershed;
        params.palette_limit = 24;
        let json = serde_json::to_string(&params).map_err(|error| error.to_string())?;
        assert!(json.contains("\"segmentationMode\":\"watershed\""));
        assert!(json.contains("\"paletteLimit\":24"));
        let restored: TraceParams =
            serde_json::from_str(&json).map_err(|error| error.to_string())?;
        assert_eq!(restored, params);
        restored.validate().map_err(|error| error.message)?;

        params.palette_limit = 65;
        assert!(params.validate().is_err());
        Ok(())
    }

    #[test]
    fn line_aware_geometry_mode_serializes_and_validates() -> Result<(), String> {
        let mut params = params();
        params.geometry_mode = GeometryMode::LineAware;
        let text = serde_json::to_string(&params).map_err(|error| error.to_string())?;
        assert!(text.contains("\"geometryMode\":\"line-aware\""));
        let restored: TraceParams =
            serde_json::from_str(&text).map_err(|error| error.to_string())?;
        assert_eq!(restored.geometry_mode, GeometryMode::LineAware);
        restored.validate().map_err(|error| error.message)
    }

    #[test]
    fn quality_guard_round_trips_in_camel_case() -> Result<(), String> {
        let mut options = params();
        options.quality_guard = true;
        let value = serde_json::to_value(options).map_err(|error| error.to_string())?;
        assert_eq!(value["qualityGuard"], true);
        let restored: TraceParams =
            serde_json::from_value(value).map_err(|error| error.to_string())?;
        assert!(restored.quality_guard);
        restored.validate().map_err(|error| error.message)
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
