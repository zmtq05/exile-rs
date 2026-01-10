use serde::Serialize;
use specta::Type;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_specta::Event;


#[tauri::command]
#[specta::specta]
pub async fn pob_install(app: AppHandle) {
    let file_id = "your_file_id_here";
    let save_path = std::path::Path::new("path/to/save/file");

    let client = app.state::<reqwest::Client>().inner().clone();

    let result = crate::pob::gdrive::download_with_progress(
        client,
        file_id,
        save_path,
        move |downloaded, total| {
            let progress = PobDownloadProgress { downloaded, total };
            app.emit("pob-download-progress", progress).unwrap();
        },
    ).await;
}

#[derive(Debug, Clone, Serialize, Type, Event)]
pub struct PobDownloadProgress {
    pub downloaded: u32,
    pub total: u32,
}

#[tauri::command]
#[specta::specta]
pub async fn pob_remote_version(app: AppHandle) -> anyhow::Result<String> {
    let client = app.state::<reqwest::Client>().inner().clone();
    let proxy_info = crate::pob::gdrive::fetch_remote_pob(client).await?;
    Ok(proxy_info.name)
}