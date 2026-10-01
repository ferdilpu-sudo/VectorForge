use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter};
use uuid::Uuid;
use vtracer::CancelToken;

use crate::engine::WorkGate;
use crate::files::FileRegistry;
use crate::models::{
    AppError, BatchHandle, BatchItem, BatchOutput, BatchProgress, BatchStatus, BatchSummary,
    ErrorCode, ItemStatus, JobStage, OutputStatus, validate_batch_id,
};

use super::processor::{BatchItemWork, ItemProcessResult, process_item};

const EVENT_PROGRESS: &str = "batch://progress";
const EVENT_DONE: &str = "batch://done";
const EVENT_INTERVAL: Duration = Duration::from_millis(100);

struct StoredItem {
    work: BatchItemWork,
    view: BatchItem,
}

struct BatchRecord {
    batch_id: String,
    run_id: String,
    sequence: u64,
    status: BatchStatus,
    cancel_requested: bool,
    items: Vec<StoredItem>,
    active: HashMap<String, CancelToken>,
    workers_remaining: usize,
    run_started: Instant,
    last_emit: Instant,
}

#[derive(Default)]
struct SchedulerState {
    batch: Option<BatchRecord>,
}

struct SchedulerShared {
    state: Mutex<SchedulerState>,
    gate: Arc<WorkGate>,
    registry: Arc<Mutex<FileRegistry>>,
}

pub struct BatchScheduler {
    shared: Arc<SchedulerShared>,
}

impl BatchScheduler {
    pub fn new(gate: Arc<WorkGate>, registry: Arc<Mutex<FileRegistry>>) -> Self {
        Self {
            shared: Arc::new(SchedulerShared {
                state: Mutex::new(SchedulerState::default()),
                gate,
                registry,
            }),
        }
    }

    pub fn start(
        &self,
        app: AppHandle,
        works: Vec<BatchItemWork>,
        requested_workers: usize,
    ) -> Result<BatchHandle, AppError> {
        if works.is_empty() {
            return Err(AppError::new(
                ErrorCode::InvalidParams,
                "Batch tidak memiliki item.",
            ));
        }

        let batch_id = Uuid::new_v4().to_string();
        let run_id = Uuid::new_v4().to_string();
        let worker_limit = requested_workers
            .clamp(1, 4)
            .min(self.shared.gate.capacity())
            .min(works.len());

        let mut stem_counts = HashMap::<String, u32>::new();
        let items = works
            .into_iter()
            .map(|mut work| {
                let base = source_stem(&work.name);
                if work.overwrite {
                    let key = base.to_lowercase();
                    let count = stem_counts
                        .entry(key)
                        .and_modify(|value| *value = value.saturating_add(1))
                        .or_insert(1);
                    work.output_stem = if *count == 1 {
                        base
                    } else {
                        format!("{base} ({count})")
                    };
                } else {
                    work.output_stem = base;
                }

                StoredItem {
                view: BatchItem {
                    id: Uuid::new_v4().to_string(),
                    file_id: work.file_id.clone(),
                    name: work.name.clone(),
                    status: ItemStatus::Queued,
                    stage: JobStage::Waiting,
                    outputs: work
                        .formats
                        .iter()
                        .copied()
                        .map(queued_output)
                        .collect(),
                    error: None,
                    elapsed_ms: None,
                },
                    work,
                }
            })
            .collect::<Vec<_>>();

        let progress = {
            let mut state = self.shared.state.lock().map_err(lock_error)?;
            if state
                .batch
                .as_ref()
                .is_some_and(|batch| batch.status != BatchStatus::Finished)
            {
                return Err(AppError::new(
                    ErrorCode::BatchBusy,
                    "Masih ada batch yang berjalan.",
                ));
            }

            let now = Instant::now();
            let record = BatchRecord {
                batch_id: batch_id.clone(),
                run_id: run_id.clone(),
                sequence: 1,
                status: BatchStatus::Running,
                cancel_requested: false,
                items,
                active: HashMap::new(),
                workers_remaining: worker_limit,
                run_started: now,
                last_emit: now,
            };
            let progress = snapshot(&record);
            state.batch = Some(record);
            progress
        };

        emit_progress(&app, progress);
        spawn_workers(
            Arc::clone(&self.shared),
            app,
            batch_id.clone(),
            run_id.clone(),
            worker_limit,
        );

        Ok(BatchHandle { batch_id, run_id })
    }

