use crate::pob::error::PobError;
use serde::Serialize;
use specta::Type;

/// IPC error type for frontend consumption.
/// Designed for UI-actionable categories, not implementation details.
#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    /// User cancelled the operation (not an error, no toast needed)
    Cancelled,
    /// Network/connectivity issues (retry may help)
    Network(String),
    /// Filesystem/permission issues
    Io(String),
    /// Resource not found (e.g., file not on Google Drive)
    NotFound(String),
    /// Conflict state (e.g., PoB is running)
    Conflict(String),
    /// Other domain errors
    Domain(String),
}

impl From<tauri::Error> for ErrorKind {
    fn from(err: tauri::Error) -> Self {
        ErrorKind::Domain(err.to_string())
    }
}

impl From<PobError> for ErrorKind {
    fn from(err: PobError) -> Self {
        match err {
            // Control flow - not an error
            PobError::Cancelled => ErrorKind::Cancelled,

            // Conflict states
            PobError::ProcessRunning => {
                ErrorKind::Conflict("PoB가 실행 중입니다. 종료 후 다시 시도해주세요.".into())
            }

            // Network issues
            PobError::Network(e) => ErrorKind::Network(e.to_string()),

            // IO/filesystem issues
            PobError::Io(e) => ErrorKind::Io(e.to_string()),

            // Not found
            PobError::NotFoundFromDrive(msg) => ErrorKind::NotFound(msg),

            // Domain errors (everything else)
            other => ErrorKind::Domain(other.to_string()),
        }
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        ErrorKind::Io(err.to_string())
    }
}
