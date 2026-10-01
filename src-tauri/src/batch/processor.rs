use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use vtracer::CancelToken;

use crate::engine::{WorkGate, decode_full, empty_svg, trace_image, write_alpha_svg};
use crate::export::{
    flatten_on_white, reserve_batch_output, svg_to_pdf, validate_svg_size, write_eps,
};
use crate::files::{FileRegistry, SourceSnapshot, probe_source};
use crate::models::{
    AppError, BatchOutput, ErrorCode, ExportFormat, JobStage, OutputStatus, TraceParams,
};

#[derive(Clone)]
pub struct BatchItemWork {
    pub file_id: String,
    pub name: String,
    pub source: SourceSnapshot,
    pub params: TraceParams,
    pub formats: Vec<ExportFormat>,
    pub output_dir: PathBuf,
    pub output_stem: String,
    pub overwrite: bool,
}

pub struct ItemProcessResult {
    pub outputs: Vec<BatchOutput>,
    pub item_error: Option<AppError>,
    pub cancelled: bool,
    pub elapsed_ms: u64,
}

pub fn process_item<F>(
    work: BatchItemWork,
    cancel: &CancelToken,
    gate: &Arc<WorkGate>,
    registry: &Arc<Mutex<FileRegistry>>,
    mut on_stage: F,
) -> ItemProcessResult
where
    F: FnMut(JobStage, Option<ExportFormat>),
{
    let started = Instant::now();
    let mut outputs = work
        .formats
        .iter()
        .copied()
        .map(queued_output)
        .collect::<Vec<_>>();

    let permit = match gate.acquire_normal() {
        Ok(permit) => permit,
        Err(error) => {
            fail_queued(&mut outputs, error.clone());
            return finish(outputs, Some(error), false, started);
        }
    };

    if cancel.is_cancelled() {
        cancel_queued(&mut outputs);
        drop(permit);
        return finish(outputs, None, true, started);
    }

    let stem = work.output_stem.clone();
    let vector_formats = work
        .formats
        .iter()
        .copied()
        .filter(|format| matches!(format, ExportFormat::Svg | ExportFormat::Pdf))
        .collect::<Vec<_>>();

    let mut item_error = None;

    if !vector_formats.is_empty() {
        on_stage(JobStage::Decoding, None);
        match decode_verified(&work.source) {
            Ok(image) => {
                let width = image.width();
                let height = image.height();
                if cancel.is_cancelled() {
                    cancel_queued(&mut outputs);
                    drop(permit);
                    return finish(outputs, None, true, started);
                }

                on_stage(JobStage::Tracing, None);
                match trace_image(image, &work.params, cancel) {
                    Ok(document) => {
                        let svg = match document.as_ref() {
                            Some(doc) => write_alpha_svg(doc),
                            None => empty_svg(width, height),
                        };

                        for format in &vector_formats {
                            if cancel.is_cancelled() {
                                cancel_queued(&mut outputs);
                                drop(permit);
                                return finish(outputs, item_error, true, started);
                            }
                            on_stage(JobStage::Exporting, Some(*format));
                            let result = match format {
                                ExportFormat::Svg => {
                                    validate_svg_size(svg.svg.len(), ExportFormat::Svg, false)
                                        .and_then(|_| {
                                            commit_payload(
                                                &work,
                                                &stem,
                                                *format,
                                                svg.svg.as_bytes(),
                                                registry,
                                            )
                                        })
                                }
                                ExportFormat::Pdf => svg_to_pdf(&svg.svg).and_then(|payload| {
                                    commit_payload(&work, &stem, *format, &payload, registry)
                                }),
                                ExportFormat::Eps => Err(AppError::new(
                                    ErrorCode::InvalidState,
                                    "Format EPS masuk ke jalur export SVG/PDF.",
                                )),
                            };
                            apply_output_result(&mut outputs, *format, result);
                        }
                    }
                    Err(error) if error.code == ErrorCode::Cancelled => {
                        cancel_queued(&mut outputs);
                        drop(permit);
                        return finish(outputs, None, true, started);
                    }
                    Err(error) => {
                        fail_formats(&mut outputs, &vector_formats, error.clone());
                        item_error.get_or_insert(error);
                    }
                }
            }
            Err(error) => {
                fail_formats(&mut outputs, &vector_formats, error.clone());
                item_error.get_or_insert(error);
            }
        }
    }

    if work.formats.contains(&ExportFormat::Eps) {
        if cancel.is_cancelled() {
            cancel_queued(&mut outputs);
            drop(permit);
            return finish(outputs, item_error, true, started);
        }

        on_stage(JobStage::Decoding, None);
        match decode_verified(&work.source) {
            Ok(image) => {
                if cancel.is_cancelled() {
                    cancel_queued(&mut outputs);
                    drop(permit);
                    return finish(outputs, item_error, true, started);
                }

                on_stage(JobStage::Tracing, None);
                match trace_image(flatten_on_white(image), &work.params, cancel) {
                    Ok(Some(document)) => {
                        on_stage(JobStage::Exporting, Some(ExportFormat::Eps));
                        let result = write_eps(&document).and_then(|payload| {
                            commit_payload(&work, &stem, ExportFormat::Eps, &payload, registry)
                        });
                        apply_output_result(&mut outputs, ExportFormat::Eps, result);
                    }
                    Ok(None) => {
                        let error = AppError::new(
                            ErrorCode::ExportFailed,
                            "Trace EPS tidak menghasilkan dokumen.",
                        );
                        apply_output_result(&mut outputs, ExportFormat::Eps, Err(error.clone()));
                        item_error.get_or_insert(error);
                    }
                    Err(error) if error.code == ErrorCode::Cancelled => {
                        cancel_queued(&mut outputs);
                        drop(permit);
                        return finish(outputs, item_error, true, started);
                    }
                    Err(error) => {
                        apply_output_result(&mut outputs, ExportFormat::Eps, Err(error.clone()));
                        item_error.get_or_insert(error);
                    }
                }
            }
            Err(error) => {
                apply_output_result(&mut outputs, ExportFormat::Eps, Err(error.clone()));
                item_error.get_or_insert(error);
            }
        }
    }

    drop(permit);
    finish(outputs, item_error, false, started)
}

