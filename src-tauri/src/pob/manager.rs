use std::{
    collections::HashMap,
    num::NonZeroU32,
    path::{Path, PathBuf},
    time::Instant,
};

use tokio::{fs, sync::Mutex, sync::RwLock};
use tokio_util::sync::CancellationToken;

use crate::{
    pob::{
        error::PobError,
        google_drive::{GoogleDriveClient, GoogleDriveFileInfo},
        progress::{InstallPhase, InstallReporter, InstallStatus},
        version::PobVersion,
    },
    util::{async_copy_dir_recursive, datetime_to_systemtime},
};

pub struct PobManager {
    client: GoogleDriveClient,
    data_dir: PathBuf,

    cached_result: Mutex<HashMap<String, GoogleDriveFileInfo>>,

    /// Lock for mutating operations (install, uninstall).
    /// Write lock = exclusive access for install/uninstall.
    operation_lock: RwLock<()>,
}

impl PobManager {
    pub fn new(client: GoogleDriveClient, data_dir: PathBuf) -> Self {
        Self {
            client,
            data_dir,
            cached_result: Mutex::new(HashMap::new()),
            operation_lock: RwLock::new(()),
        }
    }

    /// Try to acquire exclusive lock for install/uninstall.
    /// Returns guard on success, None if another operation in progress.
    pub fn try_write_lock(&self) -> Option<tokio::sync::RwLockWriteGuard<'_, ()>> {
        self.operation_lock.try_write().ok()
    }

    pub fn install_path(&self) -> PathBuf {
        self.data_dir.join("PoeCharm")
    }

    pub fn version_file_path(&self) -> PathBuf {
        self.install_path().join("pob_version.json")
    }

    pub fn backup_dir(&self) -> PathBuf {
        self.data_dir.join("backup")
    }

    pub fn exe_path(&self) -> PathBuf {
        self.install_path().join("PoeCharm3.exe")
    }

    pub fn pob_version_file_path(&self) -> PathBuf {
        self.install_path().join("pob_version.json")
    }

    pub async fn fetch_latest_file(
        &self,
        force_refresh: bool,
    ) -> Result<GoogleDriveFileInfo, PobError> {
        // Currently, hardcodeing the folder ID
        const FOLDER_ID: &str = "1_5YhTy59gkyJpWqPuKA_z1cnobQcS8gi";

        if !force_refresh {
            let cache = self.cached_result.lock().await;
            if let Some(cached) = cache.get(FOLDER_ID) {
                return Ok(cached.clone());
            }
        }

        let latest = self.client.find_latest(FOLDER_ID).await?;

        let latest = latest.ok_or_else(|| PobError::NotFoundFromDrive(FOLDER_ID.to_string()))?;

        let mut cache = self.cached_result.lock().await;
        cache.insert(FOLDER_ID.to_string(), latest.clone());

        Ok(latest)
    }

    pub async fn installed_version(
        &self,
    ) -> Result<Option<crate::pob::version::PobVersion>, PobError> {
        let path = self.version_file_path();
        if !path.exists() {
            return Ok(None);
        }
        let data = tokio::fs::read_to_string(&path).await?;
        let installed: crate::pob::version::PobVersion = serde_json::from_str(&data)?;
        Ok(Some(installed))
    }

    pub(crate) async fn download_with_progress<P: AsRef<std::path::Path>>(
        &self,
        file_id: &str,
        dst: P,
        cancel_token: CancellationToken,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        use futures_util::StreamExt;
        use tokio::io::{AsyncWriteExt, BufWriter};

        let res = self.client.get_file(file_id).await?;
        let total_size = res.content_length().unwrap_or(0);

        let f = tokio::fs::File::create(dst.as_ref()).await?;
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
                total_size: NonZeroU32::new(total_size as u32),
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
                    tokio::fs::remove_file(dst.as_ref()).await.ok();
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

    pub(crate) async fn extract_with_progress<P: AsRef<std::path::Path>>(
        &self,
        zip_path: P,
        dest_path: P,
        cancel_token: CancellationToken,
        reporter: InstallReporter,
    ) -> Result<(), PobError> {
        if dest_path.as_ref().exists() {
            tokio::fs::remove_dir_all(&dest_path).await?;
        }
        tokio::fs::create_dir_all(&dest_path).await?;

        let zip_path = zip_path.as_ref().to_path_buf();
        let dest_path = dest_path.as_ref().to_path_buf();

        let task = tokio::task::spawn_blocking(move || -> Result<(), PobError> {
            let f = std::fs::File::open(&zip_path)?;
            let mut archive = zip::ZipArchive::new(f)?;
            let file_count = archive.len() as u32;

            // Detect nested structure BEFORE extraction
            let skip_prefix = detect_nested_structure(&archive)?;
            if let Some(ref prefix) = skip_prefix {
                tracing::warn!(
                    phase = "extract",
                    prefix = %prefix.display(),
                    "Detected nested directory structure, will strip prefix during extraction"
                );
            }

            reporter.report(
                InstallPhase::Extracting,
                InstallStatus::Started {
                    total_size: NonZeroU32::new(file_count),
                },
            );
            let mut last_report = Instant::now();

            for i in 0..file_count {
                if cancel_token.is_cancelled() {
                    tracing::info!(phase = "extract", "Extraction cancelled");
                    reporter.report(InstallPhase::Extracting, InstallStatus::Cancelled);
                    if let Err(e) = std::fs::remove_dir_all(&dest_path) {
                        tracing::warn!(
                            phase = "extract",
                            path = %dest_path.display(),
                            error = %e,
                            "Failed to remove partially extracted directory"
                        );
                    }
                    return Err(PobError::Cancelled);
                }

                let mut file = archive.by_index(i as usize)?;

                let Some(outpath) = file.enclosed_name() else {
                    tracing::warn!(
                        phase = "extract",
                        name = file.name(),
                        "Skipping dangerous path"
                    );
                    continue;
                };

                // Apply prefix removal if nested structure detected
                let final_path = if let Some(ref prefix) = skip_prefix {
                    outpath
                        .strip_prefix(prefix)
                        .map(Path::to_path_buf)
                        .unwrap_or(outpath)
                } else {
                    outpath
                };

                let outpath = dest_path.join(final_path);

                if file.is_dir() {
                    std::fs::create_dir_all(&outpath)?;
                } else {
                    if let Some(p) = outpath.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    let mut outfile = std::fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;

                    if let Some(last_modified) = file.last_modified()
                        && let Some(t) = datetime_to_systemtime(&last_modified)
                    {
                        outfile.set_modified(t)?;
                    }
                }

                if last_report.elapsed().as_millis() < 100 {
                    continue;
                }
                let percent = (i + 1) as f64 / file_count as f64 * 100.0;
                reporter.report(
                    InstallPhase::Extracting,
                    InstallStatus::InProgress { percent },
                );
                last_report = Instant::now();
            }

            reporter.report(InstallPhase::Extracting, InstallStatus::Completed);
            Ok(())
        });

        task.await?
    }

    pub(crate) async fn backup(&self, reporter: &InstallReporter) -> Result<(), PobError> {
        tracing::info!(phase = "backup", "Starting backup");
        reporter.report(
            InstallPhase::BackingUp,
            InstallStatus::Started { total_size: None },
        );

        let install_path = self.install_path();
        tracing::debug!(
            phase = "backup",
            install_path = %install_path.display(),
            exists = %install_path.exists(),
            "Backup source path"
        );

        // write to `<backup_dir>/backup.new`
        let existing_backup = self.backup_dir();
        let backup_path = self.backup_dir().with_extension("new");
        tracing::debug!(
            phase = "backup",
            backup_new = %backup_path.display(),
            existing_backup = %existing_backup.display(),
            "Backup paths determined"
        );

        // Ensure backup.new directory exists (especially for first install)
        if backup_path.exists() {
            tokio::fs::remove_dir_all(&backup_path).await?;
        }
        tokio::fs::create_dir_all(&backup_path).await?;
        tracing::debug!(phase = "backup", path = %backup_path.display(), "Created backup.new directory");

        for relative_path in self.backup_targets() {
            let absolute_path = install_path.join(&relative_path);
            if !absolute_path.exists() {
                tracing::debug!(phase = "backup", path = %relative_path.display(), "Backup target does not exist, skipping");
                continue;
            }

            let backup_target_path = backup_path.join(&relative_path);

            if absolute_path.is_dir() {
                async_copy_dir_recursive(&absolute_path, &backup_target_path).await?;
            } else {
                if let Some(parent) = backup_target_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::copy(&absolute_path, &backup_target_path).await?;
            }
        }
        tracing::info!(phase = "backup", "Backup copy completed");
        reporter.report(InstallPhase::BackingUp, InstallStatus::Completed);

        // finalize: swap backup.new -> backup (with .old staging if exists)
        let old = existing_backup.with_extension("old");
        tracing::debug!(
            phase = "backup",
            backup_new = %backup_path.display(),
            existing = %existing_backup.display(),
            existing_exists = %existing_backup.exists(),
            old = %old.display(),
            "Finalizing backup swap"
        );

        if existing_backup.exists() {
            tracing::debug!(phase = "backup", "Moving existing backup to .old");
            fs::rename(&existing_backup, &old).await?;
        }
        tracing::debug!(phase = "backup", "Moving backup.new to backup");
        fs::rename(&backup_path, &existing_backup).await?;
        if old.exists() {
            tracing::debug!(phase = "backup", "Cleaning up backup.old");
            fs::remove_dir_all(&old).await.ok(); // best-effort cleanup
        }
        tracing::info!(phase = "backup", "Backup finalized");

        Ok(())
    }

    pub(crate) fn backup_targets(&self) -> Vec<PathBuf> {
        const TARGETS: &[&str] = &[
            "POE1 POB/Builds",
            "POE2 POB/Builds",
            "POE1 POB/Settings.xml",
            "POE2 POB/Settings.xml",
            "Data/Fonts",
        ];

        TARGETS.iter().map(PathBuf::from).collect()
    }

    pub(crate) async fn restore(&self, reporter: &InstallReporter) -> Result<(), PobError> {
        tracing::info!(phase = "restore", "Starting restore from backup");
        reporter.report(
            InstallPhase::Restoring,
            InstallStatus::Started { total_size: None },
        );

        let install_path = self.install_path();
        let backup_path = self.backup_dir();

        if !backup_path.exists() {
            tracing::warn!(
                phase = "restore",
                "No backup directory found, skipping restore (likely first install)"
            );
            reporter.report(InstallPhase::Restoring, InstallStatus::Completed);
            return Ok(());
        }

        let target_paths: Vec<PathBuf> = self.backup_targets();

        for relative_path in target_paths {
            let backup_target_path = backup_path.join(&relative_path);
            if !backup_target_path.exists() {
                tracing::debug!(phase = "restore", path = %relative_path.display(), "Backup target does not exist, skipping");
                continue;
            }

            let restore_target_path = install_path.join(&relative_path);

            if backup_target_path.is_dir() {
                async_copy_dir_recursive(&backup_target_path, &restore_target_path).await?;
            } else {
                if let Some(parent) = restore_target_path.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::copy(&backup_target_path, &restore_target_path).await?;
            }
        }
        tracing::info!(phase = "restore", "Restore completed");
        reporter.report(InstallPhase::Restoring, InstallStatus::Completed);

        Ok(())
    }

    pub(crate) async fn save_version_info(&self, version: &PobVersion) -> Result<(), PobError> {
        let path = self.pob_version_file_path();
        let data = serde_json::to_string_pretty(version)?;
        tokio::fs::write(&path, data).await?;
        Ok(())
    }

    pub(crate) async fn rename(
        &self,
        extracted: &Path,
        install_dir: &Path,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        tracing::info!(
            phase = "rename",
            from = %extracted.display(),
            to = %install_dir.display(),
            "rename() called"
        );

        reporter.report(
            InstallPhase::Moving,
            InstallStatus::Started { total_size: None },
        );

        // move existing to .old
        let old = install_dir.with_extension("old");
        tracing::debug!(
            phase = "rename",
            install_dir = %install_dir.display(),
            exists = %install_dir.exists(),
            old = %old.display(),
            "Checking if install_dir exists"
        );

        if install_dir.exists() {
            tracing::info!(phase = "rename", "Moving existing install to .old");

            // Remove orphaned .old directory from previous failed installation
            if old.exists() {
                tracing::warn!(
                    phase = "rename",
                    path = %old.display(),
                    "Removing orphaned .old directory from previous failed installation"
                );
                tokio::fs::remove_dir_all(&old).await?;
            }

            tokio::fs::rename(install_dir, &old).await?;
            tracing::info!(phase = "rename", "Existing install moved to .old");
        } else {
            tracing::info!(
                phase = "rename",
                "No existing install, skipping .old rename"
            );
        }

        // move new in place
        tracing::debug!(
            phase = "rename",
            from = %extracted.display(),
            to = %install_dir.display(),
            from_exists = %extracted.exists(),
            "Attempting to rename extracted to install_dir"
        );

        // NOTE: Cross-device fallback은 현재 불필요 (모두 app_local_data_dir 내부)
        // 향후 커스텀 설치 경로 지원 시 async_copy_dir_recursive fallback 추가 필요
        tokio::fs::rename(extracted, install_dir).await?;
        tracing::info!(
            phase = "rename",
            install_dir = %install_dir.display(),
            exists_after = %install_dir.exists(),
            "Rename completed"
        );

        reporter.report(InstallPhase::Moving, InstallStatus::Completed);

        Ok(())
    }
}

