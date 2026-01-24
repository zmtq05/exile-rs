use serde::Serialize;
use specta::Type;

#[derive(Debug, Serialize, Type)]
#[serde(tag = "kind", content = "message")]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    PobError(String),
    TauriError(String),
    Io(String),
}

impl From<tauri::Error> for ErrorKind {
    fn from(err: tauri::Error) -> Self {
        ErrorKind::TauriError(err.to_string())
    }
}

impl From<crate::pob::error::PobError> for ErrorKind {
    fn from(err: crate::pob::error::PobError) -> Self {
        ErrorKind::PobError(err.to_string())
    }
}

impl From<std::io::Error> for ErrorKind {
    fn from(err: std::io::Error) -> Self {
        ErrorKind::Io(err.to_string())
    }
}
