use std::panic::{AssertUnwindSafe, catch_unwind};

use image::RgbaImage;
use vtracer::{
    CancelToken, ColorImage, Config, Error as VTracerError, FitMode,
    Hierarchical as VTracerHierarchical, VectorDoc,
};

use crate::models::{AppError, ErrorCode, HierarchicalMode, TraceMode, TraceParams};

use super::alpha::split_by_source_alpha;

pub fn trace_image(
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
    use vtracer::ir::PathCmd;
    use vtracer::{CancelToken, VectorDoc};

    use crate::models::{HierarchicalMode, TraceMode, TraceParams};

    use super::{config_from_params, trace_image};

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


    // A path_count > 0 assertion cannot detect a logo reduced to one glyph.
    // Require visible dark geometry within every separated foreground region.
    fn dark_shape_region_coverage(
        doc: &VectorDoc,
        regions: &[(f64, f64, f64, f64)],
    ) -> Vec<bool> {
        let mut found = vec![false; regions.len()];
        for shape in &doc.shapes {
            let color = shape.paint.color();
            if color.r >= 125 || color.g >= 125 || color.b >= 125 || color.a == 0 {
                continue;
            }
            for subpath in &shape.path.subpaths {
                for command in &subpath.commands {
                    let mut mark = |x: f64, y: f64| {
                        for (hit, &(min_x, max_x, min_y, max_y)) in
                            found.iter_mut().zip(regions)
                        {
                            if x >= min_x && x <= max_x && y >= min_y && y <= max_y {
                                *hit = true;
                            }
                        }
                    };
                    match command {
                        PathCmd::MoveTo(point) | PathCmd::LineTo(point) => {
                            mark(point.x, point.y);
                        }
                        PathCmd::CubicTo(a, b, c) => {
                            mark(a.x, a.y);
                            mark(b.x, b.y);
                            mark(c.x, c.y);
                        }
                        PathCmd::Close => {}
                    }
                }
            }
        }
        found
    }

    fn logo_preset() -> TraceParams {
        TraceParams {
            color_precision: 4,
            filter_speckle: 8,
            layer_difference: 32,
            corner_threshold: 60,
            length_threshold: 4.0,
            mode: TraceMode::Spline,
            hierarchical: HierarchicalMode::Stacked,
        }
    }

    #[test]
    fn logo_preset_retains_all_eight_separated_opaque_shapes() -> Result<(), String> {
        let source = RgbaImage::from_fn(392, 120, |x, y| {
            let foreground = (0..8).any(|index| {
                let left = 20 + index * 44;
                x >= left && x < left + 26 && (30..84).contains(&y)
            });
            if foreground {
                Rgba([22, 25, 29, 255])
            } else {
                Rgba([249, 248, 245, 255])
            }
        });
        let doc = trace_image(source, &logo_preset(), &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "unexpected empty trace".to_owned())?;
        let regions: Vec<(f64, f64, f64, f64)> = (0..8)
            .map(|index| {
                let x = (20 + 44 * index) as f64;
                (x - 3.0, x + 29.0, 27.0, 87.0)
            })
            .collect();
        let covered = dark_shape_region_coverage(&doc, &regions);
        assert!(covered.iter().all(|value| *value), "missing shapes: {covered:?}");
        Ok(())
    }

    #[test]
    #[ignore = "Set VECTORFORGE_QUALITY_IMAGE to a local, authorized 1536x1024 Gallery Computer PNG"]
    fn gallery_computer_wordmark_retains_all_seven_dark_letters() -> Result<(), String> {
        let path = std::env::var("VECTORFORGE_QUALITY_IMAGE")
            .map_err(|_| "VECTORFORGE_QUALITY_IMAGE not set".to_owned())?;
        let source = image::open(path)
            .map_err(|error| error.to_string())?
            .to_rgba8();
        if source.dimensions() != (1536, 1024) {
            return Err(format!("expected 1536x1024, got {:?}", source.dimensions()));
        }
        let doc = trace_image(source, &logo_preset(), &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "unexpected empty trace".to_owned())?;
        // Seven letter bounds are measured on the user-supplied 1536x1024
        // source. The image remains local; no branded fixture is committed.
        let regions = [
            (302.0, 447.0, 577.0, 718.0),
            (469.0, 605.0, 581.0, 715.0),
            (638.0, 721.0, 581.0, 715.0),
            (756.0, 838.0, 581.0, 715.0),
            (874.0, 956.0, 581.0, 715.0),
            (999.0, 1102.0, 581.0, 715.0),
            (1117.0, 1236.0, 581.0, 715.0),
        ];
        let covered = dark_shape_region_coverage(&doc, &regions);
        assert!(covered.iter().all(|value| *value), "missing wordmark letters: {covered:?}");
        Ok(())
    }

    #[test]
    fn fully_transparent_preview_skips_tracer() -> Result<(), String> {
        let image = RgbaImage::from_pixel(16, 16, Rgba([10, 20, 30, 0]));
        let result =
            trace_image(image, &params(), &CancelToken::new()).map_err(|error| error.message)?;
        assert!(result.is_none());
        Ok(())
    }

    #[test]
    fn same_rgb_partial_alpha_survives_production_adapter() -> Result<(), String> {
        let image = RgbaImage::from_fn(32, 16, |x, _| {
            let alpha = if x < 16 { 128 } else { 255 };
            Rgba([40, 120, 220, alpha])
        });
        let doc = trace_image(image, &params(), &CancelToken::new())
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
