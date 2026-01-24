use std::{process::Stdio, sync::atomic::Ordering};

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::ErrorKind,
    pob::{
        error::PobError,
        google_drive::GoogleDriveFileInfo,
        manager::{CancelEvent, PobManager},
        progress::{InstallPhase, InstallProgress, InstallStatus},
        version::PobVersion,
    },
};

type Result<T, E = ErrorKind> = std::result::Result<T, E>;

#[tauri::command]
#[specta::specta]
pub async fn fetch_pob(
    refresh: bool,
    manager: State<'_, PobManager>,
) -> Result<GoogleDriveFileInfo> {
    Ok(manager.fetch_latest_file(refresh).await?)
}

#[tauri::command]
#[specta::specta]
pub async fn parse_version(file_name: String) -> Result<String> {
    let version = crate::pob::version::parse_from_name(&file_name)?;
    Ok(version)
}

#[tauri::command]
#[specta::specta]
pub async fn installed_pob_info(manager: State<'_, PobManager>) -> Result<Option<PobVersion>> {
    Ok(manager.installed_version().await?)
}

#[tauri::command]
#[specta::specta]
pub async fn uninstall_pob(manager: State<'_, PobManager>, app: AppHandle) -> Result<()> {
    let path = manager.install_path();
    if path.exists() {
        tracing::info!(phase = "uninstall", path = %path.display(), "Starting uninstall");
        let progress = InstallProgress::new(
            "pob:uninstall",
            InstallPhase::Uninstalling,
            InstallStatus::Started { total_size: None },
        );
        progress.report(&app);

        tokio::fs::remove_dir_all(&path).await?;
        progress.derived(InstallStatus::Completed).report(&app);
        tracing::info!(phase = "uninstall", "Uninstall completed");
    } else {
        tracing::debug!(phase = "uninstall", "No installation found, skipping");
    }

    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn install_pob(
    file_data: Option<GoogleDriveFileInfo>,
    manager: State<'_, PobManager>,
    installing: State<'_, crate::pob::Installing>,
    app: AppHandle,
) -> Result<bool> {
    if installing
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err(ErrorKind::PobError(
            "이미 다른 설치 작업이 진행 중입니다.".into(),
        ));
    }

    let result = install_pob_internal(file_data, manager, app).await;
    installing.store(false, Ordering::Release);
    result
}

