mod batch;
mod error;
mod export;
mod files;
pub(crate) mod preferences;
mod trace;

pub use batch::{
    BatchHandle, BatchItem, BatchOutput, BatchProgress, BatchRequest, BatchStatus, BatchSummary,
    ItemStatus, JobStage, OutputStatus, validate_batch_id,
};
pub use error::{AppError, ErrorCode};
pub use export::{ExportRequest, ExportResult};
pub use files::{
    Destination, DestinationKind, DestinationRequest, ExportFormat, ImportRejection, ImportRequest,
    ImportResult, SourceFile, SourceFormat,
};
pub use preferences::{
    AppSettings, PRESETS_VERSION, Preset, PresetsFile, SETTINGS_VERSION, SavePresetRequest,
    SettingsFile,
};

pub use trace::{
    HierarchicalMode, PreviewRequest, PreviewResult, TraceMode, TraceParams, TraceStats,
    validate_request_id,
};
