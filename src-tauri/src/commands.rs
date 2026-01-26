use std::{
    process::Stdio,
    sync::{Arc, atomic::Ordering},
};

use tauri::{AppHandle, Manager, State};
use tauri_specta::Event;
use tokio_util::sync::CancellationToken;

use crate::{
    errors::ErrorKind,
    pob::{
        google_drive::GoogleDriveFileInfo,
        manager::{CancelEvent, PobManager},
        progress::{InstallReporter, TauriProgressSink},
        version::PobVersion,
    },
    util::generate_task_id,
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
    let task_id = generate_task_id("pob");
    let reporter = InstallReporter::new(task_id, Arc::new(TauriProgressSink::new(app)));

    manager.uninstall(&reporter).await?;
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
    // Concurrency guard
    if installing
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return Err(ErrorKind::Conflict(
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
    // Get file info
    let file_info = match file_data {
        Some(data) => data,
        None => manager.fetch_latest_file(false).await?,
    };

    // Create reporter with unique task_id
    let task_id = generate_task_id("pob");
    let reporter = InstallReporter::new(task_id, Arc::new(TauriProgressSink::new(app.clone())));

    // Get temp directory
    let temp_dir = app.path().temp_dir()?;

    // Setup cancellation
    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();
    CancelEvent::once(&app, move |_event| {
        cancel_token_clone.cancel();
    });

    // Delegate to manager
    manager
        .install(file_info, temp_dir, cancel_token, reporter)
        .await?;

    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_install_pob(app: AppHandle) {
    _ = CancelEvent.emit(&app);
}

#[tauri::command]
#[specta::specta]
pub async fn execute_pob(manager: State<'_, PobManager>) -> Result<()> {
    let exe_path = manager.exe_path();
    if !exe_path.exists() {
        return Err(ErrorKind::NotFound(format!(
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
        .map_err(|e| ErrorKind::Io(format!("POB 실행에 실패했습니다: {}", e)))?;

    Ok(())
}
