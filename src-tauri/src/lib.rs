mod commands;
pub mod errors;
pub mod pob;
pub mod util;
use std::time::Duration;

use tauri::Manager;
use tauri_plugin_tracing::{
    Builder as TracingBuilder, LevelFilter, MaxFileSize, Rotation, RotationStrategy,
};
use tauri_specta::{collect_commands, collect_events};

use crate::pob::{
    Installing,
    google_drive::GoogleDriveClient,
    manager::{CancelEvent, PobManager},
    progress::InstallProgress,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let client = reqwest::Client::builder()
        .tls_backend_rustls()
        .pool_max_idle_per_host(10)
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_keepalive(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(300))
        .build()
        .expect("Failed to build reqwest client");

    let specta_builder = specta_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            TracingBuilder::default()
                .with_max_level(LevelFilter::DEBUG) // Set max log level to DEBUG
                .with_target("exile_rs_lib", LevelFilter::TRACE)
                .with_target("h2", LevelFilter::WARN)
                .with_target("hyper", LevelFilter::WARN)
                .with_colors()
                .with_file_logging() // Enable file logging to platform log directory
                .with_rotation(Rotation::Daily) // Rotate log files daily
                .with_rotation_strategy(RotationStrategy::KeepSome(7)) // Keep last 7 log files
                .with_max_file_size(MaxFileSize::mb(10)) // Rotate when file reaches 10 MB
                .with_file(true) // Show source file in logs
                .with_line_number(true) // Show line number in logs
                .with_target_display(true) // Show module target in logs
                .with_level(true) // Show log level in logs
                .with_default_subscriber() // Set as global tracing subscriber
                .build(),
        )
        .invoke_handler(specta_builder.invoke_handler())
        .manage(Installing::default())
        .setup(move |app| {
            specta_builder.mount_events(app.handle());

            let client = GoogleDriveClient::new(client);

            let data_dir = app
                .path()
                .app_local_data_dir()
                .expect("Failed to get app local data dir");

            let pob_manager = PobManager::new(client, data_dir);
            app.manage(pob_manager);

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn specta_builder() -> tauri_specta::Builder {
    let builder = tauri_specta::Builder::new()
        .commands(collect_commands![
            commands::fetch_pob,
            commands::installed_pob_info,
            commands::install_pob,
            commands::cancel_install_pob,
            commands::parse_version,
            commands::uninstall_pob,
            commands::execute_pob,
            commands::get_install_path,
        ])
        .events(collect_events![InstallProgress, CancelEvent,]);

    #[cfg(debug_assertions)]
    {
        builder
            .export(
                specta_typescript::Typescript::default(),
                "../src/lib/bindings.ts",
            )
            .expect("failed to export specta bindings");
    }

    builder
}
