mod proxy;

pub use proxy::{PROXY_URL, fetch_remote_pob};

use std::path::{Path, PathBuf};

use anyhow::Context;
use futures_util::StreamExt;
use tokio::io::AsyncWriteExt;

pub async fn download_with_progress(
    client: reqwest::Client,
    file_id: &str,
    save_path: &Path,
    progress_callback: impl Fn(u32, u32),
) -> anyhow::Result<PathBuf> {
    let file_name = save_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

    let resp = match download_file(client.clone(), file_id).await { // use direct link first for speed
        Ok(response) => response,
        Err(_) => proxy::download_file(client.clone(), file_id, file_name).await?, // fallback to Google Drive API
    };

    let total_bytes = resp
        .content_length()
        .ok_or_else(|| anyhow::anyhow!("Failed to get content length"))?;

    let total_bytes: u32 = total_bytes
        .try_into()
        .unwrap_or_else(|_| {
            log::warn!("File size exceeds u32 limit, capping to u32::MAX");
            u32::MAX
        });

    let mut downloaded: u32 = 0;
    let f = tokio::fs::File::create(save_path)
        .await
        .context("Failed to create file")?;
    let mut writer = tokio::io::BufWriter::new(f);

    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("Failed to read chunk")?;
        writer
            .write_all(&chunk)
            .await
            .context("Failed to write chunk to file")?;
        downloaded += chunk.len() as u32;
        progress_callback(downloaded, total_bytes);
    }
    writer.flush().await.context("Failed to flush writer")?;

    Ok(save_path.to_path_buf())
}

async fn download_file(
    client: reqwest::Client,
    file_id: &str,
) -> anyhow::Result<reqwest::Response> {
    let url = format!(
        "https://drive.usercontent.google.com/download?confirm=t&id={}",
        file_id
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to send request to download URL")?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("Direct download request returned error status: {}", status);
    }

    Ok(resp)
}
