// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    log::debug!("Calling greet command: {}", name);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg(not(debug_assertions))]
fn init_logger() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
}

#[cfg(debug_assertions)]
fn init_logger() -> tauri_plugin_log::Builder {
    use std::path::PathBuf;

    use tauri_plugin_log::{Target, TargetKind};

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("logs");
    tauri_plugin_log::Builder::new()
        .clear_targets()
        .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
        .level(log::LevelFilter::Debug)
        .target(Target::new(TargetKind::Folder {
            path,
            file_name: None,
        }))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(init_logger().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet])
        .setup(|app| {
            log::info!("Starting app {}", app.package_info().version);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
