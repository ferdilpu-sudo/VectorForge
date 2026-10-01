use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::TraceParams;

    #[test]
    fn trace_params_deserialize_camel_case_and_validate() -> Result<(), String> {
        let json = r#"{
            "colorPrecision": 6,
            "filterSpeckle": 4,
            "layerDifference": 16,
            "cornerThreshold": 60,
            "lengthThreshold": 4.0,
            "mode": "spline",
            "hierarchical": "stacked"
        }"#;
        let params: TraceParams =
            serde_json::from_str(json).map_err(|error| error.to_string())?;

        params.validate().map_err(|error| error.message)
    }

    #[test]
    fn trace_params_reject_non_finite_and_out_of_range() {
        let mut params = TraceParams {
            color_precision: 6,
            filter_speckle: 4,
            layer_difference: 16,
            corner_threshold: 60,
            length_threshold: f64::NAN,
            mode: super::TraceMode::Spline,
            hierarchical: super::HierarchicalMode::Stacked,
        };
        assert!(params.validate().is_err());

        params.length_threshold = 4.0;
        params.color_precision = 0;
        assert!(params.validate().is_err());
    }
}
