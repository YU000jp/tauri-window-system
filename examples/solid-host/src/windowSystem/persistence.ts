import type { WindowDescriptor } from "tauri-plugin-window-system-api"
import { scheduleIdleWork } from "../bootTelemetry"
import type { RegistrySnapshotCache } from "./types"

const REGISTRY_CACHE_KEY = "solid-host:registry-snapshot:v1";

let queuedRegistrySnapshot: WindowDescriptor[] | null = null;
let cacheWriteQueued = false;

export function queueRegistrySnapshotCache(rows: WindowDescriptor[]) {
  if (typeof sessionStorage === "undefined" || typeof localStorage === "undefined") {
    return;
  }

  queuedRegistrySnapshot = rows;
  if (cacheWriteQueued) {
    return;
  }

  cacheWriteQueued = true;
  const flush = () => {
    cacheWriteQueued = false;
    const snapshot = queuedRegistrySnapshot;
    if (!snapshot) {
      return;
    }

    const payload: RegistrySnapshotCache = {
      version: 1,
      savedAt: new Date().toISOString(),
      rows: snapshot,
    };

    const serialized = JSON.stringify(payload);
    tryWriteCache(sessionStorage, serialized);
    tryWriteCache(localStorage, serialized);
  };

  scheduleIdleWork(flush);
}

export function readRegistrySnapshotCache(): RegistrySnapshotCache | null {
  const sessionCache = readRegistrySnapshotCacheFrom(sessionStorage);
  if (sessionCache) {
    return sessionCache;
  }

  return readRegistrySnapshotCacheFrom(localStorage);
}

function tryWriteCache(storage: Storage, serialized: string) {
  try {
    storage.setItem(REGISTRY_CACHE_KEY, serialized);
  } catch {
    // Best-effort cache only; startup must not fail because storage is unavailable.
  }
}

function readRegistrySnapshotCacheFrom(storage: Storage): RegistrySnapshotCache | null {
  try {
    const raw = storage.getItem(REGISTRY_CACHE_KEY);
    if (!raw) {
      return null;
    }

    const parsed = JSON.parse(raw) as Partial<RegistrySnapshotCache>;
    if (parsed?.version !== 1 || !Array.isArray(parsed.rows)) {
      return null;
    }

    return {
      version: 1,
      savedAt: typeof parsed.savedAt === "string" ? parsed.savedAt : new Date(0).toISOString(),
      rows: parsed.rows as WindowDescriptor[],
    };
  } catch {
    return null;
  }
}
