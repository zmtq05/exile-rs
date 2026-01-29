use std::path::Path;

use zip::DateTime;

/// Generate a unique task ID with prefix, timestamp, and random suffix.
/// Format: `{prefix}_{timestamp_hex}_{random_hex}`
/// Example: `pob_18abc1234def_a3f2`
pub fn generate_task_id(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();

    // Simple random using timestamp nanoseconds as seed
    let random: u16 = {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        (nanos & 0xFFFF) as u16
    };

    format!("{prefix}_{timestamp:x}_{random:04x}")
}

pub async fn async_copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    tokio::fs::create_dir_all(dst).await?;
    let mut entries = tokio::fs::read_dir(src).await?;

    while let Some(entry) = entries.next_entry().await? {
        let ty = entry.file_type().await?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            Box::pin(async_copy_dir_recursive(&src_path, &dst_path)).await?;
        } else {
            tokio::fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

/// Generate a [`NaiveDateTime`] from a [`DateTime`].
///
/// [`NaiveDateTime`]: chrono::NaiveDateTime
// Ref: https://docs.rs/zip/7.2.0/src/zip/read.rs.html#2238-2253
fn generate_chrono_datetime(datetime: &DateTime) -> Option<chrono::NaiveDateTime> {
    if let Some(d) = chrono::NaiveDate::from_ymd_opt(
        datetime.year().into(),
        datetime.month().into(),
        datetime.day().into(),
    ) && let Some(d) = d.and_hms_opt(
        datetime.hour().into(),
        datetime.minute().into(),
        datetime.second().into(),
    ) {
        return Some(d);
    }
    None
}

/// Generate a [`SystemTime`] from a [`DateTime`].
///
/// [`SystemTime`]: std::time::SystemTime
// Ref: https://docs.rs/zip/7.2.0/src/zip/read.rs.html#2227-2234
pub fn datetime_to_systemtime(datetime: &DateTime) -> Option<std::time::SystemTime> {
    if let Some(t) = generate_chrono_datetime(datetime) {
        let time = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(t, chrono::Utc);
        return Some(time.into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_task_id_format() {
        let task_id = generate_task_id("test");

        // Format: {prefix}_{timestamp_hex}_{random_hex}
        let parts: Vec<&str> = task_id.split('_').collect();
        assert_eq!(parts.len(), 3, "Task ID should have 3 parts");
        assert_eq!(parts[0], "test", "Prefix should match");

        // Timestamp should be hex
        assert!(
            u128::from_str_radix(parts[1], 16).is_ok(),
            "Timestamp should be hex"
        );

        // Random should be 4-digit hex
        assert_eq!(parts[2].len(), 4, "Random suffix should be 4 characters");
        assert!(
            u16::from_str_radix(parts[2], 16).is_ok(),
            "Random should be hex"
        );
    }

    #[test]
    fn test_generate_task_id_uniqueness() {
        let id1 = generate_task_id("test");
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_task_id("test");

        assert_ne!(id1, id2, "Task IDs should be unique");
    }

    #[test]
    fn test_generate_task_id_prefix() {
        let prefixes = vec!["pob", "task", "download", "extract"];

        for prefix in prefixes {
            let task_id = generate_task_id(prefix);
            assert!(
                task_id.starts_with(prefix),
                "Task ID should start with prefix: {}",
                prefix
            );
        }
    }

    #[test]
    fn test_datetime_to_systemtime_valid() {
        // Valid datetime: 2024-05-20 14:30:00
        let datetime = DateTime::from_date_and_time(2024, 5, 20, 14, 30, 0).unwrap();
        let result = datetime_to_systemtime(&datetime);

        assert!(
            result.is_some(),
            "Valid datetime should convert successfully"
        );
    }

    #[test]
    fn test_datetime_to_systemtime_epoch() {
        // Epoch: 1980-01-01 00:00:00 (ZIP epoch)
        let datetime = DateTime::from_date_and_time(1980, 1, 1, 0, 0, 0).unwrap();
        let result = datetime_to_systemtime(&datetime);

        assert!(result.is_some(), "ZIP epoch should convert successfully");
    }

    #[test]
    fn test_generate_chrono_datetime_valid() {
        use chrono::{Datelike, Timelike};

        // ZIP DateTime uses 2-second precision, so 59 seconds becomes 58
        let datetime = DateTime::from_date_and_time(2024, 12, 25, 23, 59, 58).unwrap();
        let result = generate_chrono_datetime(&datetime);

        assert!(result.is_some());
        let chrono_dt = result.unwrap();
        assert_eq!(chrono_dt.year(), 2024);
        assert_eq!(chrono_dt.month(), 12);
        assert_eq!(chrono_dt.day(), 25);
        assert_eq!(chrono_dt.hour(), 23);
        assert_eq!(chrono_dt.minute(), 59);
        assert_eq!(chrono_dt.second(), 58); // ZIP has 2-second precision
    }

    #[tokio::test]
    async fn test_async_copy_dir_recursive() {
        use tempfile::tempdir;

        let temp = tempdir().unwrap();
        let src = temp.path().join("src");
        let dst = temp.path().join("dst");

        // Create source directory structure
        tokio::fs::create_dir_all(&src).await.unwrap();
        tokio::fs::create_dir_all(src.join("subdir")).await.unwrap();
        tokio::fs::write(src.join("file1.txt"), b"content1")
            .await
            .unwrap();
        tokio::fs::write(src.join("subdir/file2.txt"), b"content2")
            .await
            .unwrap();

        // Copy
        let result = async_copy_dir_recursive(&src, &dst).await;
        assert!(result.is_ok());

        // Verify
        assert!(dst.exists());
        assert!(dst.join("file1.txt").exists());
        assert!(dst.join("subdir").exists());
        assert!(dst.join("subdir/file2.txt").exists());

        let content1 = tokio::fs::read_to_string(dst.join("file1.txt"))
            .await
            .unwrap();
        let content2 = tokio::fs::read_to_string(dst.join("subdir/file2.txt"))
            .await
            .unwrap();
        assert_eq!(content1, "content1");
        assert_eq!(content2, "content2");
    }
}
