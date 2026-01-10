mod pob;

use tauri_specta::{collect_commands, collect_events};

use crate::pob::commands::PobDownloadProgress;

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
        .target(Target::new(TargetKind::Stdout))
}

fn init_reqwest_client() -> reqwest::Client {
    static USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .build()
        .expect("Failed to build reqwest client")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri_specta::Builder::<tauri::Wry>::new()
        .commands(collect_commands![pob::commands::pob_install,])
        .events(collect_events![PobDownloadProgress]);

    #[cfg(debug_assertions)]
    builder
        .export(
            specta_typescript::Typescript::default(),
            "../src/lib/bindings.ts",
        )
        .expect("Failed to export TypeScript bindings");

    tauri::Builder::default()
        .plugin(init_logger().build())
        .plugin(tauri_plugin_opener::init())
        .manage(init_reqwest_client())
        .invoke_handler(builder.invoke_handler())
        .setup(move |app| {
            builder.mount_events(app.handle());
            log::info!("Starting app {}", app.package_info().version);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
