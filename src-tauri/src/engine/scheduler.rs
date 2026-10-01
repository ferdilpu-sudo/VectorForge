use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use vtracer::CancelToken;

use crate::models::{AppError, ErrorCode, PreviewResult};

use super::preview::{PreviewWork, render_preview};

type PreviewResponse = Result<PreviewResult, AppError>;
type PreviewProcessor =
    dyn Fn(PreviewWork, &CancelToken) -> PreviewResponse + Send + Sync + 'static;

struct PendingJob {
    work: PreviewWork,
    response: SyncSender<PreviewResponse>,
}

struct ActiveJob {
    request_id: String,
    cancel: CancelToken,
}

#[derive(Default)]
struct SchedulerState {
    worker_started: bool,
    shutting_down: bool,
    active: Option<ActiveJob>,
    pending: Option<PendingJob>,
}

struct SchedulerShared {
    state: Mutex<SchedulerState>,
    wake: Condvar,
    processor: Arc<PreviewProcessor>,
}

pub struct PreviewScheduler {
    shared: Arc<SchedulerShared>,
}

impl Default for PreviewScheduler {
    fn default() -> Self {
        Self::with_processor(Arc::new(render_preview))
    }
}

impl PreviewScheduler {
    fn with_processor(processor: Arc<PreviewProcessor>) -> Self {
        Self {
            shared: Arc::new(SchedulerShared {
                state: Mutex::new(SchedulerState::default()),
                wake: Condvar::new(),
                processor,
            }),
        }
    }

    pub fn submit(&self, work: PreviewWork) -> Result<Receiver<PreviewResponse>, AppError> {
        work.request.validate()?;
        let (response, receiver) = mpsc::sync_channel(1);

        let mut state = self.shared.state.lock().map_err(lock_error)?;
        self.ensure_worker(&mut state)?;

        if let Some(active) = state.active.as_ref() {
            active.cancel.cancel();
        }

        if let Some(replaced) = state.pending.take() {
            let _ = replaced.response.try_send(Err(cancelled_error()));
        }

        state.pending = Some(PendingJob { work, response });
        self.shared.wake.notify_one();
        Ok(receiver)
    }

    pub fn cancel(&self, request_id: &str) -> Result<(), AppError> {
        let mut state = self.shared.state.lock().map_err(lock_error)?;

        if let Some(active) = state
            .active
            .as_ref()
            .filter(|active| active.request_id == request_id)
        {
            active.cancel.cancel();
        }

        if let Some(pending) = state
            .pending
            .take_if(|pending| pending.work.request.request_id == request_id)
        {
            let _ = pending.response.try_send(Err(cancelled_error()));
        }

        Ok(())
    }

    fn ensure_worker(&self, state: &mut SchedulerState) -> Result<(), AppError> {
        if state.worker_started {
            return Ok(());
        }

        let shared = Arc::clone(&self.shared);
        thread::Builder::new()
            .name("vectorforge-preview".to_owned())
            .spawn(move || worker_loop(shared))
            .map_err(|error| {
                AppError::invalid_state("Worker preview gagal dibuat.", error.to_string())
            })?;
        state.worker_started = true;
        Ok(())
    }
}

impl Drop for PreviewScheduler {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.shutting_down = true;
            if let Some(active) = state.active.as_ref() {
                active.cancel.cancel();
            }
            if let Some(pending) = state.pending.take() {
                let _ = pending.response.try_send(Err(cancelled_error()));
            }
            self.shared.wake.notify_all();
        }
    }
}

