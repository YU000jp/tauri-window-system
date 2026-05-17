import {
    closeWindow,
    openWindow,
    parseWindowSystemError,
} from "tauri-plugin-window-system-api"
import { formatInlinePayload, nextChildLabel } from "./derived"
import type { UiPhase, WindowSystemBus, WindowSystemMessageTopics, WindowSystemStateSetter, WindowViewModel } from "./types"

interface WindowSystemOperationDeps {
  bus: WindowSystemBus<WindowSystemMessageTopics>;
  getSelectedTargetLabel: () => string | null;
  getChildRows: () => WindowViewModel[];
  getWindowCount: () => number;
  getOrphanCount: () => number;
  getPhase: () => UiPhase;
  setState: WindowSystemStateSetter;
  syncRegistrySnapshot: (includeRestore: boolean) => Promise<void>;
}

export interface WindowSystemOperations {
  refreshRegistry: () => Promise<void>;
  openChild: () => Promise<void>;
  pingChild: () => Promise<void>;
  closeChild: () => Promise<void>;
  broadcastStatus: () => Promise<void>;
}

export function createWindowSystemOperations(deps: WindowSystemOperationDeps): WindowSystemOperations {
  const runOperation = async (
    nextPhase: Extract<UiPhase, "opening" | "refreshing" | "closing">,
    nextMessage: string,
    successMessage: string,
    task: () => Promise<void>,
  ) => {
    // Keep busy-state transitions centralized so every action reports the same lifecycle shape.
    deps.setState("phase", nextPhase);
    deps.setState("statusMessage", nextMessage);
    deps.setState("uiError", null);

    try {
      await task();
      deps.setState("phase", "ready");
      deps.setState("statusMessage", successMessage);
    } catch (caught) {
      const normalizedError = parseWindowSystemError(caught);
      deps.setState("phase", "error");
      deps.setState("statusMessage", `${nextMessage} failed`);
      deps.setState("uiError", normalizedError);
    }
  };

  const refreshRegistry = async () => {
    await runOperation("refreshing", "Refreshing registry snapshot", "Registry snapshot refreshed", async () => {
      await deps.syncRegistrySnapshot(false);
    });
  };

  const openChild = async () => {
    await runOperation("opening", "Opening child window", "Child window opened", async () => {
      const parentLabel = deps.bus.label();
      const label = nextChildLabel(deps.getChildRows().map((window) => window.label));

      await openWindow({
        label,
        parent: parentLabel,
        url: "index.html",
        title: `Child Window ${label.slice("child-".length)}`,
      });

      deps.setState("selectedTargetLabel", label);
      await deps.bus.send({
        to: label,
        kind: "event",
        topic: "window-system:child-opened",
        payload: {
          openedAt: new Date().toISOString(),
          source: deps.bus.label(),
        },
      });
    });
  };

  const pingChild = async () => {
    const targetLabel = deps.getSelectedTargetLabel();
    if (!targetLabel) {
      return;
    }

    await runOperation("refreshing", "Requesting a child reply", "Child replied", async () => {
      const reply = await deps.bus.request({
        to: targetLabel,
        topic: "window-system:ping",
        payload: {
          at: new Date().toISOString(),
          source: deps.bus.label(),
        },
      });

      deps.setState("busMessage", `Reply from ${reply.from}: ${formatInlinePayload(reply.payload)}`);
    });
  };

  const closeChild = async () => {
    const targetLabel = deps.getSelectedTargetLabel();
    if (!targetLabel) {
      return;
    }

    await runOperation("closing", "Closing child window", "Child window closed", async () => {
      await closeWindow(targetLabel);
    });
  };

  const broadcastStatus = async () => {
    await runOperation("refreshing", "Broadcasting status", "Status broadcast sent", async () => {
      const summary = {
        source: deps.bus.label(),
        windows: deps.getWindowCount(),
        orphans: deps.getOrphanCount(),
        phase: deps.getPhase(),
      };

      await deps.bus.broadcast({
        kind: "event",
        topic: "window-system:status",
        payload: summary,
      });
      deps.setState("busMessage", `Broadcasted status: ${formatInlinePayload(summary)}`);
    });
  };

  return {
    refreshRegistry,
    openChild,
    pingChild,
    closeChild,
    broadcastStatus,
  };
}
