# tauri-plugin-window-system

Tauri 2 plugin for multi-window lifecycle management.

## Public Commands

- `plugin:window-system|open_window`
- `plugin:window-system|close_window`
- `plugin:window-system|list_windows`
- `plugin:window-system|emit_to_window`

## Rust Responsibilities

- Window creation
- Registry management
- Size and position persistence
- Parent-child close chaining
- Window event routing

## Minimal Usage

```rust
fn main() {
  let app = tauri::Builder::default()
    .plugin(tauri_plugin_os::init())
    .plugin(tauri_plugin_window_system::init())
    .build(tauri::generate_context!())
    .expect("error while building Tauri application");

  app.run(|app_handle, event| {
    if matches!(event, tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit) {
      if let Err(err) = app_handle.state::<tauri_plugin_window_system::WindowRegistry>().flush() {
        eprintln!("window-system: host exit flush failed: {err}");
      }
    }
  });
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
- `parent` must refer to an existing label
- `url` defaults to `index.html`
- `geometry` overrides restored values when present

## Close Behavior

- `close_window` and native close events share the same teardown logic
- the first `CloseRequested` is prevented so the plugin can finish child-first teardown safely
- child windows are closed before their parent is removed
- saved geometry is preserved across close/reopen cycles
- windows are created hidden and shown after geometry restoration to reduce startup flash
- identical geometry updates are skipped so move/resize storms do not thrash the registry or persistence layer
- the plugin flushes persisted geometry on app exit and also keeps a Drop fallback

## Native Parent Handling

- `parent` is resolved to a live Tauri window before the child is created
- the child is linked to the OS-native parent/owner relation through Tauri's builder API
- if the parent label does not exist or is already closing, window creation is rejected

## Persistence

Saved to `app_data_dir/window-system/windows.json`.

## Permissions

`permissions/default.toml` enables:

- `allow-open-window`
- `allow-close-window`
- `allow-list-windows`
- `allow-emit-to-window`

## Tests

- registry insert / list / child filtering
- geometry persist / restore
- `cargo check -p tauri-plugin-window-system`
