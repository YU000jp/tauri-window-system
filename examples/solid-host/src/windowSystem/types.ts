import type {
    BroadcastWindowMessageRequest,
    RestoreWindowsResult,
    SendWindowMessageRequest,
    WindowDescriptor,
    WindowMessageEnvelope,
    WindowMessageKind,
    WindowMessageRequestTopics,
    WindowMessageResponsePayload,
    WindowMessageTopicDefinition,
    WindowMessageTopicMap,
    WindowMessageTopicName,
    WindowSystemError,
} from "tauri-plugin-window-system-api"

export type UiPhase = "idle" | "opening" | "ready" | "refreshing" | "closing" | "error";

export type WindowViewModel = WindowDescriptor & {
  childCount: number;
  orphan: boolean;
};

export type WindowSystemMessageTopics = {
  "window-system:child-opened": WindowMessageTopicDefinition<{
    openedAt: string;
    source: string;
  }>;
  "window-system:ping": WindowMessageTopicDefinition<
    {
      at: string;
      source: string;
    },
    {
      label: string;
      receivedAt: string;
    }
  >;
  "window-system:status": WindowMessageTopicDefinition<{
    source: string;
    windows: number;
    orphans: number;
    phase: UiPhase;
  }>;
};

export interface WindowSystemState {
  phase: UiPhase;
  statusMessage: string;
  uiError: WindowSystemError | null;
  restoreReport: RestoreWindowsResult | null;
  busMessage: string;
  selectedTargetLabel: string | null;
  heavyPanelsVisible: boolean;
}

export interface RegistryState {
  rows: WindowDescriptor[];
  ready: boolean;
}

export interface RegistrySnapshotCache {
  version: 1;
  savedAt: string;
  rows: WindowDescriptor[];
}

export type WindowSystemStateSetter = <K extends keyof WindowSystemState>(
  key: K,
  value: WindowSystemState[K],
) => void;

export type RegistryStateSetter = <K extends keyof RegistryState>(key: K, value: RegistryState[K]) => void;

export interface WindowSystemController {
  isRootWindow: () => boolean;
  phase: () => UiPhase;
  statusMessage: () => string;
  error: () => WindowSystemError | null;
  busMessage: () => string;
  registryRows: () => WindowViewModel[];
  registryReady: () => boolean;
  isBusy: () => boolean;
  windowCount: () => number;
  childRows: () => WindowViewModel[];
  selectedTargetLabel: () => string | null;
  orphanCount: () => number;
  statusSummary: () => string;
  footerMessage: () => string;
  canRefresh: () => boolean;
  canOpenChild: () => boolean;
  canRequestSelected: () => boolean;
  canBroadcastStatus: () => boolean;
  canCloseSelected: () => boolean;
  refreshRegistry: () => Promise<void>;
  openChild: () => Promise<void>;
  pingChild: () => Promise<void>;
  closeChild: () => Promise<void>;
  broadcastStatus: () => Promise<void>;
  selectTarget: (label: string) => void;
  heavyPanelsVisible: () => boolean;
}

export interface WindowSystemBus<TTopics extends WindowMessageTopicMap = WindowSystemMessageTopics> {
  label: () => string;
  listen: (
    handler: (message: WindowMessageEnvelope<TTopics>) => void,
    filter?: {
      topic?: WindowMessageTopicName<TTopics>;
      kind?: WindowMessageKind | WindowMessageKind[];
      from?: string;
      to?: string;
    },
  ) => Promise<() => void>;
  send: <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
    request: SendWindowMessageRequest<TTopics, TTopic, TKind>,
  ) => Promise<void>;
  broadcast: <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
    request: BroadcastWindowMessageRequest<TTopics, TTopic, TKind>,
  ) => Promise<void>;
  request: <TTopic extends WindowMessageRequestTopics<TTopics>>(
    request: Omit<SendWindowMessageRequest<TTopics, TTopic, "request">, "kind"> & { correlationId?: string },
  ) => Promise<WindowMessageEnvelope<TTopics, TTopic, "response">>;
  reply: <TTopic extends WindowMessageRequestTopics<TTopics>>(
    request: WindowMessageEnvelope<TTopics, TTopic, "request">,
    payload: WindowMessageResponsePayload<TTopics, TTopic>,
    topic?: TTopic,
  ) => Promise<void>;
}
