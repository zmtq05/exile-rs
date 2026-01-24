use std::path::Path;

use zip::DateTime;

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