/// Detect if ZIP has nested directory structure (e.g., PoeCharm/POE1 POB/...)
/// Returns the prefix to skip, or None if structure is flat
fn detect_nested_structure(
    archive: &zip::ZipArchive<std::fs::File>,
) -> Result<Option<PathBuf>, PobError> {
    const REQUIRED: &[&str] = &["POE1 POB/", "POE2 POB/", "Data/"];

    // Check first occurrence of any required folder
    for name in archive.file_names() {
        for &required_folder in REQUIRED {
            if let Some(pos) = name.find(required_folder) {
                if pos == 0 {
                    // "POE1 POB/..." - top level, OK
                    tracing::info!(
                        phase = "extract",
                        "ZIP structure validated: top-level folders found"
                    );
                    return Ok(None);
                } else {
                    // "PoeCharm/POE1 POB/..." - nested structure
                    let prefix = &name[..pos];
                    let prefix = prefix.trim_end_matches('/');
                    tracing::warn!(
                        phase = "extract",
                        prefix = %prefix,
                        example_file = %name,
                        "Detected nested directory structure in ZIP"
                    );
                    return Ok(Some(PathBuf::from(prefix)));
                }
            }
        }
    }

    Err(PobError::ExtractFailed(
        "ZIP does not contain required folders (POE1 POB, POE2 POB, Data)".into(),
    ))
}

