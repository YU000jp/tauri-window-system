import { invoke } from "@tauri-apps/api/core";

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

export interface OpenWindowRequest {
  label: string;
  url?: string;
  parent?: string;
  title?: string;
  geometry?: WindowGeometry;
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

export async function emitToWindow(
  label: string,
  event: string,
  payload: unknown,
): Promise<void> {
  await invoke("plugin:window-system|emit_to_window", { label, event, payload });
}

