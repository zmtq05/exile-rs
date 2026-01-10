use std::{cell::LazyCell, convert::Infallible, fs, path::Path, str::FromStr, sync::LazyLock};

use chrono::{DateTime, Datelike, Utc};
use regex::Regex;
use serde::Serialize;
use specta::Type;

#[derive(Debug, Clone, Serialize, Type)]
#[serde(transparent)]
pub struct PobVersion(pub String);

impl FromStr for PobVersion {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // 1. "POB (yyyy.mm.dd).zip"
        static RE1: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\((\d{4})\.(\d{2})\.(\d{2})\)\.zip").unwrap());
        if let Some(caps) = RE1.captures(s) {
            let year = &caps[1];
            let month = &caps[2];
            let day = &caps[3];
            let version = format!("{}.{}.{}", year, month, day);
            return Ok(PobVersion(version));
        }

        // 2. yyyyMMdd
        static RE2: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(\d{4})(\d{2})(\d{2})").unwrap());
        if let Some(caps) = RE2.captures(s) {
            let year = &caps[1];
            let month = &caps[2];
            let day = &caps[3];
            let version = format!("{}.{}.{}", year, month, day);
            return Ok(PobVersion(version));
        }

        // 3. HTTP modified time format (yyyy-mm-ddThh:mm:ssZ)
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            let dt_utc: DateTime<Utc> = dt.with_timezone(&Utc);
            let version = format!(
                "{}.{:02}.{:02}",
                dt_utc.year(),
                dt_utc.month(),
                dt_utc.day()
            );
            return Ok(PobVersion(version));
        }

        // Fallback: now
        let now = Utc::now();
        let version = format!("{}.{}.{}", now.year(), now.month(), now.day());
        Ok(PobVersion(version))
    }
}