use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use image::RgbaImage;
use vtracer::CancelToken;

use crate::engine::{decode_full, empty_svg, trace_image, write_alpha_svg};
use crate::files::{DestinationSnapshot, SourceSnapshot, probe_source};
use crate::models::{
    AppError, DestinationKind, ErrorCode, ExportFormat, ExportRequest, TraceStats,
};

use super::eps::write_eps;
use super::fs::write_atomic;
use super::pdf::svg_to_pdf;

const MAX_SVG_BYTES: usize = 50 * 1024 * 1024;

pub struct ExportWork {
    pub source: SourceSnapshot,
    pub destination: DestinationSnapshot,
    pub request: ExportRequest,
}

pub struct CommittedExport {
    pub path: PathBuf,
    pub format: ExportFormat,
    pub bytes: u64,
    pub stats: TraceStats,
    pub elapsed_ms: u64,
}

pub fn export_work(work: ExportWork) -> Result<CommittedExport, AppError> {
    work.request.validate()?;
    validate_destination(&work.destination, work.request.format)?;
    validate_not_source(&work.source.path, &work.destination.path)?;

    let started = Instant::now();
    let before = probe_source(&work.source.path)?;
    if before.fingerprint != work.source.fingerprint {
        return Err(source_changed_error());
    }

    let image = decode_full(&work.source.path, before.orientation)?;
    let width = image.width();
    let height = image.height();

    let after_decode = probe_source(&work.source.path)?;
    if after_decode.fingerprint != work.source.fingerprint {
        return Err(source_changed_error());
    }

    let trace_input = match work.request.format {
        ExportFormat::Eps => flatten_on_white(image),
        ExportFormat::Svg | ExportFormat::Pdf => image,
    };

    let document = trace_image(trace_input, &work.request.params, &CancelToken::new())?;
    let svg_output = match document.as_ref() {
        Some(document) => write_alpha_svg(document),
        None => empty_svg(width, height),
    };

    validate_svg_size(
        svg_output.svg.len(),
        work.request.format,
        work.request.allow_large_output,
    )?;

    let svg_bytes = u64::try_from(svg_output.svg.len()).map_err(|error| {
        AppError::invalid_state("Ukuran SVG export tidak valid.", error.to_string())
    })?;
    let stats = TraceStats {
        path_count: svg_output.path_count,
        color_count: svg_output.color_count,
        svg_bytes,
        width,
        height,
    };

    let payload = match work.request.format {
        ExportFormat::Svg => svg_output.svg.into_bytes(),
        ExportFormat::Pdf => svg_to_pdf(&svg_output.svg)?,
        ExportFormat::Eps => {
            let document = document.as_ref().ok_or_else(|| {
                AppError::new(
                    ErrorCode::ExportFailed,
                    "Trace EPS tidak menghasilkan dokumen setelah komposit putih.",
                )
            })?;
            write_eps(document)?
        }
    };

    let bytes = write_atomic(
        &work.destination.path,
        &payload,
        work.destination.overwrite_confirmed,
    )?;
    let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    Ok(CommittedExport {
        path: work.destination.path,
        format: work.request.format,
        bytes,
        stats,
        elapsed_ms,
    })
}

fn validate_destination(
    destination: &DestinationSnapshot,
    format: ExportFormat,
) -> Result<(), AppError> {
    if destination.kind != DestinationKind::File {
        return Err(AppError::new(
            ErrorCode::InvalidParams,
            "Export tunggal membutuhkan tujuan file.",
        ));
    }

    if destination.format != Some(format) {
        return Err(AppError::new(
            ErrorCode::InvalidParams,
            "Format export tidak cocok dengan tujuan yang dipilih.",
        ));
    }

    Ok(())
}