    pub fn get(&self, batch_id: &str) -> Result<BatchProgress, AppError> {
        validate_batch_id(batch_id, "ID batch")?;
        let state = self.shared.state.lock().map_err(lock_error)?;
        let batch = state
            .batch
            .as_ref()
            .filter(|batch| batch.batch_id == batch_id)
            .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Batch tidak ditemukan."))?;
        Ok(snapshot(batch))
    }

    pub fn cancel(
        &self,
        app: &AppHandle,
        batch_id: &str,
        run_id: &str,
    ) -> Result<(), AppError> {
        validate_batch_id(batch_id, "ID batch")?;
        validate_batch_id(run_id, "ID run")?;

        let progress = {
            let mut state = self.shared.state.lock().map_err(lock_error)?;
            let batch = state
                .batch
                .as_mut()
                .filter(|batch| batch.batch_id == batch_id)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Batch tidak ditemukan."))?;

            if batch.run_id != run_id {
                return Err(AppError::new(
                    ErrorCode::NotFound,
                    "Run batch tidak aktif.",
                ));
            }
            if batch.status == BatchStatus::Finished {
                return Ok(());
            }

            batch.cancel_requested = true;
            batch.status = BatchStatus::Cancelling;

            for token in batch.active.values() {
                token.cancel();
            }
            for stored in &mut batch.items {
                if stored.view.status == ItemStatus::Queued {
                    stored.view.status = ItemStatus::Cancelled;
                    stored.view.stage = JobStage::Finished;
                    stored.view.error = None;
                    for output in &mut stored.view.outputs {
                        if output.status == OutputStatus::Queued {
                            output.status = OutputStatus::Cancelled;
                            output.error = None;
                        }
                    }
                }
            }

            batch.sequence = batch.sequence.saturating_add(1);
            batch.last_emit = Instant::now();
            snapshot(batch)
        };

        emit_progress(app, progress);
        Ok(())
    }

    pub fn retry_item(
        &self,
        app: AppHandle,
        batch_id: &str,
        item_id: &str,
    ) -> Result<BatchHandle, AppError> {
        validate_batch_id(batch_id, "ID batch")?;
        validate_batch_id(item_id, "ID item")?;

        let (handle, progress) = {
            let mut state = self.shared.state.lock().map_err(lock_error)?;
            let batch = state
                .batch
                .as_mut()
                .filter(|batch| batch.batch_id == batch_id)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Batch tidak ditemukan."))?;

            if batch.status != BatchStatus::Finished {
                return Err(AppError::new(
                    ErrorCode::BatchBusy,
                    "Retry hanya dapat dimulai setelah batch selesai.",
                ));
            }

            let stored = batch
                .items
                .iter_mut()
                .find(|stored| stored.view.id == item_id)
                .ok_or_else(|| AppError::new(ErrorCode::NotFound, "Item batch tidak ditemukan."))?;

            reset_for_retry(&mut stored.view)?;

            let run_id = Uuid::new_v4().to_string();
            batch.run_id = run_id.clone();
            batch.sequence = 1;
            batch.status = BatchStatus::Running;
            batch.cancel_requested = false;
            batch.active.clear();
            batch.workers_remaining = 1;
            batch.run_started = Instant::now();
            batch.last_emit = batch.run_started;

            (
                BatchHandle {
                    batch_id: batch.batch_id.clone(),
                    run_id,
                },
                snapshot(batch),
            )
        };

        emit_progress(&app, progress);
        spawn_workers(
            Arc::clone(&self.shared),
            app,
            handle.batch_id.clone(),
            handle.run_id.clone(),
            1,
        );
        Ok(handle)
    }
}

