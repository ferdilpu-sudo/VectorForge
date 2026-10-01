use std::sync::Mutex;

use crate::engine::PreviewScheduler;
use crate::files::FileRegistry;

pub struct AppState {
    pub registry: Mutex<FileRegistry>,
    pub preview: PreviewScheduler,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            registry: Mutex::new(FileRegistry::default()),
            preview: PreviewScheduler::default(),
        }
    }
}
