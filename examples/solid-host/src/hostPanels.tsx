import { Show, type Accessor } from "solid-js";
import type { WindowSystemError } from "tauri-plugin-window-system-api";
import type { UiPhase } from "./windowSystem";

interface StatusPanelProps {
  phase: Accessor<UiPhase>;
  windowCount: Accessor<number>;
  orphanCount: Accessor<number>;
}

interface ControlBarProps {
  selectedTargetLabel: Accessor<string | null>;
  canOpenChild: Accessor<boolean>;
  canRequestSelected: Accessor<boolean>;
  canBroadcastStatus: Accessor<boolean>;
  canCloseSelected: Accessor<boolean>;
  onOpenChild: () => void;
  onRequestSelected: () => void;
  onBroadcastStatus: () => void;
  onCloseSelected: () => void;
}

interface FooterPanelProps {
  footerMessage: Accessor<string>;
  error: Accessor<WindowSystemError | null>;
}

export function StatusPanel(props: StatusPanelProps) {
  return (
    <section class="overview-grid" aria-label="window-system summary">
      <article class="surface-shell">
        <span>Phase</span>
        <strong>{props.phase()}</strong>
      </article>
      <article class="surface-shell">
        <span>Windows</span>
        <strong>{props.windowCount()}</strong>
      </article>
      <article class="surface-shell">
        <span>Orphans</span>
        <strong>{props.orphanCount()}</strong>
      </article>
    </section>
  );
}

export function ControlBar(props: ControlBarProps) {
  const selectedTargetLabel = props.selectedTargetLabel;

  return (
    <section class="action-bar" aria-label="window operations">
      <Show
        when={selectedTargetLabel()}
        fallback={<p>Select a child row to target Request and Close.</p>}
      >
        {(label) => <p>Selected child: {label()}</p>}
      </Show>
      <button type="button" onClick={props.onOpenChild} disabled={!props.canOpenChild()}>
        Open child
      </button>
      <button type="button" onClick={props.onRequestSelected} disabled={!props.canRequestSelected()}>
        Request selected child
      </button>
      <button type="button" onClick={props.onBroadcastStatus} disabled={!props.canBroadcastStatus()}>
        Broadcast status
      </button>
      <button type="button" onClick={props.onCloseSelected} disabled={!props.canCloseSelected()}>
        Close selected child
      </button>
    </section>
  );
}

export function FooterPanel(props: FooterPanelProps) {
  return (
    <div class="host-footer">
      <p>{props.footerMessage()}</p>
      <Show when={props.error()}>
        <p role="alert">
          {props.error()?.kind}: {props.error()?.message}
        </p>
      </Show>
    </div>
  );
}
