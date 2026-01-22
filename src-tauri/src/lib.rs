use tauri_plugin_tracing::{
    tracing, Builder as TracingBuilder, LevelFilter, MaxFileSize, Rotation, RotationStrategy,
};
use tauri_specta::{collect_commands, collect_events};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    tracing::debug!("greet command called with name: {}", name);
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            TracingBuilder::default()
                .with_max_level(LevelFilter::DEBUG) // Set max log level to DEBUG
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
        .setup(move |app| {
            specta_builder.mount_events(app.handle());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn specta_builder<R: tauri::Runtime>() -> tauri_specta::Builder<R> {
    let builder = tauri_specta::Builder::new()
        .commands(collect_commands![
            // commands
            greet,
        ])
        .events(collect_events![
            // events
        ]);

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
