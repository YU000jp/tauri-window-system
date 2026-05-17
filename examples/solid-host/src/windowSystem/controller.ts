import { createEffect, createMemo, onCleanup, onMount, startTransition } from "solid-js"
import { createStore } from "solid-js/store"
import { createWindowBus, type WindowDescriptor } from "tauri-plugin-window-system-api"
import { createWindowSystemLifecycle } from "./bootstrap"
import { buildChildRows, buildFooterMessage, buildRegistryView, buildStatusSummary, reconcileSelectedTarget } from "./derived"
import { createWindowSystemOperations } from "./operations"
import { queueRegistrySnapshotCache, readRegistrySnapshotCache } from "./persistence"
import type {
    RegistryState,
    WindowSystemController,
    WindowSystemMessageTopics,
    WindowSystemState,
} from "./types"

export function useWindowSystem(): WindowSystemController {
  return createWindowSystemController();
}

export function createWindowSystemController(): WindowSystemController {
  const bus = createWindowBus<WindowSystemMessageTopics>();
  const isRootWindow = bus.label() === "main";
  const [state, setState] = createStore<WindowSystemState>({
    phase: isRootWindow ? "idle" : "ready",
    statusMessage: isRootWindow ? "Booting shell before registry hydration" : "Child window ready",
    uiError: null,
    restoreReport: null,
    busMessage: "No window-bus messages yet",
    selectedTargetLabel: null,
    heavyPanelsVisible: !isRootWindow,
  });
  const [registry, setRegistry] = createStore<RegistryState>({
    rows: [],
    ready: !isRootWindow,
  });

  const applyRegistrySnapshot = (windows: WindowDescriptor[]) => {
    startTransition(() => {
      setRegistry("rows", windows);
      setRegistry("ready", true);
    });
    queueRegistrySnapshotCache(windows);
  };

  const lifecycle = createWindowSystemLifecycle({
    isRootWindow,
    bus,
    setState,
    setRegistry,
    applyRegistrySnapshot,
    readRegistrySnapshotCache,
  });

  const registryItems = createMemo(() => registry.rows);
  const registryReady = createMemo(() => !isRootWindow || registry.ready);
  const isBusy = createMemo(() => state.phase === "opening" || state.phase === "refreshing" || state.phase === "closing");
  const registryView = createMemo(() => buildRegistryView(registryItems()));
  const registryRows = createMemo(() => registryView().rows);
  const childRows = createMemo(() => buildChildRows(registryRows(), bus.label()));
  const windowCount = createMemo(() => registryItems().length);
  const orphanCount = createMemo(() => registryView().orphanCount);
  const error = createMemo(() => state.uiError);

  // Keep the selected target pinned to a live child so request and close actions never hit stale labels.
  createEffect(() => {
    const next = reconcileSelectedTarget(childRows(), state.selectedTargetLabel);
    if (next !== state.selectedTargetLabel) {
      setState("selectedTargetLabel", next);
    }
  });

  const statusSummary = createMemo(() =>
    buildStatusSummary({
      phase: state.phase,
      statusMessage: state.statusMessage,
      windowCount: windowCount(),
      orphanCount: orphanCount(),
      error: error(),
    }),
  );

  const footerMessage = createMemo(() =>
    buildFooterMessage({
      error: error(),
      restoreReport: state.restoreReport,
      busMessage: state.busMessage,
    }),
  );

  const canRefresh = createMemo(() => isRootWindow && !isBusy() && (registryReady() || state.phase === "error"));
  const canOpenChild = createMemo(() => isRootWindow && state.phase === "ready" && !isBusy());
  const canRequestSelected = createMemo(
    () => isRootWindow && state.phase === "ready" && !isBusy() && state.selectedTargetLabel !== null,
  );
  const canBroadcastStatus = createMemo(() => isRootWindow && state.phase === "ready" && !isBusy());
  const canCloseSelected = createMemo(
    () => isRootWindow && state.phase === "ready" && !isBusy() && state.selectedTargetLabel !== null,
  );

  const operations = createWindowSystemOperations({
    bus,
    getSelectedTargetLabel: () => state.selectedTargetLabel,
    getChildRows: () => childRows(),
    getWindowCount: () => windowCount(),
    getOrphanCount: () => orphanCount(),
    getPhase: () => state.phase,
    setState,
    syncRegistrySnapshot: lifecycle.syncRegistrySnapshot,
  });

  onMount(lifecycle.mount);
  onCleanup(lifecycle.cleanup);

  return {
    isRootWindow: () => isRootWindow,
    phase: () => state.phase,
    statusMessage: () => state.statusMessage,
    error,
    busMessage: () => state.busMessage,
    registryRows,
    registryReady,
    isBusy,
    windowCount,
    childRows,
    selectedTargetLabel: () => state.selectedTargetLabel,
    orphanCount,
    statusSummary,
    footerMessage,
    canRefresh,
    canOpenChild,
    canRequestSelected,
    canBroadcastStatus,
    canCloseSelected,
    refreshRegistry: operations.refreshRegistry,
    openChild: operations.openChild,
    pingChild: operations.pingChild,
    closeChild: operations.closeChild,
    broadcastStatus: operations.broadcastStatus,
    selectTarget: (label: string) => {
      if (childRows().some((window) => window.label === label)) {
        setState("selectedTargetLabel", label);
      }
    },
    heavyPanelsVisible: () => state.heavyPanelsVisible,
  };
}
