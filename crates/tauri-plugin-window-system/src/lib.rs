mod commands;
mod events;
mod lifecycle;
mod message;
mod registry;

use crate::lifecycle::capture_window_geometry;
use crate::registry::{window_system_error, WindowSystemErrorKind};
use std::cmp::Ordering;
use tauri::plugin::{Builder, TauriPlugin};
use tauri::{Manager, Runtime, WebviewWindow};

pub use commands::{RestoreSkipReason, RestoreSkippedWindow, RestoreWindowsResult};
pub use message::{
    BroadcastWindowMessageRequest, SendWindowMessageRequest, WindowMessageDispatchResult,
    WindowMessageEnvelope, WindowMessageKind, WindowMessageScope,
};
pub use registry::{WindowDescriptor, WindowGeometry, WindowRegistry, WindowStateStore};

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("window-system")
        .setup(|app, _api| {
            let app_data_dir = app.path().app_data_dir().map_err(|err| err.to_string())?;
            let state_path = app_data_dir.join("window-system/windows.json");
            let registry = WindowRegistry::new(WindowStateStore::new(state_path));

            // The plugin keeps registry and persisted geometry in managed state so commands and
            // window-event hooks share a single source of truth.
            // Seed windows that Tauri already created from config so parent lookups can resolve
            // immediately on startup.
            sync_live_windows(app, &registry)?;
            app.manage(registry);
            Ok(())
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
            commands::emit_to_window,
            commands::restore_windows,
            commands::send_window_message,
            commands::broadcast_window_message
        ])
        .build()
}

pub(crate) fn sync_live_windows<R: Runtime>(
    app: &tauri::AppHandle<R>,
    registry: &WindowRegistry,
) -> Result<(), String> {
    let mut windows: Vec<_> = app.webview_windows().into_values().collect();
    windows.sort_by(compare_window_labels);

    for window in windows {
        sync_live_webview_window(registry, &window)?;
    }

    Ok(())
}

pub(crate) fn sync_live_window<R: Runtime>(
    app: &tauri::AppHandle<R>,
    registry: &WindowRegistry,
    label: &str,
) -> Result<(), String> {
    if registry.get(label)?.is_some() {
        // The caller is already registered, so avoid re-capturing geometry and rewriting
        // the same descriptor on every child-open path.
        return Ok(());
    }

    let window = app.get_webview_window(label).ok_or_else(|| {
        window_system_error(
            WindowSystemErrorKind::WindowNotFound,
            format!("window not found: {label}"),
        )
    })?;

    sync_live_webview_window(registry, &window)
}

fn sync_live_webview_window<R: Runtime>(
    registry: &WindowRegistry,
    window: &WebviewWindow<R>,
) -> Result<(), String> {
    registry.upsert_live_window(live_window_descriptor(window)?)?;
    Ok(())
}

fn live_window_descriptor<R: Runtime>(
    window: &WebviewWindow<R>,
) -> Result<WindowDescriptor, String> {
    Ok(WindowDescriptor {
        label: window.label().to_string(),
        url: window.url().map_err(|err| err.to_string())?.to_string(),
        parent: None,
        title: window.title().ok(),
        geometry: capture_window_geometry(window).ok(),
    })
}

fn compare_window_labels<R: Runtime>(
    left: &WebviewWindow<R>,
    right: &WebviewWindow<R>,
) -> Ordering {
    left.label().cmp(right.label())
}
