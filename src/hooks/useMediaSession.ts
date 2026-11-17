import { useEffect, useRef, useState } from "react";
import type { MediaSnapshot, MediaStatus } from "../types/media";
import { useMockMediaSession } from "./useMockMediaSession";
import { mediaApi } from "../lib/mediaApi";
import { desktop } from "../lib/desktop";
import {
  displayPosition,
  emptySnapshot,
  errorMessage,
} from "../lib/mediaState";

export const demoMode =
  import.meta.env.DEV && import.meta.env.VITE_MOCK_MEDIA === "true";

function useNativeMediaSession(visible: boolean) {
  const [snapshot, setSnapshot] = useState(() => emptySnapshot("loading"));
  const [pending, setPending] = useState(false);
  const [seekPreview, setSeekPreview] = useState<number | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const [clock, setClock] = useState(() => performance.now());
  const pendingRef = useRef(false);
  const alive = useRef(true);
  const canonical = useRef(snapshot);
  const revision = useRef(-1);
  const anchor = useRef({ receivedAt: performance.now(), ageAtReceipt: 0 });
  const connected = useRef(false);
  const [connectionAttempt, reconnect] = useState(0);

  function accept(next: MediaSnapshot) {
    if (!alive.current || next.revision <= revision.current) return;
    revision.current = next.revision;
    canonical.current = next;
    anchor.current = {
      receivedAt: performance.now(),
      ageAtReceipt: Math.max(
        0,
        Math.min(30_000, Date.now() - next.snapshotAtUtcMs),
      ),
    };
    setClock(anchor.current.receivedAt);
    setSnapshot(next);
  }
  useEffect(() => {
    let disposed = false;
    let unlisten: (() => void) | undefined;
    alive.current = true;
    // Each StrictMode lifecycle has its own disposal guard; no late registration survives cleanup.
    async function connect() {
      try {
        if (!desktop)
          throw new Error("Open the desktop app to connect to Spotify.");
        const stop = await mediaApi.subscribe((next) => {
          if (!disposed) accept(next);
        });
        if (disposed) {
          stop();
          return;
        }
        unlisten = stop;
        connected.current = true;
        const initial = await mediaApi.getSnapshot();
        if (!disposed) accept(initial);
      } catch (error) {
        if (disposed) return;
        if (!connected.current) setActionError(errorMessage(error));
        if (revision.current < 0)
          setSnapshot({
            ...emptySnapshot("error"),
            errorMessage: errorMessage(error),
            errorCode: "connection_failed",
          });
      }
    }
    void connect();
    return () => {
      disposed = true;
      alive.current = false;
      connected.current = false;
      unlisten?.();
    };
  }, [connectionAttempt]);

  useEffect(() => {
    if (!visible || !desktop) return;
    void mediaApi
      .refresh()
      .then(accept)
      .catch((error: unknown) => {
        if (alive.current) setActionError(errorMessage(error));
      });
  }, [visible]);
  useEffect(() => {
    if (
      !visible ||
      snapshot.status !== "playing" ||
      snapshot.positionMs === null ||
      snapshot.durationMs === null
    )
      return;
    const timer = window.setInterval(() => setClock(performance.now()), 500);
    return () => window.clearInterval(timer);
  }, [visible, snapshot.status, snapshot.positionMs, snapshot.durationMs]);

  async function perform(
    name:
      | "media_play_pause"
      | "media_next"
      | "media_previous"
      | "media_seek"
      | "media_set_volume"
      | "media_set_muted",
    value?: number,
  ) {
    if (pendingRef.current) return;
    pendingRef.current = true;
    setPending(true);
    setActionError(null);
    if (name === "media_seek") setSeekPreview(value!);
    try {
      const result =
        name === "media_seek"
          ? await mediaApi.seek(value!, canonical.current.sessionGeneration)
          : name === "media_set_volume"
            ? await mediaApi.setVolume(
                value!,
                canonical.current.sessionGeneration,
              )
            : name === "media_set_muted"
              ? await mediaApi.setMuted(
                  value === 1,
                  canonical.current.sessionGeneration,
                )
              : await mediaApi.action(
                  name,
                  canonical.current.sessionGeneration,
                );
      if (!result.accepted)
        throw new Error("Spotify declined this playback request.");
      if (result.snapshot) accept(result.snapshot);
      // The backend refreshes after every request. No optimistic playback/track change here.
    } catch (error) {
      if (alive.current) setActionError(errorMessage(error));
    } finally {
      pendingRef.current = false;
      if (alive.current) {
        setPending(false);
        setSeekPreview(null);
      }
    }
  }
  async function refresh() {
    if (!connected.current && desktop) reconnect((attempt) => attempt + 1);
    try {
      accept(await mediaApi.refresh());
      if (alive.current) setActionError(null);
    } catch (error) {
      if (alive.current) setActionError(errorMessage(error));
    }
  }
  return {
    snapshot: {
      ...snapshot,
      positionMs: displayPosition(
        snapshot,
        anchor.current.receivedAt,
        clock,
        anchor.current.ageAtReceipt,
      ),
    },
    pending,
    seekPreview,
    actionError,
    clearActionError() {
      setActionError(null);
    },
    playPause() {
      void perform("media_play_pause");
    },
    next() {
      void perform("media_next");
    },
    previous() {
      void perform("media_previous");
    },
    skip(direction: number) {
      void perform(direction < 0 ? "media_previous" : "media_next");
    },
    refresh() {
      void refresh();
    },
    seek(positionMs: number) {
      if (canonical.current.canSeek && Number.isFinite(positionMs))
        void perform("media_seek", positionMs);
    },
    setVolume(level: number) {
      if (canonical.current.canSetVolume && Number.isFinite(level))
        void perform("media_set_volume", Math.max(0, Math.min(1, level)));
    },
    setMuted(muted: boolean) {
      if (canonical.current.canSetVolume)
        void perform("media_set_muted", muted ? 1 : 0);
    },
    setDemoState(_status: MediaStatus) {},
  };
}
export function useMediaSession(visible: boolean) {
  const useProvider = demoMode ? useMockMediaSession : useNativeMediaSession;
  return useProvider(visible);
}
