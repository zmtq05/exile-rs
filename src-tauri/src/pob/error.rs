#[derive(Debug, thiserror::Error)]
pub enum PobError {
    #[error("다운로드 실패: {0}")]
    DownloadFailed(String),

    #[error("압축 해제 실패: {0}")]
    ExtractFailed(String),

    #[error("PoB가 실행 중입니다 (PoeCharm3.exe)")]
    ProcessRunning,

    #[error("백업 실패: {0}")]
    BackupFailed(String),

    #[error("설치 실패: {0}")]
    InstallFailed(String),

    #[error("복원 실패: {0}")]
    RestoreFailed(String),

    #[error("네트워크 에러: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("I/O 에러: {0}")]
    IoError(#[from] std::io::Error),

    #[error("경로 오류: {0}")]
    PathError(String),

    #[error("버전 파싱 실패: {0}")]
    VersionParseError(String),

    #[error("설치가 취소되었습니다")]
    Cancelled,

    #[error("JSON 파싱 에러: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("ZIP 에러: {0}")]
    ZipError(#[from] zip::result::ZipError),

    #[error("Tauri 에러: {0}")]
    TauriError(#[from] tauri::Error),

    #[error("Google Drive에서 파일을 찾을 수 없습니다: {0}")]
    NotFoundFromDrive(String),

    #[error("작업 조인 에러: {0}")]
    JoinError(#[from] tokio::task::JoinError),
}
