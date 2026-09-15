use std::fmt;

/// Decision 0057's typed error taxonomy for the mobile acquisition boundary.
/// The frontend switches on `code`; it must never parse `message` prose to
/// determine error class.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AcquisitionError {
    pub code: ErrorCode,
    pub message: String,
}

impl AcquisitionError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for AcquisitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for AcquisitionError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    SelectionCancelled,
    PermissionDenied,
    SourceUnavailable,
    UnsupportedEntry,
    PathEscape,
    DuplicatePath,
    CaseConflict,
    ResourceLimit,
    ArchiveInvalid,
    ArchiveSymlink,
    OperationCancelled,
    WorkspaceNotFound,
    WorkspaceNotReady,
    ExportConflict,
    SourceDiverged,
    InternalIo,
}

pub type AcquisitionResult<T> = Result<T, AcquisitionError>;
