# tauri-window-ui

Solid window wrapper for Tauri applications.

## Install

```bash
npm install tauri-window-ui
```

## Export

- `WindowFrame`

## Example

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

## Slots

- `title`
- `meta`
- `actions`
- `footer`

## Notes

- `header` is not supported
- Styling is owned by the host app; this package only provides the structural shell.
- Dragging is handled directly through the Tauri window API
- The published entrypoint resolves from `dist/index.js`
- This package is published as a public npm package from the repository release workflow