fn spawn_workers(
    shared: Arc<SchedulerShared>,
    app: AppHandle,
    batch_id: String,
    run_id: String,
    count: usize,
) {
    for index in 0..count {
        let shared_for_thread = Arc::clone(&shared);
        let app_for_thread = app.clone();
        let batch_for_thread = batch_id.clone();
        let run_for_thread = run_id.clone();

        let spawn = thread::Builder::new()
            .name(format!("vectorforge-batch-{index}"))
            .spawn(move || {
                worker_loop(
                    shared_for_thread,
                    app_for_thread,
                    batch_for_thread,
                    run_for_thread,
                )
            });

        if let Err(error) = spawn {
            worker_spawn_failed(
                &shared,
                &app,
                &batch_id,
                &run_id,
                AppError::invalid_state("Worker batch gagal dibuat.", error.to_string()),
            );
        }
    }
}

fn worker_loop(
    shared: Arc<SchedulerShared>,
    app: AppHandle,
    batch_id: String,
    run_id: String,
) {
    while let Some(claimed) = claim_next(&shared, &app, &batch_id, &run_id) {
        let item_id = claimed.item_id.clone();
        let shared_for_stage = Arc::clone(&shared);
        let app_for_stage = app.clone();
        let batch_for_stage = batch_id.clone();
        let run_for_stage = run_id.clone();
        let item_for_stage = item_id.clone();

        let result = process_item(
            claimed.work,
            &claimed.cancel,
            &shared.gate,
            &shared.registry,
            move |stage, format| {
                update_stage(
                    &shared_for_stage,
                    &app_for_stage,
                    &batch_for_stage,
                    &run_for_stage,
                    &item_for_stage,
                    stage,
                    format,
                );
            },
        );

        complete_item(
            &shared,
            &app,
            &batch_id,
            &run_id,
            &item_id,
            result,
        );
    }

    worker_finished(&shared, &app, &batch_id, &run_id);
}

struct ClaimedItem {
    item_id: String,
    work: BatchItemWork,
    cancel: CancelToken,
}

fn claim_next(
    shared: &Arc<SchedulerShared>,
    app: &AppHandle,
    batch_id: &str,
    run_id: &str,
) -> Option<ClaimedItem> {
    let (claimed, progress) = {
        let mut state = shared.state.lock().ok()?;
        let batch = state.batch.as_mut()?;
        if batch.batch_id != batch_id
            || batch.run_id != run_id
            || batch.status != BatchStatus::Running
            || batch.cancel_requested
        {
            return None;
        }

        let stored = batch
            .items
            .iter_mut()
            .find(|stored| stored.view.status == ItemStatus::Queued)?;

        let formats = stored
            .view
            .outputs
            .iter()
            .filter(|output| output.status == OutputStatus::Queued)
            .map(|output| output.format)
            .collect::<Vec<_>>();
        if formats.is_empty() {
            stored.view.status = ItemStatus::Failed;
            stored.view.stage = JobStage::Finished;
            stored.view.error = Some(AppError::new(
                ErrorCode::InvalidState,
                "Item batch tidak memiliki output yang dapat diproses.",
            ));
            batch.sequence = batch.sequence.saturating_add(1);
            return None;
        }

        let mut work = stored.work.clone();
        work.formats = formats;
        let item_id = stored.view.id.clone();
        let cancel = CancelToken::new();

        stored.view.status = ItemStatus::Processing;
        stored.view.stage = JobStage::Decoding;
        stored.view.error = None;
        stored.view.elapsed_ms = None;
        batch.active.insert(item_id.clone(), cancel.clone());
        batch.sequence = batch.sequence.saturating_add(1);
        let progress = throttled_snapshot(batch, false);

        (
            ClaimedItem {
                item_id,
                work,
                cancel,
            },
            progress,
        )
    };

    if let Some(progress) = progress {
        emit_progress(app, progress);
    }
    Some(claimed)
}

