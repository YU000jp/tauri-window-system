use crate::events::emit_to_window as emit_to_window_impl;
use crate::lifecycle::{
    attach_native_parent, close_window_tree as close_window_tree_impl, handle_close_requested,
};
use crate::registry::{WindowDescriptor, WindowGeometry, WindowRegistry};
use dpi::{PhysicalPosition, PhysicalSize};
use serde::Deserialize;
use serde_json::Value;
use tauri::{
    command, AppHandle, Position, Runtime, Size, State, WebviewUrl, WebviewWindowBuilder,
    WindowEvent,
};

#[derive(Debug, Deserialize)]
pub struct OpenWindowRequest {
    pub label: String,
    pub url: Option<String>,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub geometry: Option<WindowGeometry>,
}

#[command]
pub async fn open_window<R: Runtime>(
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    request: OpenWindowRequest,
) -> Result<WindowDescriptor, String> {
    let reservation = registry.reserve_window(&request.label, request.parent.as_deref())?;

    let url = request.url.unwrap_or_else(|| "index.html".to_string());
    let builder =
        WebviewWindowBuilder::new(&handle, &request.label, WebviewUrl::App(url.clone().into()))
            .visible(false);
    let builder = attach_native_parent(&handle, builder, request.parent.as_deref())?;
    let window = builder.build().map_err(|err| err.to_string())?;

    if let Some(title) = request.title.as_deref() {
        window.set_title(title).map_err(|err| err.to_string())?;
    }

    let geometry = request
        .geometry
        .or_else(|| registry.restore_geometry(&request.label));
    if let Some(geometry) = geometry.clone() {
        apply_geometry(&window, &geometry)?;
    }

    let descriptor = WindowDescriptor {
        label: request.label.clone(),
        url,
        parent: request.parent.clone(),
        title: request.title.clone(),
        geometry,
    };
    registry.insert_reserved(descriptor.clone())?;
    reservation.commit();

    let watched_label = request.label.clone();
    let registry_for_events = (*registry).clone();
    let handle_for_events = handle.clone();
    let window_for_events = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Moved(_) | WindowEvent::Resized(_) => {
            if let Ok(geometry) = capture_geometry(&window_for_events) {
                let _ = registry_for_events.remember_geometry(&watched_label, geometry);
            }
        }
        WindowEvent::CloseRequested { api, .. } => {
            handle_close_requested(&handle_for_events, &registry_for_events, &watched_label, api);
        }
        _ => {}
    });

    if let Err(err) = window.show().map_err(|err| err.to_string()) {
        let _ = registry.remove(&request.label);
        let _ = window.destroy();
        return Err(err);
    }

    Ok(descriptor)
}

#[command]
pub async fn close_window<R: Runtime>(
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    label: String,
) -> Result<(), String> {
    if let Err(err) = close_window_tree_impl(&handle, &*registry, &label, false).await {
        eprintln!(
            "{}",
            crate::lifecycle::format_teardown_failure("command-close", &label, &err)
        );
        return Err(err);
    }

    Ok(())
}

#[command]
pub fn list_windows(registry: State<'_, WindowRegistry>) -> Result<Vec<WindowDescriptor>, String> {
    registry.list()
}

#[command]
pub fn emit_to_window<R: Runtime>(
    handle: AppHandle<R>,
    label: String,
    event: String,
    payload: Value,
) -> Result<(), String> {
    emit_to_window_impl(&handle, &label, &event, payload)
}

fn capture_geometry<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
) -> Result<WindowGeometry, String> {
    let position = window.outer_position().map_err(|err| err.to_string())?;
    let size = window.outer_size().map_err(|err| err.to_string())?;

    Ok(WindowGeometry {
        x: position.x as f64,
        y: position.y as f64,
        width: size.width as f64,
        height: size.height as f64,
    })
}

fn apply_geometry<R: Runtime>(
    window: &tauri::WebviewWindow<R>,
    geometry: &WindowGeometry,
) -> Result<(), String> {
    window
        .set_position(Position::Physical(PhysicalPosition {
            x: geometry.x as i32,
            y: geometry.y as i32,
        }))
        .map_err(|err| err.to_string())?;
    window
        .set_size(Size::Physical(PhysicalSize {
            width: geometry.width as u32,
            height: geometry.height as u32,
        }))
        .map_err(|err| err.to_string())
}
