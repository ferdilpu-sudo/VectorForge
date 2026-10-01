use std::sync::{Arc, Condvar, Mutex};

use crate::models::AppError;

#[derive(Default)]
struct GateState {
    active: usize,
    preview_waiters: usize,
}

pub struct WorkGate {
    capacity: usize,
    state: Mutex<GateState>,
    wake: Condvar,
}

impl Default for WorkGate {
    fn default() -> Self {
        let capacity = std::thread::available_parallelism()
            .map(|value| value.get())
            .unwrap_or(1)
            .clamp(1, 4);
        Self::new(capacity)
    }
}

impl WorkGate {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            state: Mutex::new(GateState::default()),
            wake: Condvar::new(),
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn acquire_preview(self: &Arc<Self>) -> Result<WorkPermit, AppError> {
        self.acquire(true)
    }

    pub fn acquire_normal(self: &Arc<Self>) -> Result<WorkPermit, AppError> {
        self.acquire(false)
    }

    fn acquire(self: &Arc<Self>, preview: bool) -> Result<WorkPermit, AppError> {
        let mut state = self.state.lock().map_err(lock_error)?;
        if preview {
            state.preview_waiters = state.preview_waiters.saturating_add(1);
        }

        while state.active >= self.capacity || (!preview && state.preview_waiters > 0) {
            state = self.wake.wait(state).map_err(lock_error)?;
        }

        if preview {
            state.preview_waiters = state.preview_waiters.saturating_sub(1);
        }
        state.active = state.active.saturating_add(1);

        Ok(WorkPermit {
            gate: Arc::clone(self),
        })
    }
}

pub struct WorkPermit {
    gate: Arc<WorkGate>,
}

impl Drop for WorkPermit {
    fn drop(&mut self) {
        if let Ok(mut state) = self.gate.state.lock() {
            state.active = state.active.saturating_sub(1);
            self.gate.wake.notify_all();
        }
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> AppError {
    AppError::invalid_state("Heavy-work gate tidak dapat dikunci.", error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    use super::WorkGate;

    #[test]
    fn gate_blocks_work_above_capacity_until_permit_is_released() -> Result<(), String> {
        let gate = Arc::new(WorkGate::new(1));
        let first = gate.acquire_normal().map_err(|error| error.message)?;

        let (sent, received) = mpsc::sync_channel(1);
        let gate_for_thread = Arc::clone(&gate);
        let worker = thread::spawn(move || {
            let permit = gate_for_thread
                .acquire_normal()
                .map_err(|error| error.message)?;
            sent.send(()).map_err(|error| error.to_string())?;
            drop(permit);
            Ok::<(), String>(())
        });

        assert!(received.recv_timeout(Duration::from_millis(30)).is_err());
        drop(first);
        received
            .recv_timeout(Duration::from_secs(1))
            .map_err(|error| error.to_string())?;
        worker.join().map_err(|_| "worker panicked".to_owned())??;
        Ok(())
    }
}
