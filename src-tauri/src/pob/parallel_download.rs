//! Parallel chunk download implementation for PoB files.
//!
//! Downloads large files by splitting them into chunks and downloading
//! each chunk in parallel using HTTP Range requests.

use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Instant,
};

use futures_util::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::{
    fs::File,
    io::{AsyncSeekExt, AsyncWriteExt, BufWriter},
    sync::Semaphore,
};
use tokio_util::sync::CancellationToken;

use crate::pob::{
    error::PobError,
    google_drive::{FileDownloadInfo, GoogleDriveClient},
    progress::{InstallPhase, InstallReporter, InstallStatus},
};

/// Download mode selection
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DownloadMode {
    /// Automatically choose based on file size and server support
    #[default]
    Auto,
    /// Force parallel chunk download (faster for high-speed networks)
    Parallel,
    /// Force single-stream download (more stable)
    Single,
}

/// Configuration for parallel downloads
#[derive(Debug, Clone)]
pub struct ParallelDownloadConfig {
    /// Number of concurrent chunk downloads
    pub concurrency: usize,
    /// Minimum file size to use parallel download (bytes)
    pub min_parallel_size: u64,
    /// Target chunk size (bytes)
    pub chunk_size: u64,
    /// Download mode override
    pub mode: DownloadMode,
}

impl Default for ParallelDownloadConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            min_parallel_size: 50 * 1024 * 1024,  // 50MB minimum (smaller files use single-stream)
            chunk_size: 128 * 1024 * 1024,        // 128MB chunks (reduce HTTP connection overhead)
            mode: DownloadMode::Auto,
        }
    }
}

/// Tracks download progress across all chunks
struct ProgressTracker {
    total_size: u64,
    downloaded: AtomicU64,
    reporter: InstallReporter,
    last_report: std::sync::Mutex<Instant>,
}

impl ProgressTracker {
    fn new(total_size: u64, reporter: InstallReporter) -> Self {
        Self {
            total_size,
            downloaded: AtomicU64::new(0),
            reporter,
            last_report: std::sync::Mutex::new(Instant::now()),
        }
    }

    fn add_progress(&self, bytes: u64) {
        let downloaded = self.downloaded.fetch_add(bytes, Ordering::Relaxed) + bytes;

        // Throttle reports to every 100ms
        let should_report = {
            let mut last = self.last_report.lock().unwrap();
            if last.elapsed().as_millis() >= 100 {
                *last = Instant::now();
                true
            } else {
                false
            }
        };

        if should_report {
            let percent = downloaded as f64 / self.total_size as f64 * 100.0;
            self.reporter
                .report(InstallPhase::Downloading, InstallStatus::InProgress { percent });
        }
    }
}

/// A chunk of the file to download
#[derive(Debug, Clone)]
struct Chunk {
    index: usize,
    start: u64,
    end: u64,
}

/// Parallel downloader for large files
pub struct ParallelDownloader<'a> {
    client: &'a GoogleDriveClient,
    config: ParallelDownloadConfig,
}

impl<'a> ParallelDownloader<'a> {
    pub fn new(client: &'a GoogleDriveClient, config: ParallelDownloadConfig) -> Self {
        Self { client, config }
    }

    /// Download a file using parallel chunks if supported
    pub async fn download(
        &self,
        file_id: &str,
        dst: &Path,
        cancel_token: CancellationToken,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        // Determine download strategy based on mode
        let use_parallel = match self.config.mode {
            DownloadMode::Single => {
                tracing::info!(
                    phase = "download",
                    mode = "single",
                    "Using single-stream download (user preference)"
                );
                false
            }
            DownloadMode::Parallel => {
                // Get file info to check if Range is supported
                let file_info = self.client.get_file_download_info(file_id).await?;
                if !file_info.accepts_ranges {
                    tracing::warn!(
                        phase = "download",
                        mode = "parallel",
                        "Parallel download requested but server doesn't support Range, falling back to single-stream"
                    );
                    return self
                        .download_single_stream(file_id, file_info.content_length, dst, cancel_token, reporter)
                        .await;
                }
                tracing::info!(
                    phase = "download",
                    mode = "parallel",
                    content_length = %file_info.content_length,
                    concurrency = %self.config.concurrency,
                    chunk_size = %self.config.chunk_size,
                    "Using parallel chunk download (user preference)"
                );
                return self
                    .download_parallel(file_id, &file_info, dst, cancel_token, reporter)
                    .await;
            }
            DownloadMode::Auto => {
                // Get file info to determine download strategy
                let file_info = self.client.get_file_download_info(file_id).await?;

                let should_parallel = file_info.accepts_ranges
                    && file_info.content_length >= self.config.min_parallel_size;

                if should_parallel {
                    tracing::info!(
                        phase = "download",
                        mode = "auto",
                        content_length = %file_info.content_length,
                        concurrency = %self.config.concurrency,
                        chunk_size = %self.config.chunk_size,
                        "Using parallel chunk download (auto-detected)"
                    );
                    return self
                        .download_parallel(file_id, &file_info, dst, cancel_token, reporter)
                        .await;
                } else {
                    tracing::info!(
                        phase = "download",
                        mode = "auto",
                        content_length = %file_info.content_length,
                        accepts_ranges = %file_info.accepts_ranges,
                        "Using single-stream download (auto: parallel not supported or file too small)"
                    );
                    return self
                        .download_single_stream(file_id, file_info.content_length, dst, cancel_token, reporter)
                        .await;
                }
            }
        };

        // Single mode fallback (no file info fetch needed for basic single stream)
        if !use_parallel {
            let res = self.client.get_file(file_id).await?;
            let total_size = res.content_length().unwrap_or(0);
            // Close response and use single stream method
            drop(res);
            return self
                .download_single_stream(file_id, total_size, dst, cancel_token, reporter)
                .await;
        }

        Ok(())
    }

