use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::pob::{error::PobError, google_drive::GoogleDriveFileInfo};

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PobVersion {
    pub version: String,
    pub installed_at: String,
    pub file_id: String,
}

impl TryFrom<GoogleDriveFileInfo> for PobVersion {
    type Error = PobError;

    fn try_from(value: GoogleDriveFileInfo) -> Result<Self, Self::Error> {
        Self::try_from(&value)
    }
}

impl TryFrom<&GoogleDriveFileInfo> for PobVersion {
    type Error = PobError;

    fn try_from(value: &GoogleDriveFileInfo) -> Result<Self, Self::Error> {
        let version = parse_from_name(&value.name)?;
        let installed_at = chrono::Utc::now().to_rfc3339();

        Ok(Self {
            version,
            installed_at,
            file_id: value.id.clone(),
        })
    }
}

pub fn parse_from_name(name: &str) -> Result<String, PobError> {
    static RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"POE1&2 통합 한글 POB\s?\((\d{4}\.\d{2}\.\d{2})\).zip").unwrap()
    });

    RE.captures(name)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| PobError::VersionParseError(name.to_string()))
}
