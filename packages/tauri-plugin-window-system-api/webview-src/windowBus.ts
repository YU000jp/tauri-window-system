import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  broadcastWindowMessage,
  listenWindowMessages,
  sendWindowMessage,
  type BroadcastWindowMessageRequest,
  type WindowMessageEnvelope,
  type WindowMessageKind,
  type WindowMessageRequestTopics,
  type WindowMessageResponsePayload,
  type WindowMessageTopicMap,
  type WindowMessageTopicName,
  type SendWindowMessageRequest,
} from "./index.js";

export interface WindowMessageFilter<TTopics extends WindowMessageTopicMap = WindowMessageTopicMap> {
  topic?: WindowMessageTopicName<TTopics>;
  kind?: WindowMessageKind | WindowMessageKind[];
  from?: string;
  to?: string;
}

export interface RequestWindowMessageOptions {
  timeoutMs?: number;
}

export interface WindowBus<TTopics extends WindowMessageTopicMap = WindowMessageTopicMap> {
  label: () => string;
  listen: (
    handler: (message: WindowMessageEnvelope<TTopics>) => void,
    filter?: WindowMessageFilter<TTopics>,
  ) => Promise<() => void>;
  send: <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
    request: SendWindowMessageRequest<TTopics, TTopic, TKind>,
  ) => Promise<void>;
  broadcast: <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
    request: BroadcastWindowMessageRequest<TTopics, TTopic, TKind>,
  ) => Promise<void>;
  request: <TTopic extends WindowMessageRequestTopics<TTopics>>(
    request: Omit<SendWindowMessageRequest<TTopics, TTopic, "request">, "kind"> & { correlationId?: string },
    options?: RequestWindowMessageOptions,
  ) => Promise<WindowMessageEnvelope<TTopics, TTopic, "response">>;
  reply: <TTopic extends WindowMessageRequestTopics<TTopics>>(
    request: WindowMessageEnvelope<TTopics, TTopic, "request">,
    payload: WindowMessageResponsePayload<TTopics, TTopic>,
    topic?: TTopic,
  ) => Promise<void>;
}

export function createWindowBus<TTopics extends WindowMessageTopicMap = WindowMessageTopicMap>(): WindowBus<TTopics> {
  const bus = {
    label: () => getCurrentWindow().label,
    listen: async (
      handler: (message: WindowMessageEnvelope<TTopics>) => void,
      filter?: WindowMessageFilter<TTopics>,
    ) => {
      return listenWindowMessages((message) => {
        const typedMessage = message as WindowMessageEnvelope<TTopics>;

        if (matchesWindowMessageFilter(typedMessage, filter)) {
          handler(typedMessage);
        }
      });
    },
    send: async <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
      request: SendWindowMessageRequest<TTopics, TTopic, TKind>,
    ) => {
      await sendWindowMessage(request);
    },
    broadcast: async <TTopic extends WindowMessageTopicName<TTopics>, TKind extends WindowMessageKind>(
      request: BroadcastWindowMessageRequest<TTopics, TTopic, TKind>,
    ) => {
      await broadcastWindowMessage(request);
    },
    request: async <TTopic extends WindowMessageRequestTopics<TTopics>>(
      request: Omit<SendWindowMessageRequest<TTopics, TTopic, "request">, "kind"> & { correlationId?: string },
      options?: RequestWindowMessageOptions,
    ): Promise<WindowMessageEnvelope<TTopics, TTopic, "response">> => {
      const correlationId = request.correlationId ?? globalThis.crypto.randomUUID();
      const timeoutMs = options?.timeoutMs ?? 5000;

      return await new Promise<WindowMessageEnvelope<TTopics, TTopic, "response">>((resolve, reject) => {
        let settled = false;
        let unlisten: (() => void) | null = null;
        const cleanup = () => {
          globalThis.clearTimeout(timeoutHandle);
          unlisten?.();
          unlisten = null;
        };
        const timeoutHandle = globalThis.setTimeout(() => {
          if (settled) {
            return;
          }

          settled = true;
          cleanup();
          reject(new Error(`window-system-message-timeout: ${request.to}`));
        }, timeoutMs);

        const finalize = (next: WindowMessageEnvelope<TTopics, TTopic, "response">) => {
          if (settled) {
            return;
          }

          settled = true;
          cleanup();
          resolve(next);
        };

        void (async () => {
          try {
            unlisten = await listenWindowMessages((message) => {
              if (
                message.kind === "response" &&
                message.correlationId === correlationId &&
                message.from === request.to
              ) {
                finalize(message as WindowMessageEnvelope<TTopics, TTopic, "response">);
              }
            });

            await sendWindowMessage({
              ...request,
              kind: "request",
              correlationId,
            });
          } catch (error) {
            if (settled) {
              return;
            }

            settled = true;
            cleanup();
            reject(error);
          }
        })();
      });
    },
    reply: async <TTopic extends WindowMessageRequestTopics<TTopics>>(
      request: WindowMessageEnvelope<TTopics, TTopic, "request">,
      payload: WindowMessageResponsePayload<TTopics, TTopic>,
      topic: TTopic = request.topic,
    ) => {
      if (!request.from) {
        throw new Error("window-system-message-invalid-request: missing sender");
      }

      await sendWindowMessage({
        to: request.from,
        kind: "response",
        topic,
        correlationId: request.correlationId ?? undefined,
        payload,
      });
    },
  };
  return bus as WindowBus<TTopics>;
}

function matchesWindowMessageFilter<TTopics extends WindowMessageTopicMap>(
  message: WindowMessageEnvelope<TTopics>,
  filter?: WindowMessageFilter<TTopics>,
) {
  if (!filter) {
    return true;
  }

  if (filter.topic && message.topic !== filter.topic) {
    return false;
  }

  if (filter.from && message.from !== filter.from) {
    return false;
  }

  if (filter.to && message.to !== filter.to) {
    return false;
  }

  if (!filter.kind) {
    return true;
  }

  return Array.isArray(filter.kind) ? filter.kind.includes(message.kind) : message.kind === filter.kind;
}
