use reqwest::Response;
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

        Ok(html_parser::parse_google_drive_folder_html(&body))
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