fn decode_verified(source: &SourceSnapshot) -> Result<image::RgbaImage, AppError> {
    let before = probe_source(&source.path)?;
    if before.fingerprint != source.fingerprint {
        return Err(source_changed());
    }

    let image = decode_full(&source.path, before.orientation)?;
    let after = probe_source(&source.path)?;
    if after.fingerprint != source.fingerprint {
        return Err(source_changed());
    }

    Ok(image)
}

fn commit_payload(
    work: &BatchItemWork,
    stem: &str,
    format: ExportFormat,
    payload: &[u8],
    registry: &Arc<Mutex<FileRegistry>>,
) -> Result<BatchOutput, AppError> {
    let reservation =
        reserve_batch_output(&work.output_dir, stem, format.extension(), work.overwrite)?;
    let path = reservation.path().to_path_buf();
    let output_id = {
        let mut registry = registry.lock().map_err(|error| {
            AppError::invalid_state("Registry output tidak dapat dikunci.", error.to_string())
        })?;
        registry.register_output(path.clone())?
    };

    match reservation.commit(payload) {
        Ok((path, bytes)) => Ok(BatchOutput {
            format,
            status: OutputStatus::Done,
            output_id: Some(output_id),
            out_path: Some(path.to_string_lossy().into_owned()),
            bytes: Some(bytes),
            error: None,
        }),
        Err(error) => {
            if let Ok(mut registry) = registry.lock() {
                registry.unregister_output(&output_id);
            }
            Err(error)
        }
    }
}

fn queued_output(format: ExportFormat) -> BatchOutput {
    BatchOutput {
        format,
        status: OutputStatus::Queued,
        output_id: None,
        out_path: None,
        bytes: None,
        error: None,
    }
}

fn apply_output_result(
    outputs: &mut [BatchOutput],
    format: ExportFormat,
    result: Result<BatchOutput, AppError>,
) {
    let Some(output) = outputs.iter_mut().find(|output| output.format == format) else {
        return;
    };

    match result {
        Ok(done) => *output = done,
        Err(error) => {
            output.status = OutputStatus::Failed;
            output.error = Some(error);
        }
    }
}

fn fail_formats(outputs: &mut [BatchOutput], formats: &[ExportFormat], error: AppError) {
    for output in outputs
        .iter_mut()
        .filter(|output| formats.contains(&output.format))
    {
        if output.status == OutputStatus::Queued {
            output.status = OutputStatus::Failed;
            output.error = Some(error.clone());
        }
    }
}

fn fail_queued(outputs: &mut [BatchOutput], error: AppError) {
    for output in outputs
        .iter_mut()
        .filter(|output| output.status == OutputStatus::Queued)
    {
        output.status = OutputStatus::Failed;
        output.error = Some(error.clone());
    }
}

fn cancel_queued(outputs: &mut [BatchOutput]) {
    for output in outputs.iter_mut().filter(|output| {
        matches!(
            output.status,
            OutputStatus::Queued | OutputStatus::Processing
        )
    }) {
        output.status = OutputStatus::Cancelled;
        output.error = None;
    }
}

fn source_changed() -> AppError {
    AppError::new(
        ErrorCode::SourceChanged,
        "File sumber berubah sejak diimpor.",
    )
}

