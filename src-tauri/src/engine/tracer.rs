use std::panic::{AssertUnwindSafe, catch_unwind};

use image::RgbaImage;
use vtracer::{
    CancelToken, ColorImage, Config, Error as VTracerError, FitMode,
    Hierarchical as VTracerHierarchical, VectorDoc,
};

use crate::models::{AppError, ErrorCode, HierarchicalMode, TraceMode, TraceParams};

use super::alpha::split_by_source_alpha;

pub fn trace_preview(
    image: RgbaImage,
    params: &TraceParams,
    cancel: &CancelToken,
) -> Result<Option<VectorDoc>, AppError> {
    if cancel.is_cancelled() {
        return Err(cancelled_error());
    }

    let source = rgba_to_color_image(image);
    if is_fully_transparent(&source) {
        return Ok(None);
    }

    match catch_unwind(AssertUnwindSafe(|| trace_inner(&source, params, cancel))) {
        Ok(result) => result.map(Some),
        Err(_) => Err(AppError::new(
            ErrorCode::TraceFailed,
            "Engine tracing berhenti secara tidak terduga.",
        )),
    }
}

fn trace_inner(
    source: &ColorImage,
    params: &TraceParams,
    cancel: &CancelToken,
) -> Result<VectorDoc, AppError> {
    let pipeline = config_from_params(params).build().map_err(vtracer_error)?;

    let mut progress = |_| {};
    let segmentation = pipeline
        .segment_with_progress(source, cancel, &mut progress)
        .map_err(vtracer_error)?;
    let segmentation = split_by_source_alpha(segmentation, source, cancel)?;
    pipeline
        .finish_with_progress(&segmentation, cancel, &mut progress)
        .map_err(vtracer_error)
}

fn config_from_params(params: &TraceParams) -> Config {
    Config {
        hierarchical: match params.hierarchical {
            HierarchicalMode::Stacked => VTracerHierarchical::Stacked,
            HierarchicalMode::Cutout => VTracerHierarchical::Cutout,
        },
        filter_speckle: usize::from(params.filter_speckle),
        color_precision: i32::from(params.color_precision),
        layer_difference: i32::from(params.layer_difference),
        mode: match params.mode {
            TraceMode::Spline => FitMode::Spline,
            TraceMode::Polygon => FitMode::Polygon,
        },
        corner_threshold: i32::from(params.corner_threshold),
        length_threshold: params.length_threshold,
        ..Config::default()
    }
}

fn rgba_to_color_image(image: RgbaImage) -> ColorImage {
    ColorImage {
        width: image.width() as usize,
        height: image.height() as usize,
        pixels: image.into_raw(),
    }
}

fn is_fully_transparent(image: &ColorImage) -> bool {
    image
        .pixels
        .iter()
        .skip(3)
        .step_by(4)
        .all(|alpha| *alpha == 0)
}

fn vtracer_error(error: VTracerError) -> AppError {
    match error {
        VTracerError::Cancelled => cancelled_error(),
        other => AppError::with_details(
            ErrorCode::TraceFailed,
            "Tracing preview gagal.",
            other.to_string(),
        ),
    }
}

fn cancelled_error() -> AppError {
    AppError::new(ErrorCode::Cancelled, "Preview dibatalkan.")
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};
    use vtracer::CancelToken;

    use crate::models::{HierarchicalMode, TraceMode, TraceParams};

    use super::{config_from_params, trace_preview};

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
    fn schema_params_map_to_vtracer_config() {
        let params = params();
        let config = config_from_params(&params);

        assert_eq!(config.color_precision, 6);
        assert_eq!(config.filter_speckle, 4);
        assert_eq!(config.layer_difference, 16);
        assert_eq!(config.corner_threshold, 60);
        assert_eq!(config.length_threshold, 4.0);
    }

    #[test]
    fn fully_transparent_preview_skips_tracer() -> Result<(), String> {
        let image = RgbaImage::from_pixel(16, 16, Rgba([10, 20, 30, 0]));
        let result =
            trace_preview(image, &params(), &CancelToken::new()).map_err(|error| error.message)?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn same_rgb_partial_alpha_survives_production_adapter() -> Result<(), String> {
        let image = RgbaImage::from_fn(32, 16, |x, _| {
            let alpha = if x < 16 { 128 } else { 255 };
            Rgba([40, 120, 220, alpha])
        });
        let doc = trace_preview(image, &params(), &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "trace unexpectedly empty".to_owned())?;

        let mut alphas: Vec<u8> = doc
            .shapes
            .iter()
            .map(|shape| shape.paint.color().a)
            .collect();
        alphas.sort_unstable();
        alphas.dedup();

        if !alphas.contains(&128) || !alphas.contains(&255) {
            return Err(format!("alpha levels were not preserved: {alphas:?}"));
        }

        Ok(())
    }
}
