# Tauri Window System Plugin

This repository provides a Tauri 2 plugin for multi-window management.

The codebase is split into three layers:

- Rust plugin: window creation, registry state, persistence, and event routing
- TypeScript API: thin `invoke` wrappers, registry-change listeners, broker helpers, and error parsing
- Solid UI wrapper: lightweight titlebar/frame components

## What It Solves

- A consistent `openWindow()` flow for creating windows
- Window registry management and parent-child teardown
- Persisted window size and position
- Reactive registry snapshots through `window-system:registry-changed`
- Typed brokered window messages through `window-system:message`
- Startup recovery for tracked windows through `restoreWindows()`
- Restore results include `restored`, `alreadyAlive`, and `skipped`
- Event routing between windows
- Shared frontend window-system controller for refresh / error / operation state
- Shell-first frontend boot with stale-first registry snapshot caching in the Solid host example
- Clear separation between UI and logic

## Repository Layout

- `crates/tauri-plugin-window-system`
  - Rust plugin crate
- `packages/tauri-plugin-window-system-api`
  - TypeScript API for frontend use
- `packages/tauri-window-ui`
  - Solid wrapper for window frame/titlebar UI
- `examples/solid-host`
  - Minimal host app for local validation

## Rust Plugin

### Initialization

```rust
fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_window_system::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
}
```

### Responsibilities

- `open_window`
- `close_window`
- `list_windows`
- `emit_to_window`
- `WindowRegistry`
- `WindowStateStore`

### Persistence

Window geometry is stored at `app_data_dir/window-system/windows.json`.
Closing a window does not delete the saved geometry, so reopened windows restore their last known size and position.
Windows are created hidden, restored, and then shown to avoid startup flash on desktop platforms.
Repeated move/resize updates are coalesced, and the store keeps a Drop fallback for the final flush.
`open_window` trims blank `label`, `parent`, `title`, and `url` inputs before building a window, and restore paths reuse the same normalization rules.
Open/restore timing logs are available in debug builds and can be enabled in release builds with the `window-timings` Cargo feature.

### Parent-Child Windows

- `parent` in `open_window` must refer to an existing window label
- the plugin also binds the child to the native parent/owner relationship through Tauri
- `close_window` closes child windows first
- native close actions (`CloseRequested`) are intercepted with `prevent_close()` on the first request and then routed through the same teardown path as `close_window`
- self-parenting is rejected

## TypeScript API

### Install

```bash
pnpm add tauri-plugin-window-system-api
```

### API

```ts
import {
  openWindow,
  closeWindow,
  listWindows,
  restoreWindows,
  emitToWindow,
  sendWindowMessage,
  broadcastWindowMessage,
  listenWindowMessages,
  listenRegistryChanges,
  parseWindowSystemError,
  createWindowBus,
} from "tauri-plugin-window-system-api";
```

#### `openWindow(request)`

```ts
await openWindow({
  label: "child",
  parent: "main",
  url: "index.html",
  title: "Child Window",
  geometry: { x: 100, y: 100, width: 800, height: 600 },
});
```

- `label`: required
- `url`: defaults to `index.html`
- `parent`: optional
- `title`: optional
- `geometry`: optional
- blank string inputs are trimmed before validation or use

#### `closeWindow(label)`

Closes the window for the given label.

#### `listWindows()`

Returns the current registry snapshot, sorted by label.

#### `restoreWindows()`

Reopens tracked windows from the persisted snapshot and rehydrates the runtime registry.
Restoration is parent-first, skips already-live windows, and preserves the same geometry fallback behavior as `openWindow()`.

#### `emitToWindow(label, event, payload)`

Sends an event to a target window. `payload` must be JSON-serializable.

#### `sendWindowMessage(request)`

Sends a typed direct envelope through the plugin broker.

#### `broadcastWindowMessage(request)`

Sends a typed broadcast envelope through the plugin broker.

#### `listenWindowMessages(handler)`

Subscribes to brokered `window-system:message` envelopes.

#### `listenRegistryChanges(handler)`

Subscribes to `window-system:registry-changed` snapshots.
The event payload includes `kind`, `label`, and the current `windows` list.

#### `parseWindowSystemError(value)`

Normalizes plugin errors into `{ kind, message, raw }` for UI state and logging.

