use crate::registry::{ClosingGuard, WindowRegistry};
use tauri::{AppHandle, CloseRequestApi, Manager, Runtime, WebviewWindowBuilder};

pub fn attach_native_parent<'a, R: Runtime, M: Manager<R>>(
    handle: &'a M,
    builder: WebviewWindowBuilder<'a, R, M>,
    parent_label: Option<&str>,
) -> Result<WebviewWindowBuilder<'a, R, M>, String> {
    let Some(parent_label) = parent_label else {
        return Ok(builder);
    };

    let parent_window = handle
        .get_webview_window(parent_label)
        .ok_or_else(|| format!("parent window not found: {parent_label}"))?;

    // Delegate the OS-specific owner/child wiring to Tauri so z-order and teardown
    // behavior stay aligned with the native platform semantics.
    builder.parent(&parent_window).map_err(|err| err.to_string())
}

pub fn handle_close_requested<R: Runtime>(
    handle: &AppHandle<R>,
    registry: &WindowRegistry,
    label: &str,
    api: &CloseRequestApi,
) {
    if let Ok(Some(guard)) = registry.begin_closing(label) {
        // Only the first close request is blocked. Recursive close events raised by the
        // plugin's own teardown are allowed through so the native teardown path can complete.
        api.prevent_close();

        let handle = handle.clone();
        let registry = registry.clone();
        let label = label.to_string();

        tauri::async_runtime::spawn(async move {
            let _guard: ClosingGuard = guard;
            if let Err(err) = close_window_tree(&handle, &registry, &label, true).await {
                eprintln!(
                    "{}",
                    format_teardown_failure("close-requested", &label, &err)
                );
            }
        });
    }
}

pub async fn close_window_tree<R: Runtime>(
    handle: &AppHandle<R>,
    registry: &WindowRegistry,
    label: &str,
    already_marked: bool,
) -> Result<(), String> {
    enum Frame {
        Enter { label: String, already_marked: bool },
        Exit {
            label: String,
            guard: Option<ClosingGuard>,
        },
    }

    let mut stack = vec![Frame::Enter {
        label: label.to_string(),
        already_marked,
    }];
    let mut teardown_errors = Vec::new();

    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Enter {
                label,
                already_marked,
            } => {
                let guard = if already_marked {
                    None
                } else {
                    match registry.begin_closing(&label)? {
                        Some(guard) => Some(guard),
                        None => continue,
                    }
                };

                stack.push(Frame::Exit {
                    label: label.clone(),
                    guard,
                });

                let mut children = registry.children_of(&label)?;
                children.reverse();
                for child in children {
                    stack.push(Frame::Enter {
                        label: child.label,
                        already_marked: false,
                    });
                }
            }
            Frame::Exit { label, guard } => {
                let mut errors = Vec::new();

                if let Some(window) = handle.get_webview_window(&label) {
                    if let Err(err) = window.destroy() {
                        errors.push(format!("destroy failed: {err}"));
                    }
                }

                if let Err(err) = registry.remove(&label) {
                    errors.push(format!("registry remove failed: {err}"));
                }
                drop(guard);

                teardown_errors.extend(errors.into_iter().map(|err| format!("{label}: {err}")));
            }
        }
    }

    if teardown_errors.is_empty() {
        Ok(())
    } else {
        Err(teardown_errors.join("; "))
    }
}

pub fn format_teardown_failure(stage: &str, label: &str, error: &str) -> String {
    format!("window-system: teardown failed stage={stage} label={label} error={error}")
}

#[cfg(test)]
mod tests {
    use super::format_teardown_failure;

    #[test]
    fn teardown_failure_message_includes_context() {
        let message = format_teardown_failure("close-requested", "main", "boom");

        assert!(message.contains("stage=close-requested"));
        assert!(message.contains("label=main"));
        assert!(message.contains("error=boom"));
    }

    #[test]
    fn teardown_error_message_is_collectable() {
        let message = format!("main: destroy failed: boom");

        assert!(message.contains("main"));
        assert!(message.contains("destroy failed"));
        assert!(message.contains("boom"));
    }
}
