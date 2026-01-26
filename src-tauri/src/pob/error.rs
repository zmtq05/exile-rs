/// Domain-specific errors for PoB operations.
/// These are internal errors; they get mapped to ErrorKind for IPC.
#[derive(Debug, thiserror::Error)]
pub enum PobError {
    // === Semantic states (UI needs special handling) ===
    /// User cancelled the operation
    #[error("설치가 취소되었습니다")]
    Cancelled,

    /// PoB process is running (conflict state)
    #[error("PoB가 실행 중입니다")]
    ProcessRunning,

    /// File not found on Google Drive
    #[error("Google Drive에서 파일을 찾을 수 없습니다: {0}")]
    NotFoundFromDrive(String),

    // === Operation failures (phase-specific) ===
    /// Download failed with context
    #[error("다운로드 실패: {0}")]
    DownloadFailed(String),

    /// Extraction failed with context
    #[error("압축 해제 실패: {0}")]
    ExtractFailed(String),

    /// Version parsing failed
    #[error("버전 파싱 실패: {0}")]
    VersionParseError(String),

    // === Wrapped external errors ===
    /// Network errors (reqwest)
    #[error("네트워크 에러: {0}")]
    Network(#[from] reqwest::Error),

    /// Filesystem I/O errors
    #[error("I/O 에러: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parsing errors
    #[error("JSON 파싱 에러: {0}")]
    Json(#[from] serde_json::Error),

    /// ZIP extraction errors
    #[error("ZIP 에러: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// Tauri runtime errors
    #[error("Tauri 에러: {0}")]
    Tauri(#[from] tauri::Error),

    /// Tokio task join errors
    #[error("작업 조인 에러: {0}")]
    Join(#[from] tokio::task::JoinError),
}
