use tauri_specta::{collect_commands, collect_events};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
#[specta::specta]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let specta_builder = specta_builder();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
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
            greet,
        ])
        .events(collect_events![
            // events
        ]);

    #[cfg(debug_assertions)]
    {
        builder
            .export(specta_typescript::Typescript::default(), "../src/lib/bindings.ts")
            .expect("failed to export specta bindings");
    }

    builder
}
