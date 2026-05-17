import type {
    RestoreWindowsResult,
    WindowDescriptor,
    WindowMessageEnvelope,
    WindowSystemError,
} from "tauri-plugin-window-system-api"
import type { UiPhase, WindowSystemMessageTopics, WindowViewModel } from "./types"

export function buildRegistryView(rows: WindowDescriptor[]) {
  const list = [...rows];
  const labels = new Set(list.map((window) => window.label));
  const childCounts = new Map<string, number>();

  for (const window of list) {
    if (window.parent) {
      childCounts.set(window.parent, (childCounts.get(window.parent) ?? 0) + 1);
    }
  }

  let orphanCount = 0;
  const viewRows: WindowViewModel[] = list.map((window) => {
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

  return { rows: viewRows, orphanCount };
}

export function buildChildRows(rows: WindowViewModel[], parentLabel: string) {
  return rows.filter((window) => window.parent === parentLabel).sort(compareWindowLabels);
}

export function reconcileSelectedTarget(rows: WindowViewModel[], current: string | null) {
  if (rows.length === 0) {
    return null;
  }

  if (!current || !rows.some((window) => window.label === current)) {
    return rows[rows.length - 1]?.label ?? null;
  }

  return current;
}

export function buildStatusSummary(args: {
  phase: UiPhase;
  statusMessage: string;
  windowCount: number;
  orphanCount: number;
  error: WindowSystemError | null;
}) {
  const base =
    `${args.phase} | ${args.statusMessage} | ${args.windowCount} window${args.windowCount === 1 ? "" : "s"}` +
    ` | ${args.orphanCount} orphan${args.orphanCount === 1 ? "" : "s"}`;

  return args.error ? `${base} | ${args.error.kind}: ${args.error.message}` : base;
}

export function buildFooterMessage(args: {
  error: WindowSystemError | null;
  restoreReport: RestoreWindowsResult | null;
  busMessage: string;
}) {
  if (args.error) {
    return `Last error: ${args.error.kind} - ${args.error.message}`;
  }

  if (args.restoreReport) {
    return formatRestoreSummary(args.restoreReport);
  }

  return args.busMessage;
}

export function formatWindowMessage(message: WindowMessageEnvelope<WindowSystemMessageTopics>) {
  const delivery = message.scope === "broadcast" ? "broadcast" : `to ${message.to ?? "unknown"}`;
  return `Last bus message: ${message.kind} ${delivery} ${message.topic} from ${message.from}`;
}

export function formatInlinePayload(payload: unknown) {
  if (payload && typeof payload === "object") {
    return JSON.stringify(payload);
  }

  return String(payload);
}

export function formatRestoreSummary(report: RestoreWindowsResult) {
  const skipped = report.skipped.length;
  const alreadyAlive = report.alreadyAlive.length;
  const restored = report.restored.length;

  return [`Restore summary: ${restored} restored`, `${alreadyAlive} already alive`, `${skipped} skipped`].join(
    ", ",
  );
}

export function nextChildLabel(labels: Iterable<string>) {
  let nextIndex = 1;

  for (const label of labels) {
    const match = /^child-(\d+)$/.exec(label);
    if (!match) {
      continue;
    }

    nextIndex = Math.max(nextIndex, Number.parseInt(match[1], 10) + 1);
  }

  return `child-${nextIndex}`;
}

export function compareWindowLabels(left: WindowViewModel, right: WindowViewModel) {
  return left.label.localeCompare(right.label);
}