// ============================================================================
// Install Context & Workflow
// ============================================================================

/// Tracks which stages of installation have been completed for rollback purposes
#[derive(Debug, Default)]
struct InstallContext {
    temp_zip_path: Option<PathBuf>,
    extract_dir: Option<PathBuf>,
    install_path: PathBuf,
    backed_up: bool,
    swapped: bool,
}

impl PobManager {
    /// Main installation workflow - transactional update with rollback support.
    ///
    /// Stages: download → extract → backup → swap → restore → save version
    pub async fn install(
        &self,
        file_info: GoogleDriveFileInfo,
        temp_dir: PathBuf,
        cancel_token: CancellationToken,
        reporter: InstallReporter,
    ) -> Result<(), PobError> {
        tracing::info!("=== INSTALL START ===");

        let install_path = self.install_path();
        let extract_dir = install_path.with_extension("new");
        let mut temp_zip_path = temp_dir.join(&file_info.name).with_extension("part");

        let mut ctx = InstallContext {
            temp_zip_path: None,
            extract_dir: Some(extract_dir.clone()),
            install_path: install_path.clone(),
            backed_up: false,
            swapped: false,
        };

        tracing::info!(phase = "init", path = %install_path.display(), "Install path determined");

        // 1. Download
        let download_result = self
            .download_with_progress(
                &file_info.id,
                &temp_zip_path,
                cancel_token.clone(),
                &reporter,
            )
            .await;

        if let Err(e) = download_result {
            tracing::error!(
                phase = "download",
                error = %e,
                "Failed to download POB file from Google Drive"
            );
            tokio::fs::remove_file(&temp_zip_path).await.ok();
            return Err(e);
        }

        // Rename .part to .zip
        let zip_path = temp_zip_path.with_extension("zip");
        tokio::fs::rename(&temp_zip_path, &zip_path).await?;
        temp_zip_path = zip_path;
        ctx.temp_zip_path = Some(temp_zip_path.clone());

        // 2. Extract
        tracing::info!(
            phase = "extract",
            from = %temp_zip_path.display(),
            to = %extract_dir.display(),
            "Extracting to .new directory"
        );

        let extract_result = self
            .extract_with_progress(
                &temp_zip_path,
                &extract_dir,
                cancel_token.clone(),
                reporter.clone(),
            )
            .await;

        if let Err(e) = extract_result {
            tracing::info!(operation = "cleanup", path = %temp_zip_path.display(), "Cleaning up temp ZIP file after extract failure");
            tokio::fs::remove_file(&temp_zip_path).await.ok();
            return Err(e);
        }

        tracing::info!(phase = "extract", path = %extract_dir.display(), "Extract completed");

        // 3. Backup existing user data
        tracing::info!(phase = "backup", "Starting backup phase");
        self.backup(&reporter).await?;
        ctx.backed_up = true;
        tracing::info!(phase = "backup", "Backup completed");

        // 4-6: Atomic operations with rollback on failure
        let result = self
            .finish_install(&extract_dir, &install_path, &file_info, &reporter)
            .await;

        if let Err(e) = result {
            tracing::error!(phase = "rollback", error = %e, "Installation failed, attempting rollback");
            self.rollback(&ctx).await;
            return Err(e);
        }

        ctx.swapped = true;

        // Success: cleanup
        self.cleanup_success(&ctx, &temp_zip_path).await;

        tracing::info!("=== INSTALL SUCCESS ===");
        Ok(())
    }

