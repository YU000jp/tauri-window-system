import { startTransition } from "solid-js"
import {
    listenRegistryChanges,
    listWindows,
    parseWindowSystemError,
    restoreWindows,
    type WindowDescriptor,
    type WindowMessageEnvelope,
} from "tauri-plugin-window-system-api"
import { markBoot, measureBoot, scheduleAfterFirstPaint } from "../bootTelemetry"
import { formatWindowMessage } from "./derived"
import type {
    RegistrySnapshotCache,
    RegistryStateSetter,
    WindowSystemBus,
    WindowSystemMessageTopics,
    WindowSystemStateSetter,
} from "./types"

interface WindowSystemLifecycleDeps {
  isRootWindow: boolean;
  bus: WindowSystemBus<WindowSystemMessageTopics>;
  setState: WindowSystemStateSetter;
  setRegistry: RegistryStateSetter;
  applyRegistrySnapshot: (windows: WindowDescriptor[]) => void;
  readRegistrySnapshotCache: () => RegistrySnapshotCache | null;
}

export interface WindowSystemLifecycle {
  mount: () => void;
  cleanup: () => void;
  syncRegistrySnapshot: (includeRestore: boolean) => Promise<void>;
}

export function createWindowSystemLifecycle(deps: WindowSystemLifecycleDeps): WindowSystemLifecycle {
  let detachWindowBus: (() => void) | null = null;
  let detachRegistryListener: (() => void) | null = null;
  let disposed = false;
  let bootstrapStarted = false;
  let listenerReadyCount = 0;
  const expectedListenerReadyCount = deps.isRootWindow ? 2 : 1;
  const cachedRegistrySnapshot = deps.isRootWindow ? deps.readRegistrySnapshotCache() : null;

  if (deps.isRootWindow && cachedRegistrySnapshot) {
    deps.setRegistry("rows", cachedRegistrySnapshot.rows);
    deps.setRegistry("ready", true);
    deps.setState("phase", "refreshing");
    deps.setState("statusMessage", "Showing cached registry snapshot");
    markBoot("cached registry ready");
    measureBoot("render -> cached registry ready", "render start", "cached registry ready");
  }

  const noteListenerReady = () => {
    listenerReadyCount += 1;
    if (listenerReadyCount === expectedListenerReadyCount) {
      markBoot("listeners ready");
      measureBoot("render -> listeners ready", "render start", "listeners ready");
    }
  };

  const syncRegistrySnapshot = async (includeRestore: boolean) => {
    if (!deps.isRootWindow) {
      return;
    }

    if (includeRestore) {
      const report = await restoreWindows();
      deps.setState("restoreReport", report);
    }

    const windows = await listWindows();
    deps.applyRegistrySnapshot(windows);
  };

  const startWindowBusListener = () => {
    void deps
      .bus
      .listen((message) => {
        deps.setState("busMessage", formatWindowMessage(message));

        if (message.kind === "request" && message.topic === "window-system:ping") {
          const pingRequest = message as WindowMessageEnvelope<
            WindowSystemMessageTopics,
            "window-system:ping",
            "request"
          >;

          void deps.bus.reply(pingRequest, {
            label: deps.bus.label(),
            receivedAt: new Date().toISOString(),
          });
        }
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }

        detachWindowBus = unlisten;
        noteListenerReady();
      })
      .catch((caught) => {
        deps.setState("phase", "error");
        deps.setState("statusMessage", "Window bus listener failed");
        deps.setState("uiError", parseWindowSystemError(caught));
      });
  };

  const startRegistryListener = () => {
    if (!deps.isRootWindow) {
      return;
    }

    void listenRegistryChanges((event) => {
      deps.applyRegistrySnapshot(event.windows);
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
          return;
        }

        detachRegistryListener = unlisten;
        noteListenerReady();
      })
      .catch((caught) => {
        deps.setState("phase", "error");
        deps.setState("statusMessage", "Registry listener failed");
        deps.setState("uiError", parseWindowSystemError(caught));
      });
  };

  const startRootBootstrap = async () => {
    if (!deps.isRootWindow || bootstrapStarted) {
      return;
    }

    bootstrapStarted = true;
    markBoot("bootstrap start");

    deps.setState("phase", "refreshing");
    deps.setState(
      "statusMessage",
      cachedRegistrySnapshot ? "Refreshing live registry snapshot" : "Loading registry snapshot",
    );
    deps.setState("uiError", null);

    try {
      await syncRegistrySnapshot(true);
      if (disposed) {
        return;
      }

      startTransition(() => {
        deps.setState("phase", "ready");
        deps.setState("statusMessage", "Registry snapshot ready");
      });
      markBoot("registry ready");
      measureBoot("render -> registry ready", "render start", "registry ready");
    } catch (caught) {
      if (disposed) {
        return;
      }

      deps.setState("phase", "error");
      deps.setState("statusMessage", "Registry snapshot failed");
      deps.setState("uiError", parseWindowSystemError(caught));
    }
  };

  const mount = () => {
    scheduleAfterFirstPaint(() => {
      if (disposed) {
        return;
      }

      if (deps.isRootWindow) {
        startTransition(() => {
          deps.setState("heavyPanelsVisible", true);
        });
        markBoot("interactive ready");
        measureBoot("render -> interactive ready", "render start", "interactive ready");
      }

      startWindowBusListener();

      if (deps.isRootWindow) {
        startRegistryListener();
        void startRootBootstrap();
      } else {
        deps.setState("phase", "ready");
      }
    });
  };

  const cleanup = () => {
    disposed = true;
    detachWindowBus?.();
    detachWindowBus = null;
    detachRegistryListener?.();
    detachRegistryListener = null;
  };

  return { mount, cleanup, syncRegistrySnapshot };
}