fn update_stage(
    shared: &Arc<SchedulerShared>,
    app: &AppHandle,
    batch_id: &str,
    run_id: &str,
    item_id: &str,
    stage: JobStage,
    format: Option<crate::models::ExportFormat>,
) {
    let progress = {
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        let Some(batch) = state.batch.as_mut() else {
            return;
        };
        if batch.batch_id != batch_id || batch.run_id != run_id {
            return;
        }

        let Some(stored) = batch.items.iter_mut().find(|stored| stored.view.id == item_id) else {
            return;
        };
        if stored.view.status != ItemStatus::Processing {
            return;
        }

        stored.view.stage = stage;
        if let Some(format) = format
            && let Some(output) = stored
                .view
                .outputs
                .iter_mut()
                .find(|output| output.format == format && output.status == OutputStatus::Queued)
        {
            output.status = OutputStatus::Processing;
        }
        batch.sequence = batch.sequence.saturating_add(1);
        throttled_snapshot(batch, false)
    };

    if let Some(progress) = progress {
        emit_progress(app, progress);
    }
}

fn complete_item(
    shared: &Arc<SchedulerShared>,
    app: &AppHandle,
    batch_id: &str,
    run_id: &str,
    item_id: &str,
    result: ItemProcessResult,
) {
    let progress = {
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        let Some(batch) = state.batch.as_mut() else {
            return;
        };
        if batch.batch_id != batch_id || batch.run_id != run_id {
            return;
        }

        batch.active.remove(item_id);
        let Some(stored) = batch.items.iter_mut().find(|stored| stored.view.id == item_id) else {
            return;
        };

        for output in result.outputs {
            if let Some(existing) = stored
                .view
                .outputs
                .iter_mut()
                .find(|existing| existing.format == output.format)
            {
                *existing = output;
            }
        }

        stored.view.stage = JobStage::Finished;
        stored.view.elapsed_ms = Some(result.elapsed_ms);
        stored.view.status = derive_item_status(&stored.view.outputs, result.cancelled);
        stored.view.error = if matches!(stored.view.status, ItemStatus::Failed | ItemStatus::Partial)
        {
            result.item_error.or_else(|| {
                stored
                    .view
                    .outputs
                    .iter()
                    .find_map(|output| output.error.clone())
            })
        } else {
            None
        };

        batch.sequence = batch.sequence.saturating_add(1);
        throttled_snapshot(batch, false)
    };

    if let Some(progress) = progress {
        emit_progress(app, progress);
    }
}

fn worker_finished(
    shared: &Arc<SchedulerShared>,
    app: &AppHandle,
    batch_id: &str,
    run_id: &str,
) {
    let terminal = {
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        let Some(batch) = state.batch.as_mut() else {
            return;
        };
        if batch.batch_id != batch_id || batch.run_id != run_id {
            return;
        }

        batch.workers_remaining = batch.workers_remaining.saturating_sub(1);
        if batch.workers_remaining != 0 {
            return;
        }

        if batch
            .items
            .iter()
            .any(|stored| stored.view.status == ItemStatus::Queued)
        {
            let error = AppError::new(
                ErrorCode::InvalidState,
                "Worker batch berhenti sebelum seluruh item diproses.",
            );
            fail_remaining_queued(batch, error);
        }

        batch.status = BatchStatus::Finished;
        batch.sequence = batch.sequence.saturating_add(1);
        batch.last_emit = Instant::now();
        Some((snapshot(batch), summary(batch)))
    };

    if let Some((progress, summary)) = terminal {
        emit_progress(app, progress);
        emit_done(app, summary);
    }
}

