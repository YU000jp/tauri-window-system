use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_window_system::init())
        .build(tauri::generate_context!())
        .expect("error while building Tauri application");

    app.run(|app_handle, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            if let Err(err) = app_handle.state::<tauri_plugin_window_system::WindowRegistry>().flush()
            {
                eprintln!("window-system: host exit flush failed: {err}");
            }
        }
    });
}

fn main() {
    run();
}
