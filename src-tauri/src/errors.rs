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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion_cancelled() {
        let pob_error = PobError::Cancelled;
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Cancelled => {}
            _ => panic!("Expected Cancelled"),
        }
    }

    #[test]
    fn test_error_conversion_process_running() {
        let pob_error = PobError::ProcessRunning;
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Conflict(msg) => {
                assert!(msg.contains("PoB가 실행 중입니다"));
            }
            _ => panic!("Expected Conflict"),
        }
    }

    #[tokio::test]
    async fn test_error_conversion_network() {
        let reqwest_error = reqwest::get("http://invalid.invalid").await.unwrap_err();
        let pob_error: PobError = reqwest_error.into();
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Network(_) => {}
            _ => panic!("Expected Network"),
        }
    }

    #[test]
    fn test_error_conversion_io() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let pob_error = PobError::Io(io_error);
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Io(msg) => {
                assert!(msg.contains("file not found"));
            }
            _ => panic!("Expected Io"),
        }
    }

    #[test]
    fn test_error_conversion_not_found_from_drive() {
        let pob_error = PobError::NotFoundFromDrive("folder_id_123".to_string());
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::NotFound(msg) => {
                assert_eq!(msg, "folder_id_123");
            }
            _ => panic!("Expected NotFound"),
        }
    }

    #[test]
    fn test_error_conversion_download_failed() {
        let pob_error = PobError::DownloadFailed("connection timeout".to_string());
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Domain(msg) => {
                assert!(msg.contains("다운로드 실패"));
                assert!(msg.contains("connection timeout"));
            }
            _ => panic!("Expected Domain"),
        }
    }

    #[test]
    fn test_error_conversion_version_parse_error() {
        let pob_error = PobError::VersionParseError("invalid.zip".to_string());
        let error_kind: ErrorKind = pob_error.into();

        match error_kind {
            ErrorKind::Domain(msg) => {
                assert!(msg.contains("버전 파싱 실패"));
            }
            _ => panic!("Expected Domain"),
        }
    }

    #[test]
    fn test_error_from_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let error_kind: ErrorKind = io_error.into();

        match error_kind {
            ErrorKind::Io(msg) => {
                assert!(msg.contains("access denied"));
            }
            _ => panic!("Expected Io"),
        }
    }

    #[test]
    fn test_error_display() {
        // Test thiserror Display implementation
        let error = PobError::Cancelled;
        assert_eq!(error.to_string(), "설치가 취소되었습니다");

        let error = PobError::ProcessRunning;
        assert_eq!(error.to_string(), "PoB가 실행 중입니다");

        let error = PobError::VersionParseError("test.zip".to_string());
        assert_eq!(error.to_string(), "버전 파싱 실패: test.zip");
    }
}