    /// Finish installation: swap → restore → save version
    async fn finish_install(
        &self,
        extract_dir: &Path,
        install_path: &Path,
        file_info: &GoogleDriveFileInfo,
        reporter: &InstallReporter,
    ) -> Result<(), PobError> {
        // 4. Swap (rename .new to install_path)
        tracing::info!(
            phase = "rename",
            from = %extract_dir.display(),
            to = %install_path.display(),
            "Starting rename phase"
        );
        self.rename(extract_dir, install_path, reporter).await?;
        tracing::info!(phase = "rename", "Rename completed");

        // 5. Restore user data
        tracing::info!(phase = "restore", "Starting restore phase");
        self.restore(reporter).await?;
        tracing::info!(phase = "restore", "Restore completed");

        // 6. Save version info
        tracing::info!(phase = "finalize", "Saving version info");
        let version = PobVersion::try_from(file_info)?;
        self.save_version_info(&version).await?;
        tracing::info!(phase = "finalize", "Version info saved");

        Ok(())
    }

    /// Rollback on failure - restore from .old if available
    async fn rollback(&self, ctx: &InstallContext) {
        let old_path = ctx.install_path.with_extension("old");

        if old_path.exists() {
            tracing::info!(phase = "rollback", path = %old_path.display(), "Restoring from .old");

            // Remove partial installation
            if ctx.install_path.exists() {
                tracing::warn!(phase = "rollback", "Removing partial installation");
                tokio::fs::remove_dir_all(&ctx.install_path).await.ok();
            }

            // Restore from .old
            if let Err(e) = tokio::fs::rename(&old_path, &ctx.install_path).await {
                tracing::error!(
                    phase = "rollback",
                    error = %e,
                    old = %old_path.display(),
                    target = %ctx.install_path.display(),
                    "CRITICAL: Failed to rollback from .old, manual intervention required"
                );
            } else {
                tracing::info!(phase = "rollback", "Successfully restored from .old");
            }
        } else {
            tracing::warn!(phase = "rollback", "No .old directory to rollback from");
        }

        // Cleanup: remove .new if exists
        if let Some(ref extract_dir) = ctx.extract_dir
            && extract_dir.exists()
        {
            tracing::info!(operation = "cleanup", path = %extract_dir.display(), "Cleaning up .new directory");
            tokio::fs::remove_dir_all(extract_dir).await.ok();
        }
    }

