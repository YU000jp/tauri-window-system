# tauri-plugin-window-system

Tauri 2 plugin for multi-window lifecycle management.

## Publish Scope

- This crate is the plugin core published to crates.io.
- The validation host in `examples/solid-host` stays internal and is not part of the published crate.
- The TypeScript API and Solid wrapper are published separately as npm packages.

## Public Contract

The plugin is the source of truth for window lifecycle, registry state, and message routing.

### Commands

- `plugin:window-system|open_window`
- `plugin:window-system|close_window`
- `plugin:window-system|list_windows`
- `plugin:window-system|restore_windows`
- `plugin:window-system|emit_to_window`
- `plugin:window-system|send_window_message`
- `plugin:window-system|broadcast_window_message`
- `window-system:registry-changed` application event
- `window-system:message` application event

### Rust Responsibilities

- Window creation
- Registry management
- Size and position persistence
- Parent-child close chaining
- Window event routing
- Registry-change notifications for frontend resync
- Typed window-bus dispatch for direct, broadcast, and request/response flows
- Startup recovery for tracked windows
- `child_labels_of` for close-oriented child traversal
- `children_of` for display/diagnostics-oriented descriptor traversal

`child_labels_of` is the close path: it returns only labels so `close_window_tree` can stay lightweight and label-based.
`children_of` is the display path: it expands those labels into descriptors for UI surfaces and diagnostics that need metadata.

## Minimal Usage

```rust
fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_window_system::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
}
```

## `open_window`

```rust
#[derive(Deserialize)]
pub struct OpenWindowRequest {
  pub label: String,
  pub url: Option<String>,
  pub parent: Option<String>,
  pub title: Option<String>,
  pub geometry: Option<WindowGeometry>,
}
```

### Rules

- `label` is required
- `label`, `parent`, and `title` are trimmed before use
- `parent` must refer to an existing label
- `url` defaults to `index.html`
- blank `url` values are normalized to `index.html`
- `geometry` overrides restored values when present
- error strings are prefixed with stable machine-readable codes

`open_window()` returns the created `WindowDescriptor`.

## Restore Behavior

`restore_windows()` returns a `RestoreWindowsResult` with camelCase fields:

- `restored`
- `alreadyAlive`
- `skipped`

Skipped entries use `RestoreSkippedWindow` with `reason: "missing-parent"` when a persisted child cannot be resolved.

Tracked windows are restored parent-first.
Children whose parent is still missing are reported in `skipped` and then pruned from the persisted snapshot during startup cleanup.

## Close Behavior

- `close_window` and native close events share the same teardown logic
- the first `CloseRequested` is prevented so the plugin can finish child-first teardown safely
- child windows are closed before their parent is removed
- saved geometry is preserved across close/reopen cycles
- tracked windows are restored from `app_data_dir/window-system/windows.json` during startup sync
- windows are created hidden and shown after geometry restoration to reduce startup flash
- identical geometry updates are skipped so move/resize storms do not thrash the registry or persistence layer
- identical tracked-window descriptors are ignored so repeated syncs do not rewrite unchanged state
- geometry persistence uses coalesced writes during normal operation
- the plugin keeps a Drop fallback for the final geometry flush
- registry/list/update operations emit `window-system:registry-changed`
- `restore_windows` returns restored, already-alive, and skipped window groups
- `send_window_message` routes a typed envelope to one live window
- `broadcast_window_message` emits a typed envelope to all live windows
- `createWindowBus()` on the frontend centralizes listen/send/request/reply handling
- open/restore timings are emitted in debug builds, or in release builds with the `window-timings` feature enabled

## Typed Topics

The frontend bus is topic-aware through TypeScript generics.

- `WindowMessageTopicDefinition<Request, Response>` defines the payload contract for one topic
- event-only topics omit the response payload
- request/response topics should declare both sides so `bus.request()` and `bus.reply()` stay aligned
- the broker only validates delivery and lifecycle boundaries; topic payload shapes live in the frontend API layer

## Event Shapes

- `RegistryChangedEvent` serializes as `{ kind, label, windows }`
- `RegistryChangeKind` serializes as `opened`, `closed`, or `geometry-changed`
- `WindowMessageEnvelope` serializes with camelCase fields
- `WindowMessageScope` serializes as `direct` or `broadcast`
- `WindowMessageKind` serializes as `event`, `request`, or `response`

## Native Parent Handling

- `parent` is resolved to a live Tauri window before the child is created
- the child is linked to the OS-native parent/owner relation through Tauri's builder API
- if the parent label does not exist or is already closing, window creation is rejected

## Persistence

Saved to `app_data_dir/window-system/windows.json`.
The store is updated from move/resize events, but identical values are ignored so repeated notifications do not trigger extra writes.
The plugin relies on a Drop fallback for the final flush so the latest geometry is durably written before shutdown.

## Error Codes

Stable machine-readable codes are emitted from `WindowSystemErrorKind` and preserved in Rust error strings:

- `invalid-label`
- `window-already-exists`
- `window-cannot-be-its-own-parent`
- `parent-window-not-found`
- `window-not-found`
- `window-reservation-not-found`
- `window-registry-lock-poisoned`
- `window-state-store-lock-poisoned`

## Permissions

`permissions/default.toml` enables:

- `allow-open-window`
- `allow-close-window`
- `allow-list-windows`
- `allow-restore-windows`
- `allow-emit-to-window`

## Tests

- registry insert / list / child filtering
- geometry persist / restore
- restore planning and cleanup
- command / event / message serialization contracts
- `cargo check -p tauri-plugin-window-system`
