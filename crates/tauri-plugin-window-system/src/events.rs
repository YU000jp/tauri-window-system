use crate::registry::{
    window_system_error, WindowDescriptor, WindowRegistry, WindowSystemErrorKind,
};
use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Emitter, Manager, Runtime};

pub const REGISTRY_CHANGED_EVENT: &str = "window-system:registry-changed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryChangeKind {
    Opened,
    Closed,
    GeometryChanged,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegistryChangedEvent {
    pub kind: RegistryChangeKind,
    pub label: String,
    pub windows: Vec<WindowDescriptor>,
}

pub fn emit_to_window<R: Runtime>(
    handle: &AppHandle<R>,
    label: &str,
    event: &str,
    payload: Value,
) -> Result<(), String> {
    let window = handle.get_webview_window(label).ok_or_else(|| {
        window_system_error(
            WindowSystemErrorKind::WindowNotFound,
            format!("window not found: {label}"),
        )
    })?;
    window
        .emit(event, payload)
        .map_err(|err: tauri::Error| err.to_string())
}

pub fn build_registry_changed_event(
    registry: &WindowRegistry,
    kind: RegistryChangeKind,
    label: impl Into<String>,
) -> Result<RegistryChangedEvent, String> {
    Ok(RegistryChangedEvent {
        kind,
        label: label.into(),
        windows: registry.list()?,
    })
}

pub fn emit_registry_changed<R: Runtime>(
    handle: &AppHandle<R>,
    registry: &WindowRegistry,
    kind: RegistryChangeKind,
    label: impl Into<String>,
) -> Result<(), String> {
    let payload = build_registry_changed_event(registry, kind, label)?;
    handle
        .emit(REGISTRY_CHANGED_EVENT, payload)
        .map_err(|err: tauri::Error| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{WindowGeometry, WindowStateStore};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn unique_temp_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be monotonic for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("tauri-window-system-events-{name}-{suffix}.json"))
    }

    #[test]
    fn build_registry_changed_event_returns_sorted_windows() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("payload")));
        registry
            .insert(WindowDescriptor {
                label: "beta".into(),
                url: "beta.html".into(),
                parent: None,
                title: None,
                geometry: Some(WindowGeometry {
                    x: 1.0,
                    y: 2.0,
                    width: 3.0,
                    height: 4.0,
                }),
            })
            .expect("beta should insert");
        registry
            .insert(WindowDescriptor {
                label: "alpha".into(),
                url: "alpha.html".into(),
                parent: None,
                title: None,
                geometry: None,
            })
            .expect("alpha should insert");

        let event = build_registry_changed_event(&registry, RegistryChangeKind::Opened, "alpha")
            .expect("payload should build");

        assert_eq!(event.kind, RegistryChangeKind::Opened);
        assert_eq!(event.label, "alpha");
        assert_eq!(event.windows[0].label, "alpha");
        assert_eq!(event.windows[1].label, "beta");
    }

    #[test]
    fn registry_changed_event_serializes_with_expected_shape() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("serialize")));
        registry
            .insert(WindowDescriptor {
                label: "main".into(),
                url: "index.html".into(),
                parent: None,
                title: Some("Main".into()),
                geometry: None,
            })
            .expect("main should insert");

        let event =
            build_registry_changed_event(&registry, RegistryChangeKind::GeometryChanged, "main")
                .expect("payload should build");
        let value = serde_json::to_value(event).expect("event should serialize");

        assert_eq!(value["kind"], "geometry-changed");
        assert_eq!(value["label"], "main");
        assert_eq!(value["windows"][0]["label"], "main");
    }
}
