# Tauri Window System Plugin

Tauri 2 plugin for building a multi-window foundation.

<img width="1608" height="1504" alt="image" src="https://github.com/user-attachments/assets/4e906634-c28d-43aa-9e98-9dc2dde23bf4" />

> If you found it useful, please gift it to me via the “[Buy me a pizza](https://www.buymeacoffee.com/yu000japan)”.

## Packages

- `crates/tauri-plugin-window-system`: Rust plugin crate for lifecycle, registry, persistence, and routing
- `tauri-plugin-window-system-api`: TypeScript API package for frontend integration
- `tauri-window-ui`: Solid structural wrapper for window chrome
- `examples/solid-host`: internal validation host only; not part of the published distribution

It covers the core pieces you usually need to ship a windowed desktop app:

- create and close windows with a stable `openWindow()` / `closeWindow()` flow
- build parent-child window trees and close children before parents
- persist and restore window geometry across normal close and restart
- broadcast registry updates and brokered messages between windows
- keep a thin TypeScript API for frontend integration
- provide a lightweight Solid UI shell for titlebar-style layouts
- include a validation host app that exercises the full stack

It does not try to be a full application framework.
The plugin owns window lifecycle, registry state, persistence, and message routing.
The UI package owns structure only, not styling or business logic.

## Workspace Overview

- `crates/tauri-plugin-window-system`
  - Rust plugin crate
- `packages/tauri-plugin-window-system-api`
  - TypeScript frontend API
- `packages/tauri-window-ui`
  - Solid window shell components
- `examples/solid-host`
  - Minimal host app for validation only

## What This Plugin Covers

### Window Lifecycle

- `open_window`
- `close_window`
- `list_windows`
- `restore_windows`
- `emit_to_window`
- `send_window_message`
- `broadcast_window_message`

The plugin is the source of truth for window lifecycle and registry state.
`close_window` and native `CloseRequested` use the same teardown path, and child windows are closed before their parent is removed.

### Persistence and Restore

Window geometry is stored at `app_data_dir/window-system/windows.json`.
Normal close does not delete saved geometry, so reopened windows restore their last known size and position.
Move and resize updates are coalesced, identical updates are ignored, and a `Drop` fallback handles the final flush.

Startup restore rehydrates tracked windows from the persisted snapshot.
Restore results include `restored`, `alreadyAlive`, and `skipped`.

### Messaging and Events

- `window-system:registry-changed`
- `window-system:message`

Registry changes are emitted back to the frontend so UI state can resync from the plugin snapshot.
Brokered window messages support direct, broadcast, and typed request/response flows.

### Layer Boundaries

- Rust plugin: lifecycle, registry, persistence, and event routing
- TypeScript API: thin `invoke` wrappers, typed helpers, and error parsing
- Solid UI wrapper: structural shell for window chrome
- Example host: validation UI, not plugin core

## Core APIs

### Rust Plugin

```rust
fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_window_system::init())
    .run(tauri::generate_context!())
    .expect("error while running Tauri application");
}
```

The plugin crate exposes the commands listed above and keeps `child_labels_of` and `children_of` separate:

- `child_labels_of` is the close-oriented traversal path
- `children_of` is the display and diagnostics path

### TypeScript API

Install:

```bash
npm install tauri-plugin-window-system-api
```

Common exports:

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

`openWindow()` trims blank `label`, `parent`, `title`, and `url` inputs before validation or use.
`restoreWindows()` reopens tracked windows parent-first and skips already-live windows.
`createWindowBus()` is generic so topic request and response types stay aligned.

### UI Wrapper

`packages/tauri-window-ui` provides the structural shell only.

Install:

```bash
npm install tauri-window-ui
```

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

Slots:

- `title`
- `meta`
- `actions`
- `footer`

Styling belongs to the host app.
Dragging is handled through the Tauri window API.

## Example Host

`examples/solid-host` is the validation app for:

- titlebar, meta, actions, and footer layout
- open / close / restore flows
- registry snapshots and child targeting
- direct and broadcast window messages
- startup restore and close behavior
- stale-first frontend caching while the plugin remains the registry source of truth

This app is intentionally kept internal to the workspace. It is not a published package and should not be treated as a consumer entrypoint.

Run it with:

```bash
cd examples/solid-host
pnpm install
pnpm tauri dev
```

## Development

Rust:

```bash
cargo test -p tauri-plugin-window-system
cargo check -p solid-host
```

TypeScript / frontend:

```bash
pnpm install
pnpm -r typecheck
pnpm -r build
```

## Release Workflow

Release instructions are in [RELEASE.md](./RELEASE.md).

## Notes

- `WindowGeometry` currently uses outer position and outer size for persistence
- Windows are created hidden, restored, and then shown to avoid startup flash on desktop platforms
- `open_window` trims blank inputs before the window is built
- `restoreWindows()` returns `restored`, `alreadyAlive`, and `skipped`
- `restoreWindows()` prunes unresolved children whose parent is missing
- `emitToWindow()` payloads must be JSON-serializable
- `sendWindowMessage()` targets must resolve to live labels
- `broadcastWindowMessage()` fans out through the broker event plane
- `listenRegistryChanges()` mirrors the registry-change event contract
- `parseWindowSystemError()` maps stable error codes for UI state and logging
- the UI wrapper does not depend on plugin internals
