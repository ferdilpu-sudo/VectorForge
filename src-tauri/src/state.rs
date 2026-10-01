use std::sync::{Arc, Mutex};

use crate::batch::BatchScheduler;
use crate::engine::{PreviewScheduler, WorkGate};
use crate::files::FileRegistry;

pub struct AppState {
    pub registry: Arc<Mutex<FileRegistry>>,
    pub preview: PreviewScheduler,
    pub batch: BatchScheduler,
    pub heavy: Arc<WorkGate>,
    pub storage: Arc<Mutex<()>>,
}

impl Default for AppState {
    fn default() -> Self {
        let registry = Arc::new(Mutex::new(FileRegistry::default()));
        let heavy = Arc::new(WorkGate::default());

        Self {
            registry: Arc::clone(&registry),
            preview: PreviewScheduler::new(Arc::clone(&heavy)),
            batch: BatchScheduler::new(Arc::clone(&heavy), registry),
            heavy,
            storage: Arc::new(Mutex::new(())),
        }
    }
}