fn finish(
    outputs: Vec<BatchOutput>,
    item_error: Option<AppError>,
    cancelled: bool,
    started: Instant,
) -> ItemProcessResult {
    ItemProcessResult {
        outputs,
        item_error,
        cancelled,
        elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    use image::{Rgba, RgbaImage};
    use uuid::Uuid;
    use vtracer::CancelToken;

    #[cfg(windows)]
    use std::fs::OpenOptions;
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;

    use crate::engine::WorkGate;
    use crate::files::{FileRegistry, SourceSnapshot, probe_source};
    use crate::models::{
        ExportFormat, HierarchicalMode, JobStage, OutputStatus, TraceMode, TraceParams,
    };

    use super::{BatchItemWork, process_item};

    struct TempFixture {
        dir: PathBuf,
        source: PathBuf,
    }

    impl TempFixture {
        fn create(name: &str) -> Result<Self, String> {
            let dir = std::env::temp_dir().join(format!("vectorforge-b06-item-{}", Uuid::new_v4()));
            fs::create_dir(&dir).map_err(|error| error.to_string())?;

            let source = dir.join(name);
            let image = RgbaImage::from_fn(64, 32, |x, _| {
                if x < 32 {
                    Rgba([40, 120, 220, 180])
                } else {
                    Rgba([220, 80, 60, 255])
                }
            });
            image.save(&source).map_err(|error| error.to_string())?;

            Ok(Self { dir, source })
        }

        fn work(
            &self,
            formats: Vec<ExportFormat>,
            overwrite: bool,
        ) -> Result<BatchItemWork, String> {
            let probe = probe_source(&self.source).map_err(|error| error.message)?;
            Ok(BatchItemWork {
                file_id: Uuid::new_v4().to_string(),
                name: self
                    .source
                    .file_name()
                    .and_then(|value| value.to_str())
                    .ok_or_else(|| "invalid fixture name".to_owned())?
                    .to_owned(),
                source: SourceSnapshot {
                    path: self.source.clone(),
                    fingerprint: probe.fingerprint,
                },
                params: params(),
                formats,
                output_dir: self.dir.clone(),
                output_stem: "logo".to_owned(),
                overwrite,
            })
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

    fn output_status(
        outputs: &[crate::models::BatchOutput],
        format: ExportFormat,
    ) -> Option<OutputStatus> {
        outputs
            .iter()
            .find(|output| output.format == format)
            .map(|output| output.status)
    }

    #[test]
    fn cancellation_after_first_export_keeps_committed_output() -> Result<(), String> {
        let fixture = TempFixture::create("logo.png")?;
        let cancel = CancelToken::new();
        let cancel_on_svg = cancel.clone();
        let gate = Arc::new(WorkGate::new(1));
        let registry = Arc::new(Mutex::new(FileRegistry::default()));

        let result = process_item(
            fixture.work(vec![ExportFormat::Svg, ExportFormat::Pdf], false)?,
            &cancel,
            &gate,
            &registry,
            move |stage, format| {
                if stage == JobStage::Exporting && format == Some(ExportFormat::Svg) {
                    cancel_on_svg.cancel();
                }
            },
        );

        assert!(result.cancelled);
        assert_eq!(
            output_status(&result.outputs, ExportFormat::Svg),
            Some(OutputStatus::Done)
        );
        assert_eq!(
            output_status(&result.outputs, ExportFormat::Pdf),
            Some(OutputStatus::Cancelled)
        );
        assert!(fixture.dir.join("logo.svg").is_file());
        assert!(!fixture.dir.join("logo.pdf").exists());
        Ok(())
    }

    #[test]
    fn repeated_same_name_uses_suffix_without_overwrite() -> Result<(), String> {
        let fixture = TempFixture::create("logo.png")?;
        let gate = Arc::new(WorkGate::new(1));
        let registry = Arc::new(Mutex::new(FileRegistry::default()));

        let first = process_item(
            fixture.work(vec![ExportFormat::Svg], false)?,
            &CancelToken::new(),
            &gate,
            &registry,
            |_, _| {},
        );
        let second = process_item(
            fixture.work(vec![ExportFormat::Svg], false)?,
            &CancelToken::new(),
            &gate,
            &registry,
            |_, _| {},
        );

        assert_eq!(
            output_status(&first.outputs, ExportFormat::Svg),
            Some(OutputStatus::Done)
        );
        assert_eq!(
            output_status(&second.outputs, ExportFormat::Svg),
            Some(OutputStatus::Done)
        );
        assert!(fixture.dir.join("logo.svg").is_file());
        assert!(fixture.dir.join("logo (2).svg").is_file());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn svg_success_and_locked_pdf_failure_preserve_partial_outputs() -> Result<(), String> {
        let fixture = TempFixture::create("logo.png")?;
        let pdf_path = fixture.dir.join("logo.pdf");
        fs::write(&pdf_path, b"keep-old-pdf").map_err(|error| error.to_string())?;

        let locked = OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&pdf_path)
            .map_err(|error| error.to_string())?;

        let gate = Arc::new(WorkGate::new(1));
        let registry = Arc::new(Mutex::new(FileRegistry::default()));
        let result = process_item(
            fixture.work(vec![ExportFormat::Svg, ExportFormat::Pdf], true)?,
            &CancelToken::new(),
            &gate,
            &registry,
            |_, _| {},
        );

        assert_eq!(
            output_status(&result.outputs, ExportFormat::Svg),
            Some(OutputStatus::Done)
        );
        assert_eq!(
            output_status(&result.outputs, ExportFormat::Pdf),
            Some(OutputStatus::Failed)
        );
        assert!(fixture.dir.join("logo.svg").is_file());

        drop(locked);
        assert_eq!(
            fs::read(pdf_path).map_err(|error| error.to_string())?,
            b"keep-old-pdf"
        );
        Ok(())
    }
}
