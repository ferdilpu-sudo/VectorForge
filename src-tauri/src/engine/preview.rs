use std::time::Instant;

use vtracer::CancelToken;

use crate::files::{SourceSnapshot, probe_source};
use crate::models::{AppError, ErrorCode, PreviewRequest, PreviewResult, TraceStats};

use super::decode::decode_preview;
use super::svg::{empty_svg, write_alpha_svg};
use super::tracer::trace_preview;

const MAX_PREVIEW_SVG_BYTES: usize = 50 * 1024 * 1024;

pub struct PreviewWork {
    pub source: SourceSnapshot,
    pub request: PreviewRequest,
}

pub fn render_preview(work: PreviewWork, cancel: &CancelToken) -> Result<PreviewResult, AppError> {
    work.request.validate()?;
    check_cancel(cancel)?;

    let started = Instant::now();
    let before = probe_source(&work.source.path)?;
    if before.fingerprint != work.source.fingerprint {
        return Err(source_changed_error());
    }

    let image = decode_preview(&work.source.path, before.orientation, work.request.max_side)?;
    let width = image.width();
    let height = image.height();

    check_cancel(cancel)?;
    let output = match trace_preview(image, &work.request.params, cancel)? {
        Some(doc) => write_alpha_svg(&doc),
        None => empty_svg(width, height),
    };

    validate_preview_svg_size(output.svg.len())?;

    check_cancel(cancel)?;
    let after = probe_source(&work.source.path)?;
    if after.fingerprint != work.source.fingerprint {
        return Err(source_changed_error());
    }

    let svg_bytes = u64::try_from(output.svg.len()).map_err(|error| {
        AppError::invalid_state("Ukuran SVG preview tidak valid.", error.to_string())
    })?;
    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    Ok(PreviewResult {
        request_id: work.request.request_id,
        file_id: work.request.file_id,
        svg: output.svg,
        stats: TraceStats {
            path_count: output.path_count,
            color_count: output.color_count,
            svg_bytes,
            width,
            height,
        },
        elapsed_ms,
    })
}

fn validate_preview_svg_size(bytes: usize) -> Result<(), AppError> {
    if bytes > MAX_PREVIEW_SVG_BYTES {
        Err(AppError::new(
            ErrorCode::OutputTooLarge,
            "Preview SVG melebihi batas 50 MiB.",
        ))
    } else {
        Ok(())
    }
}

fn check_cancel(cancel: &CancelToken) -> Result<(), AppError> {
    if cancel.is_cancelled() {
        Err(AppError::new(ErrorCode::Cancelled, "Preview dibatalkan."))
    } else {
        Ok(())
    }
}

fn source_changed_error() -> AppError {
    AppError::new(
        ErrorCode::SourceChanged,
        "File sumber berubah sejak diimpor.",
    )
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};

    use image::{Rgba, RgbaImage};
    use uuid::Uuid;
    use vtracer::CancelToken;

    use crate::files::{SourceSnapshot, probe_source};
    use crate::models::{ErrorCode, HierarchicalMode, PreviewRequest, TraceMode, TraceParams};

    use super::{MAX_PREVIEW_SVG_BYTES, PreviewWork, render_preview, validate_preview_svg_size};

    struct TempPng {
        path: PathBuf,
    }

    impl TempPng {
        fn create(image: &RgbaImage) -> Result<Self, String> {
            let path = std::env::temp_dir().join(format!("vectorforge-b03-{}.png", Uuid::new_v4()));
            image.save(&path).map_err(|error| error.to_string())?;
            Ok(Self { path })
        }

        fn overwrite(&self, image: &RgbaImage) -> Result<(), String> {
            image.save(&self.path).map_err(|error| error.to_string())
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempPng {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.path);
        }
    }

    fn request() -> PreviewRequest {
        PreviewRequest {
            file_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            request_id: "00000000-0000-4000-8000-000000000002".to_owned(),
            max_side: 1024,
            params: TraceParams {
                color_precision: 6,
                filter_speckle: 4,
                layer_difference: 16,
                corner_threshold: 60,
                length_threshold: 4.0,
                mode: TraceMode::Spline,
                hierarchical: HierarchicalMode::Stacked,
            },
        }
    }

    #[test]
    fn real_png_fixture_renders_vector_preview_with_stats() -> Result<(), String> {
        let fixture = TempPng::create(&RgbaImage::from_fn(96, 64, |x, _| {
            if x < 48 {
                Rgba([220, 40, 60, 255])
            } else {
                Rgba([30, 100, 220, 255])
            }
        }))?;

        let probe = probe_source(fixture.path()).map_err(|error| error.message)?;
        let result = render_preview(
            PreviewWork {
                source: SourceSnapshot {
                    path: fixture.path().to_path_buf(),
                    fingerprint: probe.fingerprint,
                },
                request: request(),
            },
            &CancelToken::new(),
        )
        .map_err(|error| error.message)?;

        if result.stats.width != 96
            || result.stats.height != 64
            || result.stats.path_count == 0
            || result.stats.color_count < 2
            || !result.svg.contains("<path")
            || result.svg.contains("<image")
        {
            return Err(format!("unexpected preview result: {:?}", result.stats));
        }

        Ok(())
    }

    #[test]
    fn changed_source_is_rejected_before_preview() -> Result<(), String> {
        let fixture = TempPng::create(&RgbaImage::from_pixel(32, 32, Rgba([20, 40, 60, 255])))?;
        let original = probe_source(fixture.path()).map_err(|error| error.message)?;

        fixture.overwrite(&RgbaImage::from_pixel(32, 32, Rgba([200, 180, 20, 255])))?;

        let result = render_preview(
            PreviewWork {
                source: SourceSnapshot {
                    path: fixture.path().to_path_buf(),
                    fingerprint: original.fingerprint,
                },
                request: request(),
            },
            &CancelToken::new(),
        );

        assert!(matches!(
            result,
            Err(error) if error.code == ErrorCode::SourceChanged
        ));
        Ok(())
    }

    #[test]
    fn preview_svg_limit_accepts_boundary_and_rejects_next_byte() {
        assert!(validate_preview_svg_size(MAX_PREVIEW_SVG_BYTES).is_ok());
        assert!(matches!(
            validate_preview_svg_size(MAX_PREVIEW_SVG_BYTES + 1),
            Err(error) if error.code == ErrorCode::OutputTooLarge
        ));
    }
}
