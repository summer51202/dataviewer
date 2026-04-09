import { convertFileSrc } from "@tauri-apps/api/core";

declare global {
  interface Window {
    __TAURI__?: unknown;
    __TAURI_INTERNALS__?: unknown;
  }
}

export function hasTauriRuntime() {
  return typeof window !== "undefined" && ("__TAURI__" in window || "__TAURI_INTERNALS__" in window);
}

export async function invokeOrFallback<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  fallback: T,
) {
  if (!hasTauriRuntime()) {
    return fallback;
  }

  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(command, args);
}

export function describeError(error: unknown) {
  if (typeof error === "string") {
    return error;
  }

  if (error instanceof Error) {
    return error.message;
  }

  if (typeof error === "object" && error !== null) {
    const record = error as Record<string, unknown>;
    if (typeof record.message === "string") {
      return record.message;
    }
    if (typeof record.error === "string") {
      return record.error;
    }
    try {
      return JSON.stringify(record);
    } catch {
      return "Unknown application error";
    }
  }

  return "Unknown application error";
}

export async function pickFolder() {
  if (!hasTauriRuntime()) {
    return null;
  }

  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({
    directory: true,
    multiple: false,
    title: "Select a folder",
  });

  return typeof result === "string" ? result : null;
}

export function toAssetUrl(filePath: string) {
  if (!hasTauriRuntime()) {
    return null;
  }

  return convertFileSrc(filePath);
}
