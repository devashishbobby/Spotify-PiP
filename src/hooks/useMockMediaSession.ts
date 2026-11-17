import { useEffect, useState } from "react";
import type { MediaStatus } from "../types/media";
import { mockSnapshot, mockTracks } from "../mocks/playback";
import { emptySnapshot } from "../lib/mediaState";
const demoMode =
  import.meta.env.DEV && import.meta.env.VITE_MOCK_MEDIA === "true";
export function useMockMediaSession(visible: boolean) {
  const [snapshot, setSnapshot] = useState(() =>
    emptySnapshot(demoMode ? "loading" : "idle"),
  );
  const [track, setTrack] = useState(0);
  useEffect(() => {
    if (!demoMode) return;
    const timer = window.setTimeout(() => setSnapshot(mockSnapshot()), 650);
    return () => window.clearTimeout(timer);
  }, []);
  useEffect(() => {
    if (!demoMode || !visible || snapshot.status !== "playing") return;
    const timer = window.setInterval(
      () =>
        setSnapshot((previous) => ({
          ...previous,
          positionMs: Math.min(
            (previous.positionMs ?? 0) + 1000,
            previous.durationMs ?? 0,
          ),
          status:
            (previous.positionMs ?? 0) + 1000 >= (previous.durationMs ?? 0)
              ? "paused"
              : previous.status,
        })),
      1000,
    );
    return () => window.clearInterval(timer);
  }, [visible, snapshot.status]);
  return {
    snapshot,
    pending: false,
    seekPreview: null as number | null,
    actionError: null as string | null,
    clearActionError() {},
    refresh() {
      setSnapshot((previous) =>
        ["playing", "paused"].includes(previous.status)
          ? previous
          : mockSnapshot(track),
      );
    },
    seek(positionMs: number) {
      if (!demoMode || !Number.isFinite(positionMs)) return;
      setSnapshot((previous) =>
        previous.canSeek && previous.durationMs !== null
          ? {
              ...previous,
              positionMs: Math.max(
                0,
                Math.min(previous.durationMs, positionMs),
              ),
            }
          : previous,
      );
    },
    setVolume(level: number) {
      setSnapshot((previous) => ({
        ...previous,
        volumeLevel: Math.max(0, Math.min(1, level)),
      }));
    },
    setMuted(muted: boolean) {
      setSnapshot((previous) => ({ ...previous, volumeMuted: muted }));
    },
    setDemoState(status: MediaStatus) {
      if (demoMode) setSnapshot(mockSnapshot(track, status));
    },
    playPause() {
      if (demoMode)
        setSnapshot((previous) => ({
          ...previous,
          status: previous.status === "playing" ? "paused" : "playing",
        }));
    },
    skip(direction: number) {
      if (!demoMode) return;
      const next = (track + direction + mockTracks.length) % mockTracks.length;
      setTrack(next);
      setSnapshot(
        mockSnapshot(next, snapshot.status === "paused" ? "paused" : "playing"),
      );
    },
  };
}
