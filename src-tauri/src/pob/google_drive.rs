use reqwest::{Response, header};
use serde::{Deserialize, Serialize};
use specta::Type;

use crate::pob::error::PobError;

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GoogleDriveFileInfo {
    pub id: String,
    pub name: String,
    pub is_folder: bool,
}

/// Information about a file for download planning
#[derive(Debug, Clone)]
pub struct FileDownloadInfo {
    /// Total file size in bytes
    pub content_length: u64,
    /// Whether the server supports Range requests
    pub accepts_ranges: bool,
    /// The actual download URL (after redirects)
    pub download_url: String,
}

pub struct GoogleDriveClient {
    inner: reqwest::Client,
}

impl GoogleDriveClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { inner: client }
    }

    pub async fn fetch_folder(
        &self,
        folder_id: &str,
    ) -> Result<Vec<GoogleDriveFileInfo>, PobError> {
        let url = format!("https://drive.google.com/drive/folders/{}", folder_id);
        let res = self.inner.get(url).send().await?.error_for_status()?;

        let body = res.text().await?;

        let files = html_parser::parse_google_drive_folder_html(&body);

        if files.is_empty() {
            tracing::error!(
                folder_id = %folder_id,
                html_sample = &body[..body.len().min(500)],
                "No files found in Google Drive folder or failed to parse HTML - Google Drive UI may have changed"
            );

            return Err(PobError::NotFoundFromDrive(
                "Google Drive 폴더에서 파일을 찾을 수 없거나 HTML 파싱에 실패했습니다".to_string(),
            ));
        }
        Ok(files)
    }

    pub async fn find_latest(
        &self,
        folder_id: &str,
    ) -> Result<Option<GoogleDriveFileInfo>, PobError> {
        let mut files = self.fetch_folder(folder_id).await?;
        files.retain(|f| !f.is_folder);

        files.sort_by(|a, b| b.name.cmp(&a.name));

        Ok(files.into_iter().next())
    }

    pub async fn get_file(&self, file_id: &str) -> Result<Response, PobError> {
        let url = format!(
            "https://drive.usercontent.google.com/download?confirm=t&id={}",
            file_id
        );
        let res = self.inner.get(url).send().await?.error_for_status()?;

        Ok(res)
    }

    /// Get file download info (size, Range support) via HEAD request
    /// Google Drive redirects, so we follow redirects and check the final response
    pub async fn get_file_download_info(
        &self,
        file_id: &str,
    ) -> Result<FileDownloadInfo, PobError> {
        let url = format!(
            "https://drive.usercontent.google.com/download?confirm=t&id={}",
            file_id
        );

        // First do a GET with Range header to check if Range is supported
        // HEAD requests don't always work with Google Drive
        let res = self
            .inner
            .get(&url)
            .header(header::RANGE, "bytes=0-0")
            .send()
            .await?
            .error_for_status()?;

        let status = res.status();
        let headers = res.headers().clone();
        let final_url = res.url().to_string();

        // Check if server supports Range requests
        // 206 Partial Content means Range is supported
        let accepts_ranges = status == reqwest::StatusCode::PARTIAL_CONTENT
            || headers
                .get(header::ACCEPT_RANGES)
                .and_then(|v| v.to_str().ok())
                .is_some_and(|v| v != "none");

        // Get content length from Content-Range header (for 206) or Content-Length
        let content_length = if status == reqwest::StatusCode::PARTIAL_CONTENT {
            // Content-Range: bytes 0-0/12345678
            headers
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.split('/').next_back())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        } else {
            headers
                .get(header::CONTENT_LENGTH)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(0)
        };

        tracing::debug!(
            file_id = %file_id,
            content_length = %content_length,
            accepts_ranges = %accepts_ranges,
            status = %status,
            "File download info retrieved"
        );

        Ok(FileDownloadInfo {
            content_length,
            accepts_ranges,
            download_url: final_url,
        })
    }

    /// Download a specific byte range of a file
    pub async fn get_file_range(
        &self,
        file_id: &str,
        start: u64,
        end: u64,
    ) -> Result<Response, PobError> {
        let url = format!(
            "https://drive.usercontent.google.com/download?confirm=t&id={}",
            file_id
        );

        let range_header = format!("bytes={}-{}", start, end);
        let res = self
            .inner
            .get(url)
            .header(header::RANGE, range_header)
            .send()
            .await?
            .error_for_status()?;

        Ok(res)
    }
}

mod html_parser {
    use std::sync::LazyLock;

    use scraper::Selector;

    use crate::pob::google_drive::GoogleDriveFileInfo;

    static ROW_SELECTOR: LazyLock<Selector> =
        LazyLock::new(|| Selector::parse("tbody > tr").unwrap());
    static NAME_SELECTOR: LazyLock<Selector> = LazyLock::new(|| Selector::parse("strong").unwrap());
    static SIZE_SELECTOR: LazyLock<Selector> =
        LazyLock::new(|| Selector::parse("td[data-column-field=\"3\"] [aria-label]").unwrap());

    pub fn parse_google_drive_folder_html(html: &str) -> Vec<GoogleDriveFileInfo> {
        scraper::Html::parse_document(html)
            .select(&ROW_SELECTOR)
            .filter_map(parse_row)
            .collect()
    }

    fn parse_row(row: scraper::ElementRef) -> Option<GoogleDriveFileInfo> {
        let id = row.value().attr("data-id")?;

        let name = row.select(&NAME_SELECTOR).next()?.text().next()?;

        let is_folder = row
            .select(&SIZE_SELECTOR)
            .next()
            .and_then(|e| e.attr("aria-label"))
            .map(|label| label.contains("not available"))?;

        Some(GoogleDriveFileInfo {
            id: id.to_string(),
            name: name.to_string(),
            is_folder,
        })
    }
}
