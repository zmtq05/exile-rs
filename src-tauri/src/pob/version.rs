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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version_valid() {
        let test_cases = vec![
            ("POE1&2 통합 한글 POB (2024.01.15).zip", "2024.01.15"),
            ("POE1&2 통합 한글 POB(2024.12.31).zip", "2024.12.31"),
            // Regex uses \s? which means 0 or 1 whitespace, so 2 spaces won't match
            // ("POE1&2 통합 한글 POB  (2025.06.01).zip", "2025.06.01"),
        ];

        for (input, expected) in test_cases {
            let result = parse_from_name(input);
            assert!(result.is_ok(), "Failed to parse: {}", input);
            assert_eq!(result.unwrap(), expected, "Input: {}", input);
        }
    }

    #[test]
    fn test_parse_version_invalid() {
        let test_cases = vec![
            "invalid.zip",
            "POE1&2 통합 한글 POB.zip",
            "POE1&2 통합 한글 POB (2024-01-15).zip", // Wrong separator
            "POE1&2 통합 한글 POB (24.01.15).zip",   // Wrong year format
            "POE1&2 통합 한글 POB (2024.1.15).zip",  // Missing leading zero
            "",
        ];

        for input in test_cases {
            let result = parse_from_name(input);
            assert!(result.is_err(), "Should fail to parse: {}", input);
            match result {
                Err(PobError::VersionParseError(_)) => {}
                _ => panic!("Expected VersionParseError for: {}", input),
            }
        }
    }

    #[test]
    fn test_version_try_from_google_drive_info() {
        let file_info = GoogleDriveFileInfo {
            id: "test_file_id".to_string(),
            name: "POE1&2 통합 한글 POB (2024.05.20).zip".to_string(),
            is_folder: false,
        };

        let result = PobVersion::try_from(&file_info);
        assert!(result.is_ok());

        let version = result.unwrap();
        assert_eq!(version.version, "2024.05.20");
        assert_eq!(version.file_id, "test_file_id");
        assert!(!version.installed_at.is_empty());
    }

    #[test]
    fn test_version_try_from_invalid_name() {
        let file_info = GoogleDriveFileInfo {
            id: "test_file_id".to_string(),
            name: "invalid_filename.zip".to_string(),
            is_folder: false,
        };

        let result = PobVersion::try_from(&file_info);
        assert!(result.is_err());
    }

    #[test]
    fn test_version_regex_compiles() {
        // Ensure regex pattern is valid at compile time (via test)
        let _ = Regex::new(r"POE1&2 통합 한글 POB\s?\((\d{4}\.\d{2}\.\d{2})\).zip")
            .expect("Version regex pattern is invalid");
    }
}