fn worker_loop(shared: Arc<SchedulerShared>) {
    loop {
        let (pending, cancel, request_id) = {
            let mut state = match shared.state.lock() {
                Ok(state) => state,
                Err(_) => return,
            };

            while state.pending.is_none() && !state.shutting_down {
                state = match shared.wake.wait(state) {
                    Ok(state) => state,
                    Err(_) => return,
                };
            }

            if state.shutting_down {
                return;
            }

            let Some(pending) = state.pending.take() else {
                continue;
            };
            let request_id = pending.work.request.request_id.clone();
            let cancel = CancelToken::new();
            state.active = Some(ActiveJob {
                request_id: request_id.clone(),
                cancel: cancel.clone(),
            });

            (pending, cancel, request_id)
        };

        let mut result = match catch_unwind(AssertUnwindSafe(|| {
            (shared.processor)(pending.work, &cancel)
        })) {
            Ok(result) => result,
            Err(_) => Err(AppError::new(
                ErrorCode::TraceFailed,
                "Worker preview berhenti secara tidak terduga.",
            )),
        };

        match shared.state.lock() {
            Ok(mut state) => {
                let cancelled = state
                    .active
                    .as_ref()
                    .filter(|active| active.request_id == request_id)
                    .is_none_or(|active| active.cancel.is_cancelled());

                if state
                    .active
                    .as_ref()
                    .is_some_and(|active| active.request_id == request_id)
                {
                    state.active = None;
                }

                if cancelled && result.is_ok() {
                    result = Err(cancelled_error());
                }
            }
            Err(error) => {
                result = Err(lock_error(error));
            }
        }

        let _ = pending.response.send(result);
    }
}

fn cancelled_error() -> AppError {
    AppError::new(ErrorCode::Cancelled, "Preview dibatalkan.")
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> AppError {
    AppError::invalid_state("Scheduler preview tidak dapat dikunci.", error.to_string())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::thread;
    use std::time::Duration;

    use crate::files::SourceSnapshot;
    use crate::models::{
        ErrorCode, HierarchicalMode, PreviewRequest, PreviewResult, TraceMode, TraceParams,
        TraceStats,
    };

    use super::{PreviewProcessor, PreviewScheduler, PreviewWork};

    const A: &str = "00000000-0000-4000-8000-00000000000a";
    const B: &str = "00000000-0000-4000-8000-00000000000b";
    const C: &str = "00000000-0000-4000-8000-00000000000c";
    const FILE_ID: &str = "00000000-0000-4000-8000-000000000001";

    fn work(request_id: &str) -> PreviewWork {
        PreviewWork {
            source: SourceSnapshot {
                path: PathBuf::from("fixture.png"),
                fingerprint: "fixture".to_owned(),
            },
            request: PreviewRequest {
                file_id: FILE_ID.to_owned(),
                request_id: request_id.to_owned(),
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
            },
        }
    }

    fn success(work: PreviewWork) -> PreviewResult {
        PreviewResult {
            request_id: work.request.request_id,
            file_id: work.request.file_id,
            svg: "<svg/>".to_owned(),
            stats: TraceStats {
                path_count: 0,
                color_count: 0,
                svg_bytes: 6,
                width: 1,
                height: 1,
            },
            elapsed_ms: 0,
        }
    }

    #[test]
    fn latest_pending_request_wins_and_active_becomes_stale() -> Result<(), String> {
        let processor: Arc<PreviewProcessor> = Arc::new(|work, _cancel| {
            if work.request.request_id == A {
                thread::sleep(Duration::from_millis(80));
            }
            Ok(success(work))
        });
        let scheduler = PreviewScheduler::with_processor(processor);

        let a = scheduler.submit(work(A)).map_err(|error| error.message)?;
        thread::sleep(Duration::from_millis(10));
        let b = scheduler.submit(work(B)).map_err(|error| error.message)?;
        let c = scheduler.submit(work(C)).map_err(|error| error.message)?;

        let a_result = a
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| error.to_string())?;
        let b_result = b
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| error.to_string())?;
        let c_result = c
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| error.to_string())?;

        assert!(matches!(a_result, Err(error) if error.code == ErrorCode::Cancelled));
        assert!(matches!(b_result, Err(error) if error.code == ErrorCode::Cancelled));
        assert!(matches!(c_result, Ok(result) if result.request_id == C));
        Ok(())
    }
}
