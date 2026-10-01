use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    FileNotFound,
    UnsupportedFormat,
    ImageCorrupt,
    ImageTooLarge,
    InvalidParams,
    TraceFailed,
    ExportFailed,
    WriteFailed,
    Cancelled,
    PresetDuplicate,
    PresetReadOnly,
    NotFound,
    AccessDenied,
    SourceChanged,
    BatchBusy,
    InvalidState,
    OutputTooLarge,
    DataVersionUnsupported,
    DataCorrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn invalid_state(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::with_details(ErrorCode::InvalidState, message, details)
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, ErrorCode};

    #[test]
    fn app_error_serializes_schema_codes() -> Result<(), String> {
        let error = AppError::with_details(
            ErrorCode::SourceChanged,
            "Sumber berubah.",
            "fingerprint mismatch",
        );
        let json = serde_json::to_value(error).map_err(|error| error.to_string())?;

        if json["code"] != "SOURCE_CHANGED"
            || json["message"] != "Sumber berubah."
            || json["details"] != "fingerprint mismatch"
        {
            return Err(format!("unexpected AppError JSON: {json}"));
        }

        Ok(())
    }
}