fn validate_not_source(source: &Path, destination: &Path) -> Result<(), AppError> {
    if !destination.exists() {
        return Ok(());
    }

    let canonical_source = fs::canonicalize(source).map_err(|error| {
        AppError::with_details(
            ErrorCode::FileNotFound,
            "File sumber gagal diverifikasi.",
            error.to_string(),
        )
    })?;
    let canonical_destination = fs::canonicalize(destination).map_err(|error| {
        AppError::with_details(
            ErrorCode::WriteFailed,
            "File tujuan gagal diverifikasi.",
            error.to_string(),
        )
    })?;

    if canonical_destination == canonical_source {
        Err(AppError::new(
            ErrorCode::WriteFailed,
            "File sumber tidak boleh ditimpa oleh hasil export.",
        ))
    } else {
        Ok(())
    }
}

fn validate_svg_size(
    bytes: usize,
    format: ExportFormat,
    allow_large_output: bool,
) -> Result<(), AppError> {
    if format == ExportFormat::Svg && bytes > MAX_SVG_BYTES && !allow_large_output {
        Err(AppError::new(
            ErrorCode::OutputTooLarge,
            "SVG melebihi 50 MiB dan membutuhkan konfirmasi eksplisit.",
        ))
    } else {
        Ok(())
    }
}

