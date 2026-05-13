import { createMemo, createSignal, For, onMount, Show } from "solid-js";
import { WindowFrame } from "tauri-window-ui";
import {
  closeWindow,
  emitToWindow,
  listWindows,
  openWindow,
  type WindowDescriptor,
} from "tauri-plugin-window-system-api";

type UiPhase = "idle" | "opening" | "ready" | "refreshing" | "closing" | "error";

type WindowViewModel = WindowDescriptor & {
  childCount: number;
  orphan: boolean;
};

export default function App() {
  const [windows, setWindows] = createSignal<WindowDescriptor[]>([]);
  const [phase, setPhase] = createSignal<UiPhase>("idle");
  const [message, setMessage] = createSignal("Loading registry snapshot");
  const [error, setError] = createSignal<string | null>(null);

  const isBusy = createMemo(() => phase() === "opening" || phase() === "refreshing" || phase() === "closing");
  const windowCount = createMemo(() => windows().length);

  // Keep registry-derived facts in one memo so the summary and cards stay in sync.
  const registryView = createMemo(() => {
    const list = windows();
    const labels = new Set(list.map((window) => window.label));
    const childCounts = new Map<string, number>();

    for (const window of list) {
      if (window.parent) {
        childCounts.set(window.parent, (childCounts.get(window.parent) ?? 0) + 1);
      }
    }

    let orphanCount = 0;
    const rows: WindowViewModel[] = list.map((window) => {
      const orphan = window.parent ? !labels.has(window.parent) : false;
      if (orphan) {
        orphanCount += 1;
      }

      return {
        ...window,
        childCount: childCounts.get(window.label) ?? 0,
        orphan,
      };
    });

    return { rows, orphanCount };
  });

  const windowRows = createMemo(() => registryView().rows);
  const orphanCount = createMemo(() => registryView().orphanCount);

  const statusSummary = createMemo(() => {
    const base =
      `${phase()} | ${message()} | ${windowCount()} window${windowCount() === 1 ? "" : "s"}` +
      ` | ${orphanCount()} orphan${orphanCount() === 1 ? "" : "s"}`;
    return error() ? `${base} | ${error()}` : base;
  });

  const footerMessage = createMemo(() =>
    error()
      ? `Last error: ${error()}`
      : "The registry snapshot below is read from the Rust plugin registry.",
  );

  const toErrorMessage = (value: unknown) => (value instanceof Error ? value.message : String(value));

  const refreshSnapshot = async () => {
    setWindows(await listWindows());
  };

  const runOperation = async (
    nextPhase: Extract<UiPhase, "opening" | "refreshing" | "closing">,
    nextMessage: string,
    successMessage: string,
    task: () => Promise<void>,
  ) => {
    // Centralize phase transitions so every action reports busy, success, and error states
    // in the same shape.
    setPhase(nextPhase);
    setMessage(nextMessage);
    setError(null);

    try {
      await task();
      setPhase("ready");
      setMessage(successMessage);
    } catch (caught) {
      setPhase("error");
      setMessage(`${nextMessage} failed`);
      setError(toErrorMessage(caught));
    }
  };

  const refreshRegistry = async () => {
    await runOperation("refreshing", "Refreshing registry snapshot", "Registry snapshot refreshed", async () => {
      await refreshSnapshot();
    });
  };

  const openChild = async () => {
    await runOperation("opening", "Opening child window", "Child window opened", async () => {
      await openWindow({
        label: "child",
        parent: "main",
        url: "index.html",
        title: "Child Window",
      });
      await refreshSnapshot();
    });
  };

  const pingChild = async () => {
    await runOperation("refreshing", "Routing event to child", "Event routed", async () => {
      await emitToWindow("child", "window-system:ping", {
        at: new Date().toISOString(),
      });
    });
  };

  const closeChild = async () => {
    await runOperation("closing", "Closing child window", "Child window closed", async () => {
      await closeWindow("child");
      await refreshSnapshot();
    });
  };

  onMount(() => {
    void refreshRegistry();
  });

  return (
    <div class="app-shell">
      <WindowFrame
        title={<span>Tauri Window System</span>}
        meta={<span aria-live="polite">{statusSummary()}</span>}
        actions={
          <button type="button" onClick={refreshRegistry} disabled={isBusy()}>
            Refresh
          </button>
        }
        footer={
          <div class="window-frame__footer-stack">
            <p class="window-frame__footer-copy">{footerMessage()}</p>
            <Show when={error()}>
              <p class="window-frame__footer-error" role="alert">
                {error()}
              </p>
            </Show>
          </div>
        }
      >
        <section class="status-grid" aria-label="window-system summary">
          <article class="status-card">
            <span class="status-card__label">Phase</span>
            <strong class="status-card__value">{phase()}</strong>
          </article>
          <article class="status-card">
            <span class="status-card__label">Windows</span>
            <strong class="status-card__value">{windowCount()}</strong>
          </article>
          <article class="status-card">
            <span class="status-card__label">Orphans</span>
            <strong class="status-card__value">{orphanCount()}</strong>
          </article>
        </section>

        <section class="controls" aria-label="window operations">
          <button type="button" onClick={openChild} disabled={isBusy()}>
            Open child
          </button>
          <button type="button" onClick={pingChild} disabled={isBusy()}>
            Emit event
          </button>
          <button type="button" onClick={closeChild} disabled={isBusy()}>
            Close child
          </button>
        </section>

        <section class="window-list" aria-label="registry snapshot">
          <header class="window-list__header">
            <h2>Registry snapshot</h2>
            <p>Each row mirrors the plugin registry and highlights parent, geometry, and child counts.</p>
          </header>

          <Show
            when={windowRows().length > 0}
            fallback={<p class="window-list__empty">No windows are registered yet.</p>}
          >
            <div class="registry-grid">
              <For each={windowRows()}>
                {(window) => (
                  <article
                    classList={{
                      "registry-card": true,
                      "registry-card--orphan": window.orphan,
                    }}
                  >
                    <div class="registry-card__header">
                      <div class="registry-card__identity">
                        <strong>{window.label}</strong>
                        <span>{window.url}</span>
                      </div>

                      <div class="registry-card__badges">
                        <span
                          classList={{
                            badge: true,
                            "badge--muted": !window.parent,
                            "badge--warning": window.orphan,
                          }}
                        >
                          {window.parent ? `parent: ${window.parent}` : "root"}
                        </span>
                        <span class="badge">
                          {window.childCount} child{window.childCount === 1 ? "" : "ren"}
                        </span>
                        <span
                          classList={{
                            badge: true,
                            "badge--muted": !window.geometry,
                          }}
                        >
                          {window.geometry ? "geometry saved" : "geometry unset"}
                        </span>
                      </div>
                    </div>

                    <dl class="registry-card__details">
                      <div>
                        <dt>Parent</dt>
                        <dd>{window.parent ?? "none"}</dd>
                      </div>
                      <div>
                        <dt>Geometry</dt>
                        <dd>{formatGeometry(window.geometry)}</dd>
                      </div>
                    </dl>
                  </article>
                )}
              </For>
            </div>
          </Show>
        </section>
      </WindowFrame>
    </div>
  );
}

function formatGeometry(geometry: WindowDescriptor["geometry"]) {
  if (!geometry) {
    return "not restored";
  }

  const width = Math.round(geometry.width);
  const height = Math.round(geometry.height);
  const x = Math.round(geometry.x);
  const y = Math.round(geometry.y);

  return `${width} x ${height} @ ${x}, ${y}`;
}
