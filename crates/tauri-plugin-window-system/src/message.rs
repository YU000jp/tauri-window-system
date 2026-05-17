use crate::events::emit_to_window as emit_to_window_impl;
use crate::registry::{window_system_error, WindowRegistry, WindowSystemErrorKind};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager, Runtime, State, Window};

pub const WINDOW_MESSAGE_EVENT: &str = "window-system:message";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowMessageScope {
    Direct,
    Broadcast,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WindowMessageKind {
    Event,
    Request,
    Response,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowMessageEnvelope {
    pub from: String,
    pub to: Option<String>,
    pub scope: WindowMessageScope,
    pub kind: WindowMessageKind,
    pub topic: String,
    pub correlation_id: Option<String>,
    pub payload: Value,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendWindowMessageRequest {
    pub to: String,
    pub kind: WindowMessageKind,
    pub topic: String,
    pub correlation_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BroadcastWindowMessageRequest {
    pub kind: WindowMessageKind,
    pub topic: String,
    pub correlation_id: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowMessageDispatchResult {
    pub envelope: WindowMessageEnvelope,
    pub delivered_to: Vec<String>,
}

pub fn current_timestamp_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub fn current_live_window_labels<R: Runtime>(
    handle: &AppHandle<R>,
    registry: &WindowRegistry,
) -> Result<HashSet<String>, String> {
    let mut live_labels: HashSet<String> = handle.webview_windows().into_keys().collect();
    let active_windows = registry
        .list()?
        .into_iter()
        .map(|descriptor| descriptor.label)
        .collect::<HashSet<_>>();
    live_labels.extend(active_windows);
    Ok(live_labels)
}

pub fn build_window_message_envelope(
    from: impl Into<String>,
    to: Option<String>,
    scope: WindowMessageScope,
    kind: WindowMessageKind,
    topic: impl Into<String>,
    correlation_id: Option<String>,
    payload: Value,
) -> WindowMessageEnvelope {
    WindowMessageEnvelope {
        from: from.into(),
        to,
        scope,
        kind,
        topic: topic.into(),
        correlation_id,
        payload,
        timestamp_ms: current_timestamp_ms(),
    }
}

pub fn send_window_message<R: Runtime>(
    window: Window<R>,
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    request: SendWindowMessageRequest,
) -> Result<WindowMessageDispatchResult, String> {
    let sender = window.label().to_string();
    let live_labels = current_live_window_labels(&handle, &*registry)?;
    if !live_labels.contains(&request.to) {
        return Err(window_system_error(
            WindowSystemErrorKind::WindowNotFound,
            format!("window not found: {}", request.to),
        ));
    }

    let envelope = build_window_message_envelope(
        sender,
        Some(request.to.clone()),
        WindowMessageScope::Direct,
        request.kind,
        request.topic,
        request.correlation_id,
        request.payload,
    );

    emit_to_window_impl(
        &handle,
        &request.to,
        WINDOW_MESSAGE_EVENT,
        serde_json::to_value(&envelope).map_err(|err| err.to_string())?,
    )?;

    Ok(WindowMessageDispatchResult {
        envelope,
        delivered_to: vec![request.to],
    })
}

pub fn broadcast_window_message<R: Runtime>(
    window: Window<R>,
    handle: AppHandle<R>,
    registry: State<'_, WindowRegistry>,
    request: BroadcastWindowMessageRequest,
) -> Result<WindowMessageDispatchResult, String> {
    let sender = window.label().to_string();
    let live_labels = current_live_window_labels(&handle, &*registry)?;
    let delivered_to: Vec<String> = live_labels.iter().cloned().collect();
    let envelope = build_window_message_envelope(
        sender,
        None,
        WindowMessageScope::Broadcast,
        request.kind,
        request.topic,
        request.correlation_id,
        request.payload,
    );

    <AppHandle<R> as Emitter<R>>::emit(
        &handle,
        WINDOW_MESSAGE_EVENT,
        serde_json::to_value(&envelope).map_err(|err| err.to_string())?,
    )
    .map_err(|err: tauri::Error| err.to_string())?;

    Ok(WindowMessageDispatchResult {
        envelope,
        delivered_to,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn build_window_message_envelope_uses_consistent_shape() {
        let envelope = build_window_message_envelope(
            "main",
            Some("child".into()),
            WindowMessageScope::Direct,
            WindowMessageKind::Request,
            "window-system:ping",
            Some("corr-1".into()),
            json!({"hello": "world"}),
        );

        assert_eq!(envelope.from, "main");
        assert_eq!(envelope.to.as_deref(), Some("child"));
        assert_eq!(envelope.topic, "window-system:ping");
        assert_eq!(envelope.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(envelope.payload["hello"], "world");
        assert!(envelope.timestamp_ms > 0);
    }

    #[test]
    fn window_message_envelope_serializes_with_camel_case_fields() {
        let envelope = build_window_message_envelope(
            "main",
            None,
            WindowMessageScope::Broadcast,
            WindowMessageKind::Event,
            "window-system:status",
            None,
            json!({"windows": 3}),
        );
        let value = serde_json::to_value(envelope).expect("envelope should serialize");

        assert_eq!(value["scope"], "broadcast");
        assert_eq!(value["kind"], "event");
        assert_eq!(value["correlationId"], serde_json::Value::Null);
        assert!(value["timestampMs"].as_u64().unwrap_or_default() > 0);
    }
}
