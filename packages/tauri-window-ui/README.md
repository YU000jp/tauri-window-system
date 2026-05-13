# tauri-window-ui

Solid window wrapper for Tauri applications.

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
- Internally uses `tauri-controls`

