import { listen } from "@tauri-apps/api/event";
import { command } from "./desktop";
import type { MediaSnapshot } from "../types/media";

export interface CommandResult {
  accepted: boolean;
  snapshot?: MediaSnapshot | null;
}
export const mediaApi = {
  getSnapshot: () => command<MediaSnapshot>("get_media_snapshot"),
  refresh: () => command<MediaSnapshot>("media_refresh"),
  subscribe: (callback: (snapshot: MediaSnapshot) => void) =>
    listen<MediaSnapshot>("media://snapshot-updated", ({ payload }) =>
      callback(payload),
    ),
  action: (
    name: "media_play_pause" | "media_next" | "media_previous",
    expectedSession: number,
  ) => command<CommandResult>(name, { expectedSession }),
  seek: (positionMs: number, expectedSession: number) =>
    command<CommandResult>("media_seek", {
      positionMs: Math.round(positionMs),
      expectedSession,
    }),
  setVolume: (level: number, expectedSession: number) =>
    command<CommandResult>("media_set_volume", { level, expectedSession }),
  setMuted: (muted: boolean, expectedSession: number) =>
    command<CommandResult>("media_set_muted", { muted, expectedSession }),
};