fn worker_spawn_failed(
    shared: &Arc<SchedulerShared>,
    app: &AppHandle,
    batch_id: &str,
    run_id: &str,
    error: AppError,
) {
    let terminal = {
        let Ok(mut state) = shared.state.lock() else {
            return;
        };
        let Some(batch) = state.batch.as_mut() else {
            return;
        };
        if batch.batch_id != batch_id || batch.run_id != run_id {
            return;
        }

        batch.workers_remaining = batch.workers_remaining.saturating_sub(1);
        if batch.workers_remaining != 0 {
            return;
        }

        fail_remaining_queued(batch, error);
        batch.status = BatchStatus::Finished;
        batch.sequence = batch.sequence.saturating_add(1);
        batch.last_emit = Instant::now();
        Some((snapshot(batch), summary(batch)))
    };

    if let Some((progress, summary)) = terminal {
        emit_progress(app, progress);
        emit_done(app, summary);
    }
}

fn fail_remaining_queued(batch: &mut BatchRecord, error: AppError) {
    for stored in &mut batch.items {
        if stored.view.status == ItemStatus::Queued {
            stored.view.status = ItemStatus::Failed;
            stored.view.stage = JobStage::Finished;
            stored.view.error = Some(error.clone());
            for output in &mut stored.view.outputs {
                if output.status == OutputStatus::Queued {
                    output.status = OutputStatus::Failed;
                    output.error = Some(error.clone());
                }
            }
        }
    }
}

fn reset_for_retry(item: &mut BatchItem) -> Result<(), AppError> {
    if !matches!(item.status, ItemStatus::Failed | ItemStatus::Partial) {
        return Err(AppError::new(
            ErrorCode::InvalidState,
            "Hanya item failed atau partial yang dapat di-retry.",
        ));
    }

    let mut reset_any = false;
    for output in &mut item.outputs {
        if output.status == OutputStatus::Failed {
            output.status = OutputStatus::Queued;
            output.output_id = None;
            output.out_path = None;
            output.bytes = None;
            output.error = None;
            reset_any = true;
        }
    }

    if !reset_any {
        return Err(AppError::new(
            ErrorCode::InvalidState,
            "Item tidak memiliki output gagal untuk di-retry.",
        ));
    }

    item.status = ItemStatus::Queued;
    item.stage = JobStage::Waiting;
    item.error = None;
    item.elapsed_ms = None;
    Ok(())
}

fn derive_item_status(outputs: &[BatchOutput], cancelled: bool) -> ItemStatus {
    if cancelled {
        return ItemStatus::Cancelled;
    }

    let done = outputs
        .iter()
        .filter(|output| output.status == OutputStatus::Done)
        .count();
    let failed = outputs
        .iter()
        .filter(|output| output.status == OutputStatus::Failed)
        .count();
    let cancelled_outputs = outputs
        .iter()
        .filter(|output| output.status == OutputStatus::Cancelled)
        .count();

    if done == outputs.len() && !outputs.is_empty() {
        ItemStatus::Done
    } else if done > 0 && failed > 0 {
        ItemStatus::Partial
    } else if failed > 0 {
        ItemStatus::Failed
    } else if cancelled_outputs > 0 {
        ItemStatus::Cancelled
    } else {
        ItemStatus::Failed
    }
}

fn source_stem(name: &str) -> String {
    std::path::Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("output")
        .to_owned()
}

fn queued_output(format: crate::models::ExportFormat) -> BatchOutput {
    BatchOutput {
        format,
        status: OutputStatus::Queued,
        output_id: None,
        out_path: None,
        bytes: None,
        error: None,
    }
}

fn snapshot(batch: &BatchRecord) -> BatchProgress {
    let (done_count, partial_count, failed_count, cancelled_count) = terminal_counts(batch);
    BatchProgress {
        batch_id: batch.batch_id.clone(),
        run_id: batch.run_id.clone(),
        sequence: batch.sequence,
        status: batch.status,
        items: batch.items.iter().map(|stored| stored.view.clone()).collect(),
        completed_count: done_count + partial_count + failed_count + cancelled_count,
        done_count,
        partial_count,
        failed_count,
        cancelled_count,
        total: batch.items.len().min(u32::MAX as usize) as u32,
    }
}