async fn install_pob_internal(
    file_data: Option<GoogleDriveFileInfo>,
    manager: State<'_, PobManager>,
    app: AppHandle,
) -> Result<bool> {
    tracing::info!("=== INSTALL START ===");
    let file_info = match file_data {
        Some(data) => data,
        None => manager.fetch_latest_file(false).await?,
    };

    let install_path = manager.install_path();
    tracing::info!(phase = "init", path = %install_path.display(), "Install path determined");

    let temp_dir = app.path().temp_dir()?;
    let mut temp_zip_path = temp_dir.join(&file_info.name).with_extension("part");

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    CancelEvent::once(&app, move |_event| {
        cancel_token_clone.cancel();
    });

    // 1. download zip to <TEMP>/<FILE_NAME>.part
    let result = manager
        .download_with_progress(&file_info.id, &temp_zip_path, cancel_token.clone())
        .await;
    match result {
        Err(e) => {
            tracing::error!(
                phase = "download",
                error = %e,
                "Failed to download POB file from Google Drive. Clean up temporary file."
            );
            tokio::fs::remove_file(&temp_zip_path).await.ok();
            return Err(e.into());
        }
        Ok(_) => {
            // success: rename to <FILE_NAME>.zip
            let new_name = temp_zip_path.with_extension("zip");
            tokio::fs::rename(&temp_zip_path, &new_name).await?;
            temp_zip_path = new_name;
        }
    }

    // 2. extract to <INSTALL_PATH>.new
    let extract_dir = install_path.with_extension("new");
    tracing::info!(
        phase = "extract",
        from = %temp_zip_path.display(),
        to = %extract_dir.display(),
        "Extracting to .new directory"
    );

    let extract_result = manager
        .extract_with_progress(&temp_zip_path, &extract_dir, cancel_token.clone())
        .await;

    // Cleanup temp ZIP on extract failure/cancellation
    if extract_result.is_err() {
        tracing::info!(operation = "cleanup", path = %temp_zip_path.display(), "Cleaning up temp ZIP file after extract failure");
        tokio::fs::remove_file(&temp_zip_path).await.ok();
    }

    extract_result?;
    tracing::info!(phase = "extract", path = %extract_dir.display(), exists = %extract_dir.exists(), "Extract completed");

    // 3. backup existing
    tracing::info!(phase = "backup", "Starting backup phase");
    manager.backup().await?;
    tracing::info!(phase = "backup", "Backup completed");

    let result = async {
        // 4. move new installation
        tracing::info!(
            phase = "rename",
            from = %extract_dir.display(),
            to = %install_path.display(),
            "Starting rename phase"
        );
        manager.rename(&extract_dir, &install_path).await?;
        tracing::info!(phase = "rename", "Rename completed");

        // 5. restore
        tracing::info!(phase = "restore", "Starting restore phase");
        manager.restore().await?;
        tracing::info!(phase = "restore", "Restore completed");

        // 6. save version info
        tracing::info!(phase = "finalize", "Saving version info");
        let version = PobVersion::try_from(&file_info)?;
        manager.save_version_info(&version).await?;
        tracing::info!(phase = "finalize", "Version info saved");
        Ok::<(), PobError>(())
    }
    .await;

    if let Err(e) = result {
        tracing::error!(phase = "rollback", error = %e, "Installation failed, attempting rollback");

        // Rollback: restore from .old if exists
        let old_path = install_path.with_extension("old");
        if old_path.exists() {
            tracing::info!(phase = "rollback", path = %old_path.display(), "Restoring from .old");

            // Remove partial installation
            if install_path.exists() {
                tracing::warn!(phase = "rollback", "Removing partial installation");
                tokio::fs::remove_dir_all(&install_path).await.ok();
            }

            // Restore from .old
            if let Err(rollback_err) = tokio::fs::rename(&old_path, &install_path).await {
                tracing::error!(
                    phase = "rollback",
                    error = %rollback_err,
                    old = %old_path.display(),
                    target = %install_path.display(),
                    "CRITICAL: Failed to rollback from .old, manual intervention required"
                );
            } else {
                tracing::info!(phase = "rollback", "Successfully restored from .old");
            }
        } else {
            tracing::warn!(phase = "rollback", "No .old directory to rollback from");
        }

        // Cleanup: remove .new if exists
        if extract_dir.exists() {
            tracing::info!(operation = "cleanup", path = %extract_dir.display(), "Cleaning up .new directory");
            tokio::fs::remove_dir_all(&extract_dir).await.ok();
        }

        return Err(e.into());
    }

    // Success: cleanup .old and .new
    tracing::info!(operation = "cleanup", "Installation successful, cleaning up temporary directories");
    let old_path = install_path.with_extension("old");
    if old_path.exists() {
        tracing::debug!(operation = "cleanup", path = %old_path.display(), "Removing .old");
        tokio::fs::remove_dir_all(&old_path).await.ok();
    }
    if extract_dir.exists() {
        tracing::debug!(operation = "cleanup", path = %extract_dir.display(), "Removing .new");
        tokio::fs::remove_dir_all(&extract_dir).await.ok();
    }

    tracing::info!("=== INSTALL SUCCESS ===");
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_install_pob(app: AppHandle) {
    // Implement cancellation logic here
    _ = CancelEvent.emit(&app);
}

#[tauri::command]
#[specta::specta]
pub async fn execute_pob(manager: State<'_, PobManager>) -> Result<()> {
    let exe_path = manager.exe_path();
    if !exe_path.exists() {
        return Err(ErrorKind::PobError(format!(
            "POB 실행 파일을 찾을 수 없습니다: {}",
            exe_path.display()
        )));
    }

    tracing::info!(operation = "execute", path = %exe_path.display(), "Launching POB executable");
    tokio::process::Command::new(exe_path)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .spawn()
        .map_err(|e| ErrorKind::PobError(format!("POB 실행에 실패했습니다: {}", e)))?;

    Ok(())
}
