use std::sync::{Arc, Mutex};

use crate::engine::{PreviewScheduler, WorkGate};
use crate::files::FileRegistry;

pub struct AppState {
    pub registry: Arc<Mutex<FileRegistry>>,
    pub preview: PreviewScheduler,
    pub heavy: Arc<WorkGate>,
    pub storage: Mutex<()>,
}

impl Default for AppState {
    fn default() -> Self {
        let registry = Arc::new(Mutex::new(FileRegistry::default()));
        let heavy = Arc::new(WorkGate::default());

        Self {
            registry,
            preview: PreviewScheduler::new(Arc::clone(&heavy)),
            heavy,
            storage: Mutex::new(()),
        }
    }
}
