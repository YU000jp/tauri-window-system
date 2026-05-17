import { For, Show, createSelector, type Accessor } from "solid-js";
import type { WindowViewModel } from "./windowSystem";

interface RegistryPanelProps {
  isRootWindow: boolean;
  registryReady: Accessor<boolean>;
  registryRows: Accessor<WindowViewModel[]>;
  selectedTargetLabel: Accessor<string | null>;
  onSelectTarget: (label: string) => void;
}

interface RegistryCardProps {
  window: WindowViewModel;
  isSelected: boolean;
  selectable: boolean;
  onSelect: () => void;
}

export default function RegistryPanel(props: RegistryPanelProps) {
  const isSelectedTarget = createSelector(props.selectedTargetLabel);

  return (
    <section class="data-panel surface-shell" aria-label="registry snapshot">
      <header>
        <h2>Registry snapshot</h2>
        <p>Each row mirrors the plugin registry and highlights parent, geometry, and child counts.</p>
      </header>

      <Show
        when={props.isRootWindow}
        fallback={<p>Registry access is root-only in child windows.</p>}
      >
        <Show when={props.registryReady()} fallback={<p>Restoring tracked windows...</p>}>
          <Show when={props.registryRows().length > 0} fallback={<p>No windows are registered yet.</p>}>
            <div class="data-grid">
              <For each={props.registryRows()}>
                {(window) => (
                  <RegistryCard
                    window={window}
                    isSelected={isSelectedTarget(window.label)}
                    selectable={window.parent === "main"}
                    onSelect={() => props.onSelectTarget(window.label)}
                  />
                )}
              </For>
            </div>
          </Show>
        </Show>
      </Show>
    </section>
  );
}

function RegistryCard(props: RegistryCardProps) {
  return (
    <article
      role={props.selectable ? "button" : undefined}
      tabIndex={props.selectable ? 0 : undefined}
      class="data-card surface-shell"
      classList={{
        "data-card--orphan": props.window.orphan,
        "data-card--selectable": props.selectable,
        "data-card--selected": props.isSelected,
      }}
      aria-pressed={props.selectable ? props.isSelected : undefined}
      onClick={() => {
        if (props.selectable) {
          props.onSelect();
        }
      }}
      onKeyDown={(event) => {
        if (!props.selectable) {
          return;
        }

        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          props.onSelect();
        }
      }}
    >
      <div class="data-card__header">
        <div>
          <strong>{props.window.label}</strong>
          <span>{props.window.url}</span>
        </div>

        <div>
          <span
            classList={{
              tag: true,
              "tag--muted": !props.window.parent,
              "tag--warning": props.window.orphan,
            }}
          >
            {props.window.parent ? `parent: ${props.window.parent}` : "root"}
          </span>
          <span class="tag">
            {props.window.childCount} child{props.window.childCount === 1 ? "" : "ren"}
          </span>
          <Show when={props.isSelected}>
            <span class="tag tag--active">selected</span>
          </Show>
          <span
            classList={{
              tag: true,
              "tag--muted": !props.window.geometry,
            }}
          >
            {props.window.geometry ? "geometry saved" : "geometry unset"}
          </span>
        </div>
      </div>

      <dl>
        <div>
          <dt>Parent</dt>
          <dd>{props.window.parent ?? "none"}</dd>
        </div>
        <div>
          <dt>Geometry</dt>
          <dd>{formatGeometry(props.window.geometry)}</dd>
        </div>
      </dl>
    </article>
  );
}

function formatGeometry(geometry: { width: number; height: number; x: number; y: number } | null) {
  if (!geometry) {
    return "not restored";
  }

  const width = Math.round(geometry.width);
  const height = Math.round(geometry.height);
  const x = Math.round(geometry.x);
  const y = Math.round(geometry.y);

  return `${width} x ${height} @ ${x}, ${y}`;
}