#### `createWindowBus()`

Creates a small helper for listen/send/request/reply flows.

#### Typed Topics

The broker helper is generic over a topic map.

```ts
import {
  createWindowBus,
  type WindowMessageTopicDefinition,
} from "tauri-plugin-window-system-api";

type WindowTopics = {
  "window-system:ping": WindowMessageTopicDefinition<
    { at: string; source: string },
    { label: string; receivedAt: string }
  >;
  "window-system:status": WindowMessageTopicDefinition<{
    source: string;
    windows: number;
    orphans: number;
  }>;
};

const bus = createWindowBus<WindowTopics>();
```

- event-only topics omit the response payload
- request/response topics declare both payloads
- `bus.request()` is only for topics with a response payload
- `bus.reply()` should reuse the request envelope topic

## UI Wrapper

`packages/tauri-window-ui` provides the visual shell only.

### `WindowFrame`

```tsx
<WindowFrame
  title={<span>Main</span>}
  meta={<span>ready</span>}
  actions={<button>Refresh</button>}
  footer={<span>footer text</span>}
>
  <App />
</WindowFrame>
```

### Slots

- `title`: primary title area
- `meta`: supporting metadata
- `actions`: right-side actions
- `footer`: lower supporting area

## Capabilities and Permissions

Tauri 2 requires plugin commands to be enabled through capabilities.
`window-system` does not take a runtime config object, so `plugins.window-system` should be omitted from `tauri.conf.json`.

`examples/solid-host/src-tauri/capabilities/main.json` and `examples/solid-host/src-tauri/capabilities/child.json` split the example by window role:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "identifier": "main",
  "description": "Capability for the Solid host main window",
  "windows": ["main"],
  "permissions": [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "window-system:allow-open-window",
    "window-system:allow-close-window",
    "window-system:allow-list-windows",
    "window-system:allow-restore-windows",
    "window-system:allow-send-window-message",
    "window-system:allow-broadcast-window-message"
  ]
}
```

Child windows use a narrower capability:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "identifier": "child",
  "description": "Capability for Solid host child windows",
  "windows": ["child-*"],
  "permissions": [
    "core:event:allow-listen",
    "core:event:allow-unlisten",
    "window-system:allow-send-window-message"
  ]
}
```

This keeps child windows limited to event subscription and direct replies.

## Example Host

`examples/solid-host` is the minimal validation app for:

- titlebar / actions / footer layout
- window state transitions and inline error handling via `parseWindowSystemError()`
- `openWindow` / `closeWindow` / `restoreWindows` / `sendWindowMessage` / `broadcastWindowMessage`
- registry listing with parent, child count, geometry summary, and selectable child rows for targeting direct children via `children_of()`
- Windows WebView2 startup and close behavior
- stale-first registry caching in the Solid host example, while the Rust plugin remains the registry source of truth

## Development

### Rust

```bash
cargo test -p tauri-plugin-window-system
cargo check -p solid-host
```

### TypeScript / Frontend

```bash
pnpm install
pnpm -r typecheck
pnpm -r build
```

## Notes

- `WindowGeometry` currently uses outer position / outer size for persistence
- Saved geometry survives normal close/reopen cycles
- Identical geometry updates are ignored to reduce lock and flush pressure
- The UI example treats the plugin registry as the source of truth for its summary cards
- The Solid host example uses shell-first rendering plus stale-first registry caching, but the Rust plugin still owns the registry source of truth
- `children_of()` is the display/diagnostics path; `child_labels_of()` stays on the close-oriented path
- `emitToWindow` payloads must be serializable values
- `sendWindowMessage` targets must resolve to live labels or the broker returns `window-not-found`
- `broadcastWindowMessage` fan-outs through the broker event plane
- `listenRegistryChanges()` mirrors the Rust registry-change event contract and delivers `opened`, `closed`, and `geometry-changed` snapshots
- `restoreWindows()` reopens tracked windows on startup and rehydrates the registry snapshot
- `restoreWindows()` also prunes tracked children whose parent no longer exists, while still reporting them in the skipped list
- identical tracked-window descriptors are short-circuited to reduce redundant persistence work
- frontend snapshot refreshes are coalesced inside `createLiveSnapshot()`
- The UI wrapper does not depend on the plugin core
