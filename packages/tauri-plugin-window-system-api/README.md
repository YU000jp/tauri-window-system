# tauri-plugin-window-system-api

TypeScript API for the Tauri Window System plugin.

## Exports

- `openWindow(request)`
- `closeWindow(label)`
- `listWindows()`
- `emitToWindow(label, event, payload)`

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

