mod commands;
mod events;
mod lifecycle;
mod registry;

use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, RunEvent, Runtime};

pub use registry::{WindowDescriptor, WindowGeometry, WindowRegistry, WindowStateStore};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("window-system")
        .setup(|app, _api| {
            let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
            let state_path = app_data_dir.join("window-system/windows.json");
            let registry = WindowRegistry::new(WindowStateStore::new(state_path));

            // The plugin keeps registry and persisted geometry in managed state so commands and
            // window-event hooks share a single source of truth.
            app.manage(registry);
            Ok(())
        })
        .on_event(|app, event| {
            if matches!(event, RunEvent::ExitRequested { .. }) {
                if let Err(err) = app.state::<WindowRegistry>().flush() {
                    eprintln!("window-system: exit flush failed: {err}");
                }
            }
        })
        .on_drop(|app| {
            if let Err(err) = app.state::<WindowRegistry>().flush() {
                eprintln!("window-system: drop flush failed: {err}");
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::open_window,
            commands::close_window,
            commands::list_windows,
            commands::emit_to_window
        ])
        .build()
}
