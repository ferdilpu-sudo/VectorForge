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

pub fn render_preview(
    work: PreviewWork,
    cancel: &CancelToken,
) -> Result<PreviewResult, AppError> {
    work.request.validate()?;
    check_cancel(cancel)?;

    let started = Instant::now();
    let before = probe_source(&work.source.path)?;
    if before.fingerprint != work.source.fingerprint {
        return Err(source_changed_error());
    }

    let image = decode_preview(
        &work.source.path,
        before.orientation,
        work.request.max_side,
    )?;
    let width = image.width();
    let height = image.height();

    check_cancel(cancel)?;
    let output = match trace_preview(image, &work.request.params, cancel)? {
        Some(doc) => write_alpha_svg(&doc),
        None => empty_svg(width, height),
    };

    if output.svg.len() > MAX_PREVIEW_SVG_BYTES {
        return Err(AppError::new(
            ErrorCode::OutputTooLarge,
            "Preview SVG melebihi batas 50 MiB.",
        ));
    }

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

fn check_cancel(cancel: &CancelToken) -> Result<(), AppError> {
    if cancel.is_cancelled() {
        Err(AppError::new(
            ErrorCode::Cancelled,
            "Preview dibatalkan.",
        ))
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