    /// Cleanup after successful installation
    async fn cleanup_success(&self, ctx: &InstallContext, temp_zip_path: &Path) {
        tracing::info!(
            operation = "cleanup",
            "Installation successful, cleaning up temporary directories"
        );

        // Remove temp ZIP
        if temp_zip_path.exists() {
            tracing::debug!(operation = "cleanup", path = %temp_zip_path.display(), "Removing temp ZIP");
            tokio::fs::remove_file(temp_zip_path).await.ok();
        }

        // Remove .old
        let old_path = ctx.install_path.with_extension("old");
        if old_path.exists() {
            tracing::debug!(operation = "cleanup", path = %old_path.display(), "Removing .old");
            tokio::fs::remove_dir_all(&old_path).await.ok();
        }

        // Remove .new (should not exist at this point, but just in case)
        if let Some(ref extract_dir) = ctx.extract_dir
            && extract_dir.exists()
        {
            tracing::debug!(operation = "cleanup", path = %extract_dir.display(), "Removing .new");
            tokio::fs::remove_dir_all(extract_dir).await.ok();
        }
    }

    /// Uninstall PoB - removes the installation directory
    pub async fn uninstall(&self, reporter: &InstallReporter) -> Result<(), PobError> {
        let path = self.install_path();

        if path.exists() {
            tracing::info!(phase = "uninstall", path = %path.display(), "Starting uninstall");
            reporter.report(
                InstallPhase::Uninstalling,
                InstallStatus::Started { total_size: None },
            );

            tokio::fs::remove_dir_all(&path).await?;

            reporter.report(InstallPhase::Uninstalling, InstallStatus::Completed);
            tracing::info!(phase = "uninstall", "Uninstall completed");
        } else {
            tracing::debug!(phase = "uninstall", "No installation found, skipping");
        }

        Ok(())
    }
}
