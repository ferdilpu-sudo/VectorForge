use std::sync::Mutex;

use crate::files::FileRegistry;

#[derive(Default)]
pub struct AppState {
    pub registry: Mutex<FileRegistry>,
}
