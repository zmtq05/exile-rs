use std::{process::Stdio, sync::Arc};

use scopeguard::defer;
use tauri::{AppHandle, Manager, State};
use tokio_util::sync::CancellationToken;

use crate::{
    errors::ErrorKind,
    pob::{
        InstallCancelToken,
        google_drive::GoogleDriveFileInfo,
        manager::PobManager,
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
    // Acquire exclusive lock for uninstall operation
    let _guard = manager
        .try_write_lock()
        .ok_or_else(|| ErrorKind::Conflict("이미 다른 작업이 진행 중입니다.".into()))?;

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
    cancel_state: State<'_, InstallCancelToken>,
    app: AppHandle,
) -> Result<bool> {
    // Acquire exclusive lock for install operation (Issue 5: RwLock)
    let _guard = manager
        .try_write_lock()
        .ok_or_else(|| ErrorKind::Conflict("이미 다른 설치 작업이 진행 중입니다.".into()))?;

    // Issue 1: Store cancellation token in managed state (no event listener)
    let cancel_token = CancellationToken::new();
    cancel_state.set(cancel_token.clone());

    // Issue 1: Ensure token is cleared on all exit paths (via defer)
    defer! {
        cancel_state.take();
    }

    // Get file info
    let file_info = match file_data {
        Some(data) => data,
        None => manager.fetch_latest_file(false).await?,
    };

    // Issue 4: Create isolated per-task temp directory
    let task_id = generate_task_id("pob");
    let base_temp = app.path().temp_dir()?;
    let temp_dir = base_temp.join(&task_id);
    tokio::fs::create_dir_all(&temp_dir)
        .await
        .map_err(|e| ErrorKind::Io(format!("임시 디렉토리 생성 실패: {}", e)))?;

    // Create reporter
    let reporter = InstallReporter::new(&task_id, Arc::new(TauriProgressSink::new(app)));

    // Execute install with guaranteed temp cleanup
    let result = manager
        .install(file_info, temp_dir.clone(), cancel_token, reporter)
        .await;

    // Issue 4: Always cleanup temp subdirectory
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    result?;
    Ok(true)
}

#[tauri::command]
#[specta::specta]
pub async fn cancel_install_pob(cancel_state: State<'_, InstallCancelToken>) -> Result<()> {
    // Issue 1: Directly cancel via managed state (no event needed)
    cancel_state.cancel();
    Ok(())
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

#[tauri::command]
#[specta::specta]
pub async fn get_install_path(manager: State<'_, PobManager>) -> Result<String, ErrorKind> {
    Ok(manager.install_path().to_string_lossy().to_string())
}
