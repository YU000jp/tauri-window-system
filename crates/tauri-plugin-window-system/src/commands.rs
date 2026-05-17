use crate::events::{emit_registry_changed, RegistryChangeKind};
use crate::lifecycle::{
    apply_window_geometry, attach_native_parent, attach_window_event_handlers,
    close_window_tree as close_window_tree_impl,
};
use crate::message::{
    broadcast_window_message as broadcast_window_message_impl,
    send_window_message as send_window_message_impl, BroadcastWindowMessageRequest,
    SendWindowMessageRequest, WindowMessageDispatchResult,
};
use crate::registry::{WindowDescriptor, WindowGeometry, WindowRegistry};
use crate::sync_live_window;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Instant;
use tauri::{
    command, AppHandle, Manager, Runtime, State, WebviewUrl, WebviewWindowBuilder, Window,
};

#[derive(Debug, Deserialize)]
pub struct OpenWindowRequest {
    pub label: String,
    pub url: Option<String>,
    pub parent: Option<String>,
    pub title: Option<String>,
    pub geometry: Option<WindowGeometry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreWindowsResult {
    pub restored: Vec<WindowDescriptor>,
    pub already_alive: Vec<String>,
    pub skipped: Vec<RestoreSkippedWindow>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RestoreSkippedWindow {
    pub label: String,
    pub parent: Option<String>,
    pub reason: RestoreSkipReason,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreSkipReason {
    MissingParent,
}

#[derive(Debug)]
struct NormalizedOpenWindowRequest {
    label: String,
    url: String,
    parent: Option<String>,
    title: Option<String>,
    geometry: Option<WindowGeometry>,
}

#[cfg(any(debug_assertions, feature = "window-timings"))]
struct OperationTiming {
    operation: &'static str,
    subject: Option<String>,
    started_at: Instant,
    last_mark: Instant,
}

#[cfg(not(any(debug_assertions, feature = "window-timings")))]
struct OperationTiming;

impl OperationTiming {
    #[cfg(any(debug_assertions, feature = "window-timings"))]
    fn new(operation: &'static str, subject: Option<&str>) -> Self {
        let now = Instant::now();
        Self {
            operation,
            subject: subject.map(str::to_string),
            started_at: now,
            last_mark: now,
        }
    }

    #[cfg(not(any(debug_assertions, feature = "window-timings")))]
    fn new(_operation: &'static str, _subject: Option<&str>) -> Self {
        Self
    }

    #[cfg(any(debug_assertions, feature = "window-timings"))]
    fn mark(&mut self, stage: &str) {
        let now = Instant::now();
        let stage_elapsed = now.duration_since(self.last_mark);
        let total_elapsed = now.duration_since(self.started_at);
        log_operation_timing(
            self.operation,
            self.subject.as_deref(),
            stage,
            stage_elapsed.as_millis(),
            total_elapsed.as_millis(),
        );
        self.last_mark = now;
    }

    #[cfg(not(any(debug_assertions, feature = "window-timings")))]
    fn mark(&mut self, _stage: &str) {}

    #[cfg(any(debug_assertions, feature = "window-timings"))]
    fn finish(self, summary: Option<&str>) {
        let total_elapsed = Instant::now().duration_since(self.started_at);
        log_operation_completion(
            self.operation,
            self.subject.as_deref(),
            total_elapsed.as_millis(),
            summary,
        );
    }

    #[cfg(not(any(debug_assertions, feature = "window-timings")))]
    fn finish(self, _summary: Option<&str>) {}
}

#[command]
pub async fn open_window<R: Runtime>(
    handle: AppHandle<R>,
    window: Window<R>,
    registry: State<'_, WindowRegistry>,
    request: OpenWindowRequest,
) -> Result<WindowDescriptor, String> {
    let current_label = window.label();
    sync_live_window(&handle, &*registry, &current_label)?;
    open_window_impl(&handle, &*registry, request).await
}

#[command]
pub async fn restore_windows<R: Runtime>(
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
) -> Result<RestoreWindowsResult, String> {
    let mut timing = OperationTiming::new("restore_windows", None);

    let live_labels: HashSet<String> = registry
        .list()?
        .into_iter()
        .map(|descriptor| descriptor.label)
        .collect();

    let plan = plan_restore_windows(registry.restore_tracked_windows(), live_labels);
    timing.mark("planned");
    let mut restored = Vec::new();

    for descriptor in &plan.restore_order {
        let request = OpenWindowRequest {
            label: descriptor.label.clone(),
            url: Some(descriptor.url.clone()),
            parent: descriptor.parent.clone(),
            title: descriptor.title.clone(),
            geometry: descriptor.geometry.clone(),
        };
        open_window_impl(&handle, &*registry, request).await?;
        restored.push(descriptor.clone());
    }
    timing.mark("restored");
    cleanup_stale_restore_windows(&*registry, &plan)?;
    timing.mark("cleaned-startup-stale");

    let result = RestoreWindowsResult {
        restored,
        already_alive: plan.already_alive,
        skipped: plan.skipped,
    };
    timing.finish(Some(&format!(
        "restored={} already_alive={} skipped={}",
        result.restored.len(),
        result.already_alive.len(),
        result.skipped.len()
    )));

    Ok(result)
}

fn cleanup_stale_restore_windows(
    registry: &WindowRegistry,
    plan: &RestorePlan,
) -> Result<(), String> {
    let mut stale_labels = HashSet::new();

    for window in &plan.skipped {
        stale_labels.insert(window.label.clone());
    }

    // This cleanup only runs during startup restore, so any child still pointing at a
    // missing parent is stale persisted data rather than a live relationship we should keep.
    for window in &plan.already_alive_descriptors {
        if let Some(parent) = window.parent.as_deref() {
            if !plan.live_labels.contains(parent) {
                stale_labels.insert(window.label.clone());
            }
        }
    }

    if stale_labels.is_empty() {
        return Ok(());
    }

    for label in stale_labels {
        match registry.purge_persisted_window(&label) {
            Ok(true) => {}
            Ok(false) => {}
            Err(err) => {
                eprintln!(
                    "window-system: failed to purge stale window label={} error={err}",
                    label
                );
                return Err(err);
            }
        }
    }

    registry.flush()
}

struct RestorePlan {
    restore_order: Vec<WindowDescriptor>,
    already_alive: Vec<String>,
    already_alive_descriptors: Vec<WindowDescriptor>,
    skipped: Vec<RestoreSkippedWindow>,
    live_labels: HashSet<String>,
}

fn plan_restore_windows(
    descriptors: Vec<WindowDescriptor>,
    mut live_labels: HashSet<String>,
) -> RestorePlan {
    let mut pending = descriptors;
    let mut restore_order = Vec::new();
    let mut already_alive = Vec::new();
    let mut already_alive_descriptors = Vec::new();
    let mut skipped = Vec::new();

    while !pending.is_empty() {
        let mut next_round = Vec::new();
        let mut progress = false;

        for descriptor in pending {
            if live_labels.contains(&descriptor.label) {
                already_alive.push(descriptor.label.clone());
                already_alive_descriptors.push(descriptor);
                continue;
            }

            if let Some(parent) = descriptor.parent.as_deref() {
                if !live_labels.contains(parent) {
                    next_round.push(descriptor);
                    continue;
                }
            }

            live_labels.insert(descriptor.label.clone());
            restore_order.push(descriptor);
            progress = true;
        }

        if !progress {
            skipped.extend(
                next_round
                    .into_iter()
                    .map(|descriptor| RestoreSkippedWindow {
                        label: descriptor.label,
                        parent: descriptor.parent,
                        reason: RestoreSkipReason::MissingParent,
                    }),
            );
            break;
        }

        pending = next_round;
    }

    RestorePlan {
        restore_order,
        already_alive,
        already_alive_descriptors,
        skipped,
        live_labels,
    }
}

async fn open_window_impl<R: Runtime>(
    handle: &AppHandle<R>,
    registry: &WindowRegistry,
    request: OpenWindowRequest,
) -> Result<WindowDescriptor, String> {
    let request = normalize_open_window_request(registry, request)?;
    let mut timing = OperationTiming::new("open_window", Some(&request.label));

    let reservation = registry.reserve_window(&request.label, request.parent.as_deref())?;
    timing.mark("reserved");

    let (window, is_reused_window) = build_webview_window(handle, &request)?;
    timing.mark("built");

    let geometry = request.geometry.clone();
    if let Some(geometry) = geometry.as_ref() {
        if let Err(err) = apply_window_geometry(&window, geometry) {
            if !is_reused_window {
                let _ = window.destroy();
            }
            return Err(err);
        }
    }
    timing.mark("geometry-applied");

    let title = request.title.clone().or_else(|| window.title().ok());
    let descriptor = WindowDescriptor {
        label: request.label.clone(),
        url: request.url.clone(),
        parent: request.parent.clone(),
        title,
        geometry,
    };
    if let Err(err) = registry.insert_reserved(descriptor.clone()) {
        if !is_reused_window {
            let _ = window.destroy();
        }
        return Err(err);
    }
    reservation.commit();

    attach_window_event_handlers(handle, registry, &window, &request.label);

    // A reused handle can still be hidden or minimized, so make the reveal path explicit
    // instead of assuming the runtime window is already visible.
    if let Err(err) = reveal_window(&window) {
        let _ = registry.remove(&request.label);
        if !is_reused_window {
            let _ = window.destroy();
        }
        return Err(err);
    }
    timing.mark(if is_reused_window {
        "reused-shown"
    } else {
        "shown"
    });

    emit_registry_changed(
        &handle,
        &registry,
        RegistryChangeKind::Opened,
        &request.label,
    )?;
    timing.mark("emitted");
    timing.finish(None);

    Ok(descriptor)
}

fn normalize_open_window_request(
    registry: &WindowRegistry,
    request: OpenWindowRequest,
) -> Result<NormalizedOpenWindowRequest, String> {
    let label = normalize_required_text(request.label, "label")?;
    let parent = normalize_optional_text(request.parent);
    let title = normalize_optional_text(request.title);
    let url = normalize_window_url(request.url);
    let geometry = request
        .geometry
        .or_else(|| registry.restore_geometry(&label));

    Ok(NormalizedOpenWindowRequest {
        label,
        url,
        parent,
        title,
        geometry,
    })
}

fn normalize_required_text(value: String, field_name: &str) -> Result<String, String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(crate::registry::window_system_error(
            crate::registry::WindowSystemErrorKind::InvalidLabel,
            format!("{field_name} is required"),
        ));
    }

    Ok(normalized.to_string())
}

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_window_url(url: Option<String>) -> String {
    match url {
        Some(url) => {
            let normalized = url.trim();
            if normalized.is_empty() {
                "index.html".to_string()
            } else {
                normalized.to_string()
            }
        }
        None => "index.html".to_string(),
    }
}

fn build_webview_window<R: Runtime>(
    handle: &AppHandle<R>,
    request: &NormalizedOpenWindowRequest,
) -> Result<(tauri::WebviewWindow<R>, bool), String> {
    if let Some(window) = handle.get_webview_window(&request.label) {
        if let Some(title) = request.title.as_deref() {
            window.set_title(title).map_err(|err| err.to_string())?;
        }
        return Ok((window, true));
    }

    let mut builder = WebviewWindowBuilder::new(
        handle,
        &request.label,
        WebviewUrl::App(request.url.clone().into()),
    )
    .visible(false);

    if let Some(title) = request.title.as_deref() {
        builder = builder.title(title);
    }

    let builder = attach_native_parent(handle, builder, request.parent.as_deref())?;
    builder
        .build()
        .map(|window| (window, false))
        .map_err(|err| err.to_string())
}

fn reveal_window<R: Runtime>(window: &tauri::WebviewWindow<R>) -> Result<(), String> {
    if let Ok(true) = window.is_minimized() {
        window.unminimize().map_err(|err| err.to_string())?;
    }

    window.show().map_err(|err| err.to_string())?;

    if let Err(err) = window.set_focus() {
        eprintln!(
            "window-system: failed to focus window label={} error={err}",
            window.label()
        );
    }

    Ok(())
}

#[cfg(any(debug_assertions, feature = "window-timings"))]
fn log_operation_timing(
    operation: &str,
    subject: Option<&str>,
    stage: &str,
    stage_ms: u128,
    total_ms: u128,
) {
    let subject = subject.unwrap_or("-");

    #[cfg(debug_assertions)]
    eprintln!(
        "window-system: operation={} subject={} stage={} stage_ms={} total_ms={}",
        operation, subject, stage, stage_ms, total_ms
    );

    #[cfg(all(not(debug_assertions), feature = "window-timings"))]
    tracing::debug!(
        operation = operation,
        subject = subject,
        stage = stage,
        stage_ms = stage_ms,
        total_ms = total_ms,
        "window-system timing"
    );
}

#[cfg(any(debug_assertions, feature = "window-timings"))]
fn log_operation_completion(
    operation: &str,
    subject: Option<&str>,
    total_ms: u128,
    summary: Option<&str>,
) {
    let subject = subject.unwrap_or("-");

    #[cfg(debug_assertions)]
    match summary {
        Some(summary) => eprintln!(
            "window-system: operation={} subject={} stage=complete total_ms={} {}",
            operation, subject, total_ms, summary
        ),
        None => eprintln!(
            "window-system: operation={} subject={} stage=complete total_ms={}",
            operation, subject, total_ms
        ),
    }

    #[cfg(all(not(debug_assertions), feature = "window-timings"))]
    match summary {
        Some(summary) => tracing::debug!(
            operation = operation,
            subject = subject,
            total_ms = total_ms,
            summary = summary,
            "window-system operation complete"
        ),
        None => tracing::debug!(
            operation = operation,
            subject = subject,
            total_ms = total_ms,
            "window-system operation complete"
        ),
    }
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
    crate::events::emit_to_window(&handle, &label, &event, payload)
}

#[command]
pub fn send_window_message<R: Runtime>(
    window: Window<R>,
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    request: SendWindowMessageRequest,
) -> Result<WindowMessageDispatchResult, String> {
    send_window_message_impl(window, handle, registry, request)
}

#[command]
pub fn broadcast_window_message<R: Runtime>(
    window: Window<R>,
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    request: BroadcastWindowMessageRequest,
) -> Result<WindowMessageDispatchResult, String> {
    broadcast_window_message_impl(window, handle, registry, request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::WindowStateStore;
    use serde_json::json;
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn descriptor(label: &str, parent: Option<&str>) -> WindowDescriptor {
        WindowDescriptor {
            label: label.into(),
            url: format!("{label}.html"),
            parent: parent.map(str::to_string),
            title: None,
            geometry: None,
        }
    }

    fn unique_temp_path(name: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be monotonic for tests")
            .as_nanos();
        std::env::temp_dir().join(format!("tauri-window-system-commands-{name}-{suffix}.json"))
    }

    #[test]
    fn restore_plan_is_parent_first() {
        let plan = plan_restore_windows(
            vec![descriptor("child", Some("main")), descriptor("main", None)],
            HashSet::new(),
        );

        assert_eq!(
            plan.restore_order
                .iter()
                .map(|descriptor| descriptor.label.as_str())
                .collect::<Vec<_>>(),
            vec!["main", "child"]
        );
        assert!(plan.already_alive.is_empty());
        assert!(plan.skipped.is_empty());
    }

    #[test]
    fn restore_plan_marks_existing_windows_as_alive() {
        let mut live = HashSet::new();
        live.insert("main".to_string());

        let plan = plan_restore_windows(
            vec![descriptor("main", None), descriptor("child", Some("main"))],
            live,
        );

        assert_eq!(plan.already_alive, vec!["main".to_string()]);
        assert_eq!(
            plan.restore_order
                .iter()
                .map(|descriptor| descriptor.label.as_str())
                .collect::<Vec<_>>(),
            vec!["child"]
        );
    }

    #[test]
    fn restore_plan_skips_unresolved_children() {
        let plan = plan_restore_windows(vec![descriptor("child", Some("missing"))], HashSet::new());

        assert!(plan.restore_order.is_empty());
        assert!(plan.already_alive.is_empty());
        assert_eq!(plan.skipped.len(), 1);
        assert_eq!(plan.skipped[0].label, "child");
    }

    #[test]
    fn cleanup_skipped_restore_windows_removes_persisted_state() {
        let path = unique_temp_path("cleanup");
        let store = WindowStateStore::new(path.clone());
        let registry = WindowRegistry::new(store.clone());
        let geometry = WindowGeometry {
            x: 10.0,
            y: 20.0,
            width: 30.0,
            height: 40.0,
        };

        store
            .remember("child", geometry.clone())
            .expect("child geometry should persist");
        store
            .remember_window(WindowDescriptor {
                label: "child".into(),
                url: "child.html".into(),
                parent: Some("missing".into()),
                title: Some("Child".into()),
                geometry: Some(geometry),
            })
            .expect("child descriptor should persist");

        let plan = RestorePlan {
            restore_order: Vec::new(),
            already_alive: Vec::new(),
            already_alive_descriptors: Vec::new(),
            skipped: vec![RestoreSkippedWindow {
                label: "child".into(),
                parent: Some("missing".into()),
                reason: RestoreSkipReason::MissingParent,
            }],
            live_labels: HashSet::new(),
        };

        cleanup_stale_restore_windows(&registry, &plan).expect("cleanup should flush");

        assert!(registry
            .restore_tracked_windows()
            .iter()
            .all(|window| window.label != "child"));
        assert_eq!(registry.restore_geometry("child"), None);

        let restored_store = WindowStateStore::new(path);
        assert!(restored_store
            .restore_tracked_windows()
            .iter()
            .all(|window| window.label != "child"));
        assert_eq!(restored_store.restore("child"), None);
    }

    #[test]
    fn cleanup_stale_restore_windows_removes_orphaned_alive_child() {
        let path = unique_temp_path("orphan-alive");
        let store = WindowStateStore::new(path.clone());
        let registry = WindowRegistry::new(store.clone());
        let descriptor = WindowDescriptor {
            label: "child".into(),
            url: "child.html".into(),
            parent: Some("missing".into()),
            title: Some("Child".into()),
            geometry: None,
        };

        store
            .remember_window(descriptor.clone())
            .expect("tracked child should persist");
        store.flush().expect("snapshot should flush");

        let mut live_labels = HashSet::new();
        live_labels.insert("child".to_string());

        let plan = RestorePlan {
            restore_order: Vec::new(),
            already_alive: vec!["child".into()],
            already_alive_descriptors: vec![descriptor],
            skipped: Vec::new(),
            live_labels,
        };

        cleanup_stale_restore_windows(&registry, &plan).expect("cleanup should flush");

        let restored_store = WindowStateStore::new(path);
        assert!(restored_store.restore_tracked_windows().is_empty());
    }

    #[test]
    fn normalize_open_window_request_trims_and_restores_geometry() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("normalize")));
        let geometry = WindowGeometry {
            x: 10.0,
            y: 20.0,
            width: 300.0,
            height: 200.0,
        };

