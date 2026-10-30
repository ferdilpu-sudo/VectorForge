use std::panic::{AssertUnwindSafe, catch_unwind};

use image::RgbaImage;
use vtracer::{
    CancelToken, Clustering as VTracerClustering, ColorImage, Config, Error as VTracerError,
    FitMode, Hierarchical as VTracerHierarchical, VectorDoc,
};

use crate::models::{
    AppError, ErrorCode, GeometryMode, HierarchicalMode, SegmentationMode, TraceMode, TraceParams,
};

use super::alpha::split_by_source_alpha;
use super::geometry::PreserveStraightEdges;
use super::quality_guard::GuardedContourPass;

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
    let mut config = config_from_params(params);
    // With the guard enabled, simplification must run inside the guarded pass,
    // not as an independent unguarded curve pass.
    if params.quality_guard && params.mode == TraceMode::Spline {
        config.simplify = None;
    }
    let mut pipeline = config.build().map_err(vtracer_error)?;
    if params.quality_guard && params.mode == TraceMode::Spline {
        pipeline.curve_passes.push(Box::new(GuardedContourPass {
            simplify_tolerance: params.smoothness,
            corner_threshold_radians: f64::from(params.corner_threshold).to_radians(),
            line_aware: params.geometry_mode == GeometryMode::LineAware,
            max_deviation: 0.6,
            max_mean_deviation: 0.2,
        }));
    } else if params.geometry_mode == GeometryMode::LineAware && params.mode == TraceMode::Spline {
        // Run after the ordinary curve simplifier, still before composition.
        // VTracer applies curve passes to shared boundaries only once in cutout mode.
        pipeline.curve_passes.push(Box::new(PreserveStraightEdges {
            tolerance: 0.6,
            min_length: 8.0,
        }));
    }

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
        clustering: match params.segmentation_mode {
            SegmentationMode::ColorCluster => VTracerClustering::ColorCluster,
            SegmentationMode::Watershed => VTracerClustering::Watershed,
        },
        max_colors: if params.palette_limit == 0 {
            None
        } else {
            Some(usize::from(params.palette_limit))
        },
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
        simplify: if params.mode == TraceMode::Spline && params.smoothness > 0.0 {
            Some(params.smoothness)
        } else {
            None
        },
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
    use vtracer::{
        CancelToken, Clustering as VTracerClustering, Hierarchical as VTracerHierarchical, VectorDoc,
    };

    use crate::models::{
        GeometryMode, HierarchicalMode, SegmentationMode, TraceMode, TraceParams,
    };

    use super::{config_from_params, trace_image};

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
        assert_eq!(config.simplify, None);
        assert_eq!(config.clustering, VTracerClustering::ColorCluster);
        assert_eq!(config.max_colors, None);
    }

    #[test]
    fn professional_segmentation_and_palette_are_explicit() {
        let mut params = params();
        params.segmentation_mode = SegmentationMode::Watershed;
        params.palette_limit = 24;
        params.hierarchical = HierarchicalMode::Cutout;
        let config = config_from_params(&params);
        assert_eq!(config.clustering, VTracerClustering::Watershed);
        assert_eq!(config.hierarchical, VTracerHierarchical::Cutout);
        assert_eq!(config.max_colors, Some(24));
    }

    #[test]
    fn smoothness_only_enables_spline_curve_simplification() {
        let mut params = params();
        params.smoothness = 1.5;
        assert_eq!(config_from_params(&params).simplify, Some(1.5));

        params.mode = TraceMode::Polygon;
        assert_eq!(config_from_params(&params).simplify, None);
    }

    // A path_count > 0 assertion cannot detect a logo reduced to one glyph.
    // Require visible dark geometry within every separated foreground region.
    fn painted_region_coverage(
        doc: &VectorDoc,
        regions: &[(f64, f64, f64, f64)],
        accept_color: impl Fn(u8, u8, u8, u8) -> bool,
    ) -> Vec<bool> {
        let mut found = vec![false; regions.len()];
        for shape in &doc.shapes {
            let color = shape.paint.color();
            if !accept_color(color.r, color.g, color.b, color.a) {
                continue;
            }
            for subpath in &shape.path.subpaths {
                for command in &subpath.commands {
                    let mut mark = |x: f64, y: f64| {
                        for (hit, &(min_x, max_x, min_y, max_y)) in found.iter_mut().zip(regions) {
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
            smoothness: 0.0,
            segmentation_mode: crate::models::SegmentationMode::ColorCluster,
            palette_limit: 0,
            geometry_mode: crate::models::GeometryMode::Classic,
            quality_guard: false,
            mode: TraceMode::Spline,
            hierarchical: HierarchicalMode::Stacked,
        }
    }

    #[test]
    fn professional_modes_keep_separated_opaque_shapes() -> Result<(), String> {
        let source = RgbaImage::from_fn(180, 100, |x, y| {
            let foreground = (0..3).any(|index| {
                let left = 15 + index * 55;
                x >= left && x < left + 28 && (24..78).contains(&y)
            });
            if foreground {
                Rgba([20, 24, 29, 255])
            } else {
                Rgba([250, 247, 241, 255])
            }
        });
        let regions: Vec<(f64, f64, f64, f64)> = (0..3)
            .map(|index| {
                let left = (15 + index * 55) as f64;
                (left - 3.0, left + 31.0, 21.0, 81.0)
            })
            .collect();

        for segmentation_mode in [
            SegmentationMode::ColorCluster,
            SegmentationMode::Watershed,
        ] {
            let mut options = params();
            options.segmentation_mode = segmentation_mode;
            options.hierarchical = HierarchicalMode::Cutout;
            options.palette_limit = 24;

            let doc = trace_image(source.clone(), &options, &CancelToken::new())
                .map_err(|error| error.message)?
                .ok_or_else(|| "unexpected empty trace".to_owned())?;
            let coverage = painted_region_coverage(&doc, &regions, |r, g, b, a| {
                r < 125 && g < 125 && b < 125 && a > 0
            });
            assert!(
                coverage.iter().all(|hit| *hit),
                "missing foreground shape with {segmentation_mode:?}: {coverage:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn watershed_cutout_keeps_partially_transparent_and_opaque_shapes() -> Result<(), String> {
        let source = RgbaImage::from_fn(144, 96, |x, y| {
            if (10..61).contains(&x) && (10..84).contains(&y) {
                Rgba([45, 70, 130, 128])
            } else if (82..134).contains(&x) && (10..84).contains(&y) {
                Rgba([220, 75, 30, 255])
            } else {
                Rgba([0, 0, 0, 0])
            }
        });
        let mut options = params();
        options.segmentation_mode = SegmentationMode::Watershed;
        options.hierarchical = HierarchicalMode::Cutout;
        options.palette_limit = 24;

        let doc = trace_image(source, &options, &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "unexpected empty trace".to_owned())?;
        let alphas: Vec<u8> = doc.shapes.iter().map(|shape| shape.paint.color().a).collect();
        assert!(alphas.contains(&128), "semi-transparent shape lost: {alphas:?}");
        assert!(alphas.contains(&255), "opaque shape lost: {alphas:?}");
        Ok(())
    }

    #[test]
    fn line_aware_logo_keeps_separated_foreground_regions() -> Result<(), String> {
        let source = RgbaImage::from_fn(220, 92, |x, y| {
            let dark = (0..4).any(|index| {
                let left = 16 + index * 52;
                x >= left && x < left + 25 && (20..74).contains(&y)
            });
            if dark {
                Rgba([28, 31, 35, 255])
            } else {
                Rgba([248, 246, 241, 255])
            }
        });
        let mut params = logo_preset();
        params.geometry_mode = GeometryMode::LineAware;
        params.hierarchical = HierarchicalMode::Cutout;
        let doc = trace_image(source, &params, &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "unexpected empty trace".to_owned())?;
        let regions: Vec<(f64, f64, f64, f64)> = (0..4)
            .map(|index| {
                let x = (16 + index * 52) as f64;
                (x - 3.0, x + 30.0, 17.0, 77.0)
            })
            .collect();
        let hit = painted_region_coverage(&doc, &regions, |r, g, b, a| {
            r < 125 && g < 125 && b < 125 && a > 0
        });
        assert!(hit.iter().all(|value| *value), "missing letter-like regions: {hit:?}");
        Ok(())
    }

    #[test]
    fn guarded_polygon_tracing_matches_unguarded_output() -> Result<(), String> {
        let source = RgbaImage::from_fn(96, 64, |x, y| {
            if (10..70).contains(&x) && (10..46).contains(&y) {
                Rgba([20, 25, 30, 255])
            } else {
                Rgba([245, 243, 239, 255])
            }
        });
        let mut options = params();
        options.mode = TraceMode::Polygon;
        options.geometry_mode = GeometryMode::LineAware;
        let classic = trace_image(source.clone(), &options, &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "missing classic output".to_owned())?;
        options.quality_guard = true;
        let guarded = trace_image(source, &options, &CancelToken::new())
            .map_err(|error| error.message)?
            .ok_or_else(|| "missing guarded output".to_owned())?;
        assert_eq!(
            crate::engine::write_alpha_svg(&classic).svg,
            crate::engine::write_alpha_svg(&guarded).svg
        );
        Ok(())
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
        let covered = painted_region_coverage(&doc, &regions, |r, g, b, a| {
            r < 125 && g < 125 && b < 125 && a > 0
        });
        assert!(
            covered.iter().all(|value| *value),
            "missing shapes: {covered:?}"
        );
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
        let covered = painted_region_coverage(&doc, &regions, |r, g, b, a| {
            r < 125 && g < 125 && b < 125 && a > 0
        });
        assert!(
            covered.iter().all(|value| *value),
            "missing GALLERY letters: {covered:?}"
        );

        // Check the second, gold wordmark separately.
        let gold_regions = [
            (436.0, 492.0, 746.0, 805.0),
            (528.0, 588.0, 746.0, 805.0),
            (626.0, 686.0, 747.0, 804.0),
            (728.0, 771.0, 747.0, 804.0),
            (808.0, 855.0, 747.0, 805.0),
            (893.0, 938.0, 747.0, 804.0),
            (977.0, 1015.0, 747.0, 804.0),
            (1056.0, 1101.0, 747.0, 804.0),
        ];
        let gold_covered = painted_region_coverage(&doc, &gold_regions, |r, g, b, a| {
            a > 0 && r > 110 && i16::from(r) - i16::from(g) > 12 && i16::from(g) - i16::from(b) > 14
        });
        assert!(
            gold_covered.iter().all(|value| *value),
            "missing COMPUTER letters: {gold_covered:?}"
        );
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