    /// Download using parallel chunks
    async fn download_parallel(
        &self,
        file_id: &str,
        file_info: &FileDownloadInfo,
        dst: &Path,
        cancel_token: CancellationToken,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        let total_size = file_info.content_length;

        // Create chunks
        let chunks = self.create_chunks(total_size);
        let chunk_count = chunks.len();

        tracing::debug!(
            phase = "download",
            chunk_count = %chunk_count,
            "Created download chunks"
        );

        // Create destination file and pre-allocate
        let file = File::create(dst).await?;
        if let Err(e) = file.set_len(total_size).await {
            tracing::warn!(
                phase = "download",
                error = %e,
                "Failed to preallocate file size"
            );
        }
        let file = Arc::new(tokio::sync::Mutex::new(file));

        // Report start
        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::Started {
                total_size: std::num::NonZeroU32::new(total_size as u32),
            },
        );

        let start_time = Instant::now();
        let progress = Arc::new(ProgressTracker::new(total_size, reporter.clone()));
        let semaphore = Arc::new(Semaphore::new(self.config.concurrency));

        // Use FuturesUnordered to avoid 'static lifetime requirement
        let mut futures: FuturesUnordered<_> = chunks
            .into_iter()
            .map(|chunk| {
                let file = Arc::clone(&file);
                let progress = Arc::clone(&progress);
                let semaphore = Arc::clone(&semaphore);
                let cancel_token = cancel_token.clone();

                async move {
                    let _permit = semaphore.acquire().await.unwrap();

                    if cancel_token.is_cancelled() {
                        return Err(PobError::Cancelled);
                    }

                    download_chunk(
                        self.client,
                        file_id,
                        chunk,
                        &file,
                        &progress,
                        cancel_token,
                    )
                    .await
                }
            })
            .collect();

        // Process results as they complete
        let mut any_error: Option<PobError> = None;
        while let Some(result) = futures.next().await {
            if let Err(e) = result {
                if any_error.is_none() {
                    any_error = Some(e);
                }
                cancel_token.cancel();
            }
        }

        if let Some(e) = any_error {
            // Cancelled는 별도 상태로 emit
            if matches!(e, PobError::Cancelled) {
                tracing::info!(phase = "download", "Parallel download cancelled by user");
                reporter.report(InstallPhase::Downloading, InstallStatus::Cancelled);
            } else {
                tracing::error!(phase = "download", error = %e, "Parallel download failed");
                reporter.report(
                    InstallPhase::Downloading,
                    InstallStatus::Failed {
                        reason: e.to_string(),
                    },
                );
            }
            tokio::fs::remove_file(dst).await.ok();
            return Err(e);
        }

        tracing::info!(
            phase = "download",
            elapsed = ?start_time.elapsed(),
            "Parallel download completed"
        );
        reporter.report(InstallPhase::Downloading, InstallStatus::Completed);

