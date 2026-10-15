use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Standardized error codes for UMR diagnostic reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // Package intake & decompression errors
    ErrIntakeMalformedArchive,
    ErrIntakeZipTraversal,
    ErrIntakeZipBomb,
    ErrIntakeLimitExceeded,
    ErrIntakeUnsupportedFormat,
    ErrIntakeMissingManifest,
    ErrIntakeParseFailure,
    ErrIntakeDrmProtected,

    // Lifecycle state errors
    ErrLifecycleInvalidTransition,
    ErrLifecycleSessionNotFound,
    ErrLifecycleTimeout,
    ErrLifecycleSessionTerminated,

    // Backend capability and execution errors
    ErrBackendUnavailable,
    ErrBackendIncompatibleArchitecture,
    ErrBackendIncompatibleApiLevel,
    ErrBackendRemoteDeviceRequired,
    ErrBackendProcessCrashed,
    ErrBackendInternal,
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// A structured diagnostic error carrying an error code, message, and actionable suggestion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{code}: {message}")]
pub struct DiagnosticError {
    /// Standardized error code
    pub code: ErrorCode,
    /// Human-readable explanation of what went wrong
    pub message: String,
    /// Actionable next step or suggestion for the operator/user
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Optional context details (e.g. invalid path, required ABI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

impl DiagnosticError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            suggestion: None,
            context: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_context(mut self, context: impl Into<String>) -> Self {
        self.context = Some(context.into());
        self
    }

    // Convenience constructors
    pub fn zip_traversal(path: impl Into<String>) -> Self {
        let p = path.into();
        Self::new(
            ErrorCode::ErrIntakeZipTraversal,
            format!("Path traversal attempt detected in archive entry: {}", p),
        )
        .with_suggestion("Re-package the application to ensure all archive paths are relative and clean.")
        .with_context(p)
    }

    pub fn zip_bomb(details: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ErrIntakeZipBomb,
            "Archive exceeded safe decompression compression ratio or size limits",
        )
        .with_suggestion("Verify the package was not corrupted or maliciously compressed.")
        .with_context(details)
    }

    pub fn drm_protected(bundle_id: impl Into<String>) -> Self {
        let id = bundle_id.into();
        Self::new(
            ErrorCode::ErrIntakeDrmProtected,
            format!("Application '{}' is protected with FairPlay DRM encryption (cryptid != 0)", id),
        )
        .with_suggestion("Commercial App Store encrypted binaries cannot execute on desktop hosts. Provide an unencrypted developer or simulator build.")
        .with_context(id)
    }

    pub fn invalid_transition(from: impl std::fmt::Display, to: impl std::fmt::Display) -> Self {
        Self::new(
            ErrorCode::ErrLifecycleInvalidTransition,
            format!("Illegal state transition requested from '{}' to '{}'", from, to),
        )
        .with_suggestion("Ensure lifecycle calls follow the sequence: Unavailable -> Ready -> Preparing -> Running -> Stopping -> Terminated.")
    }

    pub fn remote_device_required(reason: impl Into<String>) -> Self {
        Self::new(
            ErrorCode::ErrBackendRemoteDeviceRequired,
            "Local execution is unavailable for this application on the current host OS",
        )
        .with_suggestion("Connect a physical device or configure a remote runner session.")
        .with_context(reason)
    }
}

pub type UmrResult<T> = Result<T, DiagnosticError>;

