const BOOT_PREFIX = "solid-host";

function queueTask(task: () => void) {
  if (typeof queueMicrotask === "function") {
    queueMicrotask(task);
    return;
  }

  Promise.resolve().then(task);
}

export function scheduleAfterFirstPaint(task: () => void) {
  if (typeof requestAnimationFrame === "function") {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        queueTask(task);
      });
    });
    return;
  }

  setTimeout(task, 0);
}

export function scheduleIdleWork(task: () => void) {
  if (typeof requestIdleCallback === "function") {
    requestIdleCallback(
      () => {
        queueTask(task);
      },
      { timeout: 800 },
    );
    return;
  }

  scheduleAfterFirstPaint(task);
}

export function markBoot(name: string) {
  if (typeof performance === "undefined" || typeof performance.mark !== "function") {
    return;
  }

  performance.mark(`${BOOT_PREFIX}:${name}`);

  if (import.meta.env.DEV) {
    console.debug(`[boot] ${name}`);
  }
}

export function measureBoot(name: string, start: string, end: string) {
  if (typeof performance === "undefined" || typeof performance.measure !== "function") {
    return;
  }

  try {
    performance.measure(`${BOOT_PREFIX}:${name}`, `${BOOT_PREFIX}:${start}`, `${BOOT_PREFIX}:${end}`);
  } catch {
    return;
  }

  if (!import.meta.env.DEV) {
    return;
  }

  const entries = performance.getEntriesByName(`${BOOT_PREFIX}:${name}`, "measure");
  const latest = entries[entries.length - 1];
  if (latest) {
    console.debug(`[boot] ${name}: ${latest.duration.toFixed(1)}ms`);
  }
}
