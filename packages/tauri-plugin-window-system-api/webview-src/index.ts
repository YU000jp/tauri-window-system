import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface WindowGeometry {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface WindowDescriptor {
  label: string;
  url: string;
  parent: string | null;
  title: string | null;
  geometry: WindowGeometry | null;
}

export type RestoreSkipReason = "missing-parent";

export interface RestoreSkippedWindow {
  label: string;
  parent: string | null;
  reason: RestoreSkipReason;
}

export interface RestoreWindowsResult {
  restored: WindowDescriptor[];
  alreadyAlive: string[];
  skipped: RestoreSkippedWindow[];
}

export const WINDOW_SYSTEM_REGISTRY_CHANGED_EVENT = "window-system:registry-changed";
export const WINDOW_SYSTEM_MESSAGE_EVENT = "window-system:message";

export type WindowRegistryChangeKind = "opened" | "closed" | "geometry-changed";

export interface WindowRegistryChangedEvent {
  kind: WindowRegistryChangeKind;
  label: string;
  windows: WindowDescriptor[];
}

export type WindowSystemErrorKind =
  | "invalid-label"
  | "window-already-exists"
  | "window-cannot-be-its-own-parent"
  | "parent-window-not-found"
  | "window-not-found"
  | "window-reservation-not-found"
  | "window-registry-lock-poisoned"
  | "window-state-store-lock-poisoned"
  | "unknown";

export interface WindowSystemError {
  kind: WindowSystemErrorKind;
  message: string;
  raw: string;
}

export interface OpenWindowRequest {
  label: string;
  url?: string;
  parent?: string;
  title?: string;
  geometry?: WindowGeometry;
}

export interface WindowMessageTopicDefinition<RequestPayload = unknown, ResponsePayload = never> {
  request: RequestPayload;
  response: ResponsePayload;
}

export type WindowMessageTopicMap = Record<string, WindowMessageTopicDefinition<unknown, unknown>>;
export type WindowMessageTopicName<TTopics extends WindowMessageTopicMap> = Extract<keyof TTopics, string>;
export type WindowMessageRequestTopics<TTopics extends WindowMessageTopicMap> = {
  [TTopic in WindowMessageTopicName<TTopics>]: WindowMessageResponsePayload<TTopics, TTopic> extends never
    ? never
    : TTopic;
}[WindowMessageTopicName<TTopics>];
export type WindowMessageRequestPayload<
  TTopics extends WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics>,
> = TTopics[TTopic] extends WindowMessageTopicDefinition<infer RequestPayload, unknown> ? RequestPayload : never;
export type WindowMessageResponsePayload<
  TTopics extends WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics>,
> = TTopics[TTopic] extends WindowMessageTopicDefinition<unknown, infer ResponsePayload> ? ResponsePayload : never;
export type WindowMessagePayload<
  TTopics extends WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind,
> = TKind extends "response"
  ? WindowMessageResponsePayload<TTopics, TTopic>
  : WindowMessageRequestPayload<TTopics, TTopic>;

export type WindowMessageScope = "direct" | "broadcast";
export type WindowMessageKind = "event" | "request" | "response";

export interface WindowMessageEnvelope<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
> {
  from: string;
  to: string | null;
  scope: WindowMessageScope;
  kind: TKind;
  topic: TTopic;
  correlationId: string | null;
  payload: WindowMessagePayload<TTopics, TTopic, TKind>;
  timestampMs: number;
}

export interface SendWindowMessageRequest<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
> {
  to: string;
  kind: TKind;
  topic: TTopic;
  correlationId?: string;
  payload: WindowMessagePayload<TTopics, TTopic, TKind>;
}

export interface BroadcastWindowMessageRequest<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
> {
  kind: TKind;
  topic: TTopic;
  correlationId?: string;
  payload: WindowMessagePayload<TTopics, TTopic, TKind>;
}

export interface WindowMessageDispatchResult<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
> {
  envelope: WindowMessageEnvelope<TTopics, TTopic, TKind>;
  deliveredTo: string[];
}

export async function openWindow(request: OpenWindowRequest): Promise<WindowDescriptor> {
  return invoke<WindowDescriptor>("plugin:window-system|open_window", { request });
}

export async function closeWindow(label: string): Promise<void> {
  await invoke("plugin:window-system|close_window", { label });
}

export async function listWindows(): Promise<WindowDescriptor[]> {
  return invoke<WindowDescriptor[]>("plugin:window-system|list_windows");
}

export async function restoreWindows(): Promise<RestoreWindowsResult> {
  return invoke<RestoreWindowsResult>("plugin:window-system|restore_windows");
}

export async function emitToWindow(
  label: string,
  event: string,
  payload: unknown,
): Promise<void> {
  await invoke("plugin:window-system|emit_to_window", { label, event, payload });
}

export async function sendWindowMessage<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
>(
  request: SendWindowMessageRequest<TTopics, TTopic, TKind>,
): Promise<WindowMessageDispatchResult<TTopics, TTopic, TKind>> {
  return invoke<WindowMessageDispatchResult<TTopics, TTopic, TKind>>("plugin:window-system|send_window_message", {
    request,
  });
}

export async function broadcastWindowMessage<
  TTopics extends WindowMessageTopicMap = WindowMessageTopicMap,
  TTopic extends WindowMessageTopicName<TTopics> = WindowMessageTopicName<TTopics>,
  TKind extends WindowMessageKind = WindowMessageKind,
>(
  request: BroadcastWindowMessageRequest<TTopics, TTopic, TKind>,
): Promise<WindowMessageDispatchResult<TTopics, TTopic, TKind>> {
  return invoke<WindowMessageDispatchResult<TTopics, TTopic, TKind>>("plugin:window-system|broadcast_window_message", {
    request,
  });
}

export async function listenRegistryChanges(
  handler: (event: WindowRegistryChangedEvent) => void,
): Promise<UnlistenFn> {
  return listen<WindowRegistryChangedEvent>(WINDOW_SYSTEM_REGISTRY_CHANGED_EVENT, (event) => {
    handler(event.payload);
  });
}

export async function listenWindowMessages<TTopics extends WindowMessageTopicMap = WindowMessageTopicMap>(
  handler: (event: WindowMessageEnvelope<TTopics>) => void,
): Promise<UnlistenFn> {
  return listen<WindowMessageEnvelope<TTopics>>(WINDOW_SYSTEM_MESSAGE_EVENT, (event) => {
    handler(event.payload);
  });
}

export function parseWindowSystemError(value: unknown): WindowSystemError {
  const raw = value instanceof Error ? value.message : String(value);
  const separatorIndex = raw.indexOf(": ");

  if (separatorIndex <= 0) {
    return {
      kind: "unknown",
      message: raw,
      raw,
    };
  }

  const kind = raw.slice(0, separatorIndex) as WindowSystemErrorKind;
  const message = raw.slice(separatorIndex + 2);

  return {
    kind: isWindowSystemErrorKind(kind) ? kind : "unknown",
    message,
    raw,
  };
}

function isWindowSystemErrorKind(value: string): value is WindowSystemErrorKind {
  return (
    value === "invalid-label" ||
    value === "window-already-exists" ||
    value === "window-cannot-be-its-own-parent" ||
    value === "parent-window-not-found" ||
    value === "window-not-found" ||
    value === "window-reservation-not-found" ||
    value === "window-registry-lock-poisoned" ||
    value === "window-state-store-lock-poisoned"
  );
}

export { createWindowBus } from "./windowBus";
