//! Structured error model (design section 19).

use std::fmt;

use serde::{Deserialize, Serialize};

/// Stable error codes shared across the control plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorCode {
    InvalidRecipe,
    InvalidRegion,
    UnsupportedCapability,
    EditorNotFound,
    EditorVersionUnsupported,
    Pp3GenerationFailed,
    PreviewFailed,
    ApplyFailed,
    ExportFailed,
    SourceChanged,
    PermissionDenied,
    ProviderError,
    ModelOutputInvalid,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidRecipe => "INVALID_RECIPE",
            ErrorCode::InvalidRegion => "INVALID_REGION",
            ErrorCode::UnsupportedCapability => "UNSUPPORTED_CAPABILITY",
            ErrorCode::EditorNotFound => "EDITOR_NOT_FOUND",
            ErrorCode::EditorVersionUnsupported => "EDITOR_VERSION_UNSUPPORTED",
            ErrorCode::Pp3GenerationFailed => "PP3_GENERATION_FAILED",
            ErrorCode::PreviewFailed => "PREVIEW_FAILED",
            ErrorCode::ApplyFailed => "APPLY_FAILED",
            ErrorCode::ExportFailed => "EXPORT_FAILED",
            ErrorCode::SourceChanged => "SOURCE_CHANGED",
            ErrorCode::PermissionDenied => "PERMISSION_DENIED",
            ErrorCode::ProviderError => "PROVIDER_ERROR",
            ErrorCode::ModelOutputInvalid => "MODEL_OUTPUT_INVALID",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single validation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub code: ErrorCode,
    pub message: String,

    /// Field path within the recipe, e.g. `global.exposure_ev`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ValidationError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            field: None,
        }
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.field {
            Some(field) => write!(f, "{} ({}): {}", self.code, field, self.message),
            None => write!(f, "{}: {}", self.code, self.message),
        }
    }
}
