use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub fn emit_to_window<R: Runtime>(
    handle: &AppHandle<R>,
    label: &str,
    event: &str,
    payload: Value,
) -> Result<(), String> {
    let window = handle
        .get_webview_window(label)
        .ok_or_else(|| "window not found".to_string())?;
    window
        .emit(event, payload)
        .map_err(|err: tauri::Error| err.to_string())
}
