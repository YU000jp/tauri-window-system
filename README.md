# Tauri Window System Plugin

This repository provides a Tauri 2 plugin for multi-window management.

The codebase is split into three layers:

- Rust plugin: window creation, registry state, persistence, and event routing
- TypeScript API: thin `invoke` wrappers
- Solid UI wrapper: `tauri-controls` based titlebar/frame components

## What It Solves

- A consistent `openWindow()` flow for creating windows
- Window registry management and parent-child teardown
- Persisted window size and position
- Event routing between windows
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
  emitToWindow,
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

#### `closeWindow(label)`

Closes the window for the given label.

#### `listWindows()`

Returns the current registry snapshot, sorted by label.

#### `emitToWindow(label, event, payload)`

Sends an event to a target window. `payload` must be JSON-serializable.

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

`examples/solid-host/src-tauri/capabilities/default.json` currently includes:

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "identifier": "default",
  "description": "Capability for the Solid host example",
  "windows": ["main"],
  "permissions": ["core:default", "os:default", "window-system:default"]
}
```

## Example Host

`examples/solid-host` is the minimal validation app for:

- titlebar / actions / footer layout
- window state transitions and inline error handling
- `openWindow` / `closeWindow` / `emitToWindow`
- registry listing with parent, child count, and geometry summary
- Windows WebView2 startup and close behavior

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
- `emitToWindow` payloads must be serializable values
- The UI wrapper does not depend on the plugin core