fn flatten_on_white(mut image: RgbaImage) -> RgbaImage {
    for pixel in image.pixels_mut() {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel.0[..3] {
            let source = u16::from(*channel);
            let blended = source
                .saturating_mul(alpha)
                .saturating_add(255u16.saturating_mul(255 - alpha))
                .saturating_add(127)
                / 255;
            *channel = blended as u8;
        }
        pixel[3] = 255;
    }
    image
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

    use crate::files::{DestinationSnapshot, SourceSnapshot, probe_source};
    use crate::models::{
        DestinationKind, ErrorCode, ExportFormat, ExportRequest, HierarchicalMode, TraceMode,
        TraceParams,
    };

    use super::{ExportWork, MAX_SVG_BYTES, export_work, flatten_on_white, validate_svg_size};

    struct TempFixture {
        dir: PathBuf,
        source: PathBuf,
    }

    impl TempFixture {
        fn create(image: &RgbaImage) -> Result<Self, String> {
            let dir =
                std::env::temp_dir().join(format!("vectorforge-b04-export-{}", Uuid::new_v4()));
            fs::create_dir(&dir).map_err(|error| error.to_string())?;
            let source = dir.join("source.png");
            image.save(&source).map_err(|error| error.to_string())?;
            Ok(Self { dir, source })
        }

        fn output(&self, name: &str) -> PathBuf {
            self.dir.join(name)
        }
    }

    impl Drop for TempFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

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

    fn work(
        fixture: &TempFixture,
        format: ExportFormat,
        output: &Path,
    ) -> Result<ExportWork, String> {
        let probe = probe_source(&fixture.source).map_err(|error| error.message)?;
        Ok(ExportWork {
            source: SourceSnapshot {
                path: fixture.source.clone(),
                fingerprint: probe.fingerprint,
            },
            destination: DestinationSnapshot {
                path: output.to_path_buf(),
                kind: DestinationKind::File,
                format: Some(format),
                overwrite_confirmed: false,
            },
            request: ExportRequest {
                file_id: "00000000-0000-4000-8000-000000000001".to_owned(),
                params: params(),
                format,
                destination_id: "00000000-0000-4000-8000-000000000002".to_owned(),
                allow_large_output: false,
            },
        })
    }

    #[test]
    fn svg_export_is_full_size_vector_and_preserves_alpha() -> Result<(), String> {
        let fixture = TempFixture::create(&RgbaImage::from_fn(96, 48, |x, _| {
            let alpha = if x < 48 { 128 } else { 255 };
            Rgba([40, 120, 220, alpha])
        }))?;
        let output = fixture.output("result.svg");
        let result = export_work(work(&fixture, ExportFormat::Svg, &output)?)
            .map_err(|error| error.message)?;
        let svg = fs::read_to_string(&output).map_err(|error| error.to_string())?;

        assert_eq!(result.stats.width, 96);
        assert_eq!(result.stats.height, 48);
        assert!(result.stats.path_count > 0);
        assert!(svg.contains("<path"));
        assert!(svg.contains("fill-opacity="));
        assert!(!svg.contains("<image"));
        assert_eq!(
            result.bytes,
            fs::metadata(output).map_err(|error| error.to_string())?.len()
        );
        Ok(())
    }

    #[test]
    fn pdf_export_is_vector_pdf_with_full_source_bounds() -> Result<(), String> {
        let fixture = TempFixture::create(&RgbaImage::from_fn(80, 40, |x, _| {
            if x < 40 {
                Rgba([220, 40, 60, 128])
            } else {
                Rgba([30, 100, 220, 255])
            }
        }))?;
        let output = fixture.output("result.pdf");
        let result = export_work(work(&fixture, ExportFormat::Pdf, &output)?)
            .map_err(|error| error.message)?;
        let pdf = fs::read(&output).map_err(|error| error.to_string())?;

        assert!(pdf.starts_with(b"%PDF-"));
        assert_eq!((result.stats.width, result.stats.height), (80, 40));
        assert!(result.stats.path_count > 0);
        Ok(())
    }

    #[test]
    fn eps_export_flattens_transparency_to_white_before_trace() -> Result<(), String> {
        let fixture = TempFixture::create(&RgbaImage::from_fn(64, 32, |x, _| {
            if x < 32 {
                Rgba([20, 40, 60, 128])
            } else {
                Rgba([200, 60, 40, 255])
            }
        }))?;
        let output = fixture.output("result.eps");
        let result = export_work(work(&fixture, ExportFormat::Eps, &output)?)
            .map_err(|error| error.message)?;
        let eps = fs::read_to_string(&output).map_err(|error| error.to_string())?;

        assert!(eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0"));
        assert!(eps.contains("%%BoundingBox: 0 0 64 32"));
        assert!(eps.contains("setrgbcolor"));
        assert!(!eps.contains("fill-opacity"));
        assert_eq!((result.stats.width, result.stats.height), (64, 32));
        Ok(())
    }

    #[test]
    fn export_refuses_to_overwrite_the_source_file() -> Result<(), String> {
        let fixture =
            TempFixture::create(&RgbaImage::from_pixel(32, 16, Rgba([20, 40, 60, 255])))?;
        let original = fs::read(&fixture.source).map_err(|error| error.to_string())?;
        let probe = probe_source(&fixture.source).map_err(|error| error.message)?;

        let result = export_work(ExportWork {
            source: SourceSnapshot {
                path: fixture.source.clone(),
                fingerprint: probe.fingerprint,
            },
            destination: DestinationSnapshot {
                path: fixture.source.clone(),
                kind: DestinationKind::File,
                format: Some(ExportFormat::Svg),
                overwrite_confirmed: true,
            },
            request: ExportRequest {
                file_id: "00000000-0000-4000-8000-000000000001".to_owned(),
                params: params(),
                format: ExportFormat::Svg,
                destination_id: "00000000-0000-4000-8000-000000000002".to_owned(),
                allow_large_output: false,
            },
        });

        assert!(matches!(
            result,
            Err(error) if error.code == ErrorCode::WriteFailed
        ));
        assert_eq!(
            fs::read(&fixture.source).map_err(|error| error.to_string())?,
            original
        );
        Ok(())
    }

    #[test]
    fn large_svg_requires_explicit_confirmation_only_for_svg() {
        assert!(matches!(
            validate_svg_size(MAX_SVG_BYTES + 1, ExportFormat::Svg, false),
            Err(error) if error.code == ErrorCode::OutputTooLarge
        ));
        assert!(validate_svg_size(MAX_SVG_BYTES + 1, ExportFormat::Svg, true).is_ok());
        assert!(validate_svg_size(MAX_SVG_BYTES + 1, ExportFormat::Pdf, false).is_ok());
    }

    #[test]
    fn white_flatten_is_opaque_and_blended() {
        let image = RgbaImage::from_pixel(1, 1, Rgba([0, 0, 0, 128]));
        let flattened = flatten_on_white(image);
        let pixel = flattened.get_pixel(0, 0);
        assert_eq!(pixel[3], 255);
        assert_eq!(pixel[0], 127);
        assert_eq!(pixel[1], 127);
        assert_eq!(pixel[2], 127);
    }
}
