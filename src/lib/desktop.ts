import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
export const desktop = isTauri();
export interface DesktopStatus {
  alwaysOnTop: boolean;
  shortcut: string | null;
  shortcutError: string | null;
  visible: boolean;
}
export const browserStatus: DesktopStatus = {
  alwaysOnTop: true,
  shortcut: null,
  shortcutError:
    "Browser preview — native controls are available in the desktop app.",
  visible: true,
};
export async function command<T = void>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!desktop)
    throw new Error("Launch the desktop app to use window controls.");
  return invoke<T>(name, args);
}
// Dispose correctly even when an async subscription resolves after StrictMode cleanup.
export function subscribe<T>(event: string, handler: (value: T) => void) {
  let disposed = false;
  let unlisten: (() => void) | undefined;
  if (desktop)
    void listen<T>(event, ({ payload }) => handler(payload))
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((error: unknown) =>
        console.error(`Cannot subscribe to ${event}`, error),
      );
  return () => {
    disposed = true;
    unlisten?.();
  };
}