        Ok(())
    }

    /// Single-stream download fallback
    async fn download_single_stream(
        &self,
        file_id: &str,
        total_size: u64,
        dst: &Path,
        cancel_token: CancellationToken,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        let res = self.client.get_file(file_id).await?;

        let f = File::create(dst).await?;
        if total_size > 0
            && let Err(e) = f.set_len(total_size).await
        {
            tracing::warn!(
                phase = "download",
                error = %e,
                "Failed to preallocate file size"
            );
        }

        reporter.report(
            InstallPhase::Downloading,
            InstallStatus::Started {
                total_size: std::num::NonZeroU32::new(total_size as u32),
            },
        );

        let start = Instant::now();
        let mut stream = res.bytes_stream();
        let mut writer = BufWriter::with_capacity(64 * 1024, f);

        let mut downloaded: u64 = 0;
        let mut last_report = start;

        loop {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    tracing::info!(phase = "download", "Download cancelled");
                    reporter.report(InstallPhase::Downloading, InstallStatus::Cancelled);
                    drop(writer);
                    tokio::fs::remove_file(dst).await.ok();
                    return Err(PobError::Cancelled);
                }
                chunk = stream.next() => {
                    match chunk {
                        Some(Ok(bytes)) => {
                            writer.write_all(&bytes).await?;
                            downloaded += bytes.len() as u64;

                            if last_report.elapsed().as_millis() < 100 {
                                continue;
                            }
                            let percent = if total_size > 0 {
                                downloaded as f64 / total_size as f64 * 100.0
                            } else {
                                0.0
                            };
                            reporter.report(InstallPhase::Downloading, InstallStatus::InProgress { percent });
                            last_report = Instant::now();
                        }
                        Some(Err(e)) => {
                            tracing::error!(phase = "download", error = %e, "Error while downloading");
                            reporter.report(InstallPhase::Downloading, InstallStatus::Failed { reason: e.to_string() });
                            return Err(PobError::DownloadFailed(e.to_string()));
                        }
                        None => {
                            writer.flush().await?;
                            tracing::info!(phase = "download", elapsed = ?start.elapsed(), "Download completed");
                            reporter.report(InstallPhase::Downloading, InstallStatus::Completed);
                            return Ok(());
                        }
                    }
                }
            }
        }
    }

    /// Create chunks for parallel download
    fn create_chunks(&self, total_size: u64) -> Vec<Chunk> {
        let chunk_size = self.config.chunk_size;
        let mut chunks = Vec::new();
        let mut start = 0u64;
        let mut index = 0usize;

        while start < total_size {
            let end = std::cmp::min(start + chunk_size - 1, total_size - 1);
            chunks.push(Chunk { index, start, end });
            start = end + 1;
            index += 1;
        }

        chunks
    }
}

/// Download a single chunk and write to file at correct offset
async fn download_chunk(
    client: &GoogleDriveClient,
    file_id: &str,
    chunk: Chunk,
    file: &tokio::sync::Mutex<File>,
    progress: &ProgressTracker,
    cancel_token: CancellationToken,
) -> Result<(), PobError> {
    let chunk_start_time = Instant::now();

    tracing::debug!(
        phase = "download",
        chunk_index = %chunk.index,
        start = %chunk.start,
        end = %chunk.end,
        "Starting chunk download"
    );

    let http_start = Instant::now();
    let res = client.get_file_range(file_id, chunk.start, chunk.end).await?;
    let http_elapsed = http_start.elapsed();

    tracing::debug!(
        phase = "download",
        chunk_index = %chunk.index,
        http_connect_ms = %http_elapsed.as_millis(),
        "HTTP Range request established"
    );

    let mut stream = res.bytes_stream();

    // Buffer the ENTIRE chunk in memory, then write once
    let chunk_size = (chunk.end - chunk.start + 1) as usize;
    let mut buffer = Vec::with_capacity(chunk_size);

    let stream_start = Instant::now();
    while let Some(result) = stream.next().await {
        if cancel_token.is_cancelled() {
            return Err(PobError::Cancelled);
        }

        let bytes = result.map_err(|e| PobError::DownloadFailed(e.to_string()))?;
        buffer.extend_from_slice(&bytes);

        // Report progress during streaming (no lock needed)
        progress.add_progress(bytes.len() as u64);
    }
    let stream_elapsed = stream_start.elapsed();

    // Single write at the end
    let write_start = Instant::now();
    {
        let mut file = file.lock().await;
        file.seek(std::io::SeekFrom::Start(chunk.start)).await?;
        file.write_all(&buffer).await?;
    }
    let write_elapsed = write_start.elapsed();

    tracing::info!(
        phase = "download",
        chunk_index = %chunk.index,
        chunk_size_mb = format!("{:.2}", chunk_size as f64 / 1024.0 / 1024.0),
        http_connect_ms = %http_elapsed.as_millis(),
        stream_ms = %stream_elapsed.as_millis(),
        write_ms = %write_elapsed.as_millis(),
        total_ms = %chunk_start_time.elapsed().as_millis(),
        "Chunk download completed"
    );

    Ok(())
}
