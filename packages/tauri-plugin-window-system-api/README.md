# tauri-plugin-window-system-api

TypeScript API for the Tauri Window System plugin.

## Install

```bash
npm install tauri-plugin-window-system-api
```

## Public API

These exports are the stable frontend contract.

- `openWindow(request)`
- `closeWindow(label)`
- `listWindows()`
- `restoreWindows()`
- `emitToWindow(label, event, payload)`
- `sendWindowMessage(request)`
- `broadcastWindowMessage(request)`
- `listenWindowMessages(handler)`
- `listenRegistryChanges(handler)`
- `createWindowBus()`
- `parseWindowSystemError(value)`
- `WindowSystemErrorKind`
- `WindowSystemError`
- `WindowRegistryChangedEvent`
- `WindowMessageEnvelope`
- `WindowMessageKind`
- `WindowMessageScope`
- `WindowMessageTopicDefinition`
- `RestoreWindowsResult`
- `RestoreSkippedWindow`
- `RestoreSkipReason`

## Typed Topics

`createWindowBus()` is generic. Define a topic map once and let the compiler carry request and response payload types through the helper.

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

Rules:

- use `WindowMessageTopicDefinition<Request, Response>`
- omit the response type when a topic is event-only
- use `bus.request()` only for topics that declare a response payload
- use `bus.reply()` with a request envelope from the same topic

## Result Shapes

- `restoreWindows()` returns `{ restored, alreadyAlive, skipped }`
- skipped restore entries use `reason: "missing-parent"`
- registry-change events carry `{ kind, label, windows }`
- message envelopes use camelCase field names
- message scope and kind values are serialized strings, not numeric enums

## Example

```ts
import { openWindow } from "tauri-plugin-window-system-api";

await openWindow({
  label: "sub",
  parent: "main",
  title: "Sub Window",
});
```

## Notes

- This is a thin `invoke` wrapper
- The types mirror the Rust-side contract
- The package entrypoint resolves from `dist/index.js` in published builds
- This package is published as a public npm package from the repository release workflow
- `openWindow()` trims blank `label`, `parent`, `title`, and `url` inputs on the Rust side before the window is created
- Registry-change listeners resync frontend state from the plugin snapshot
- `restoreWindows()` reopens tracked windows during startup sync
- `restoreWindows()` restores windows parent-first and preserves the same geometry fallback behavior as `openWindow()`
- `restoreWindows()` reports unresolved children in `skipped` and prunes them from the persisted snapshot
- `restoreWindows()` returns `restored`, `alreadyAlive`, and `skipped`
- `parseWindowSystemError()` maps stable machine-readable error codes back to `WindowSystemErrorKind`
- `sendWindowMessage()` dispatches a typed direct envelope through the plugin broker
- `broadcastWindowMessage()` emits a typed broadcast envelope through the plugin broker
- `listenWindowMessages()` receives brokered envelopes via `window-system:message`
- `createWindowBus()` centralizes listen/send/request/reply wiring for consumers