fn summary(batch: &BatchRecord) -> BatchSummary {
    let (done_count, partial_count, failed_count, cancelled_count) = terminal_counts(batch);
    BatchSummary {
        batch_id: batch.batch_id.clone(),
        run_id: batch.run_id.clone(),
        sequence: batch.sequence,
        succeeded: done_count,
        partial: partial_count,
        failed: failed_count,
        cancelled_count,
        cancel_requested: batch.cancel_requested,
        total: batch.items.len().min(u32::MAX as usize) as u32,
        total_elapsed_ms: batch
            .run_started
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64,
    }
}

fn terminal_counts(batch: &BatchRecord) -> (u32, u32, u32, u32) {
    let mut done = 0u32;
    let mut partial = 0u32;
    let mut failed = 0u32;
    let mut cancelled = 0u32;

    for stored in &batch.items {
        match stored.view.status {
            ItemStatus::Done => done = done.saturating_add(1),
            ItemStatus::Partial => partial = partial.saturating_add(1),
            ItemStatus::Failed => failed = failed.saturating_add(1),
            ItemStatus::Cancelled => cancelled = cancelled.saturating_add(1),
            ItemStatus::Queued | ItemStatus::Processing => {}
        }
    }

    (done, partial, failed, cancelled)
}

fn throttled_snapshot(batch: &mut BatchRecord, force: bool) -> Option<BatchProgress> {
    if force || batch.last_emit.elapsed() >= EVENT_INTERVAL {
        batch.last_emit = Instant::now();
        Some(snapshot(batch))
    } else {
        None
    }
}

fn emit_progress(app: &AppHandle, progress: BatchProgress) {
    let _ = app.emit(EVENT_PROGRESS, progress);
}

fn emit_done(app: &AppHandle, summary: BatchSummary) {
    let _ = app.emit(EVENT_DONE, summary);
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> AppError {
    AppError::invalid_state("Scheduler batch tidak dapat dikunci.", error.to_string())
}

#[cfg(test)]
mod tests {
    use crate::models::{BatchOutput, ExportFormat};

    use super::{derive_item_status, queued_output, reset_for_retry};
    use crate::models::{BatchItem, ItemStatus, JobStage, OutputStatus};

    fn done_output(format: ExportFormat) -> BatchOutput {
        BatchOutput {
            format,
            status: OutputStatus::Done,
            output_id: Some("00000000-0000-4000-8000-000000000010".to_owned()),
            out_path: Some("result.svg".to_owned()),
            bytes: Some(10),
            error: None,
        }
    }

    #[test]
    fn mixed_done_and_failed_outputs_become_partial() {
        let mut failed = queued_output(ExportFormat::Pdf);
        failed.status = OutputStatus::Failed;
        assert_eq!(
            derive_item_status(&[done_output(ExportFormat::Svg), failed], false),
            ItemStatus::Partial
        );
    }

    #[test]
    fn retry_resets_only_failed_outputs_and_preserves_done() -> Result<(), String> {
        let done = done_output(ExportFormat::Svg);
        let mut failed = queued_output(ExportFormat::Pdf);
        failed.status = OutputStatus::Failed;
        let mut item = BatchItem {
            id: "00000000-0000-4000-8000-000000000001".to_owned(),
            file_id: "00000000-0000-4000-8000-000000000002".to_owned(),
            name: "logo.png".to_owned(),
            status: ItemStatus::Partial,
            stage: JobStage::Finished,
            outputs: vec![done.clone(), failed],
            error: None,
            elapsed_ms: Some(10),
        };

        reset_for_retry(&mut item).map_err(|error| error.message)?;
        assert_eq!(item.status, ItemStatus::Queued);
        assert_eq!(item.outputs[0], done);
        assert_eq!(item.outputs[1].status, OutputStatus::Queued);
        Ok(())
    }

    #[test]
    fn cancelled_run_forces_item_cancelled_even_with_done_output() {
        assert_eq!(
            derive_item_status(&[done_output(ExportFormat::Svg)], true),
            ItemStatus::Cancelled
        );
    }
}