        registry
            .insert(descriptor("main", None))
            .expect("main should insert");
        registry
            .remember_geometry("main", geometry.clone())
            .expect("geometry should persist");

        let normalized = normalize_open_window_request(
            &registry,
            OpenWindowRequest {
                label: " main ".into(),
                url: Some(" child.html ".into()),
                parent: Some(" root ".into()),
                title: Some(" Child Window ".into()),
                geometry: None,
            },
        )
        .expect("request should normalize");

        assert_eq!(normalized.label, "main");
        assert_eq!(normalized.url, "child.html");
        assert_eq!(normalized.parent, Some("root".into()));
        assert_eq!(normalized.title, Some("Child Window".into()));
        assert_eq!(normalized.geometry, Some(geometry));
    }

    #[test]
    fn normalize_open_window_request_rejects_blank_label() {
        let registry = WindowRegistry::new(WindowStateStore::new(unique_temp_path("invalid")));

        let error = normalize_open_window_request(
            &registry,
            OpenWindowRequest {
                label: "   ".into(),
                url: None,
                parent: None,
                title: None,
                geometry: None,
            },
        )
        .expect_err("blank label should be rejected");

        assert!(error.contains("invalid-label"));
        assert!(error.contains("label is required"));
    }

    #[test]
    fn restore_windows_result_serializes_with_stable_field_names() {
        let result = RestoreWindowsResult {
            restored: vec![descriptor("main", None)],
            already_alive: vec!["child".into()],
            skipped: vec![RestoreSkippedWindow {
                label: "orphan".into(),
                parent: Some("missing".into()),
                reason: RestoreSkipReason::MissingParent,
            }],
        };

        let value = serde_json::to_value(result).expect("result should serialize");

        assert_eq!(value["restored"][0]["label"], "main");
        assert_eq!(value["alreadyAlive"], json!(["child"]));
        assert_eq!(value["skipped"][0]["reason"], "missing-parent");
        assert_eq!(value["skipped"][0]["parent"], "missing");
    }
}
