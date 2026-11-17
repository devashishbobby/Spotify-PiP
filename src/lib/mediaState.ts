import type { MediaSnapshot, MediaStatus } from "../types/media";

export function emptySnapshot(status: MediaStatus): MediaSnapshot {
  return {
    source: "none",
    status,
    title: null,
    artist: null,
    album: null,
    artworkDataUrl: null,
    positionMs: null,
    durationMs: null,
    seekStartMs: null,
    seekEndMs: null,
    lastTimelineUpdateUtcMs: null,
    snapshotAtUtcMs: Date.now(),
    playbackRate: 1,
    canPlayPause: false,
    canNext: false,
    canPrevious: false,
    canSeek: false,
    volumeLevel: null,
    volumeMuted: null,
    canSetVolume: false,
    errorMessage: null,
    errorCode: null,
    revision: 0,
    sessionGeneration: 0,
  };
}
export function errorMessage(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof error.message === "string"
  )
    return error.message;
  return typeof error === "string"
    ? error
    : "Windows media service is unavailable. Please retry.";
}
// Display-only interpolation uses monotonic elapsed time, bounded by the known native timeline.
export function displayPosition(
  snapshot: MediaSnapshot,
  receivedAt: number,
  now: number,
  ageAtReceipt: number,
): number | null {
  if (
    snapshot.positionMs === null ||
    snapshot.durationMs === null ||
    snapshot.durationMs <= 0
  )
    return null;
  const elapsed =
    snapshot.status === "playing"
      ? Math.max(0, now - receivedAt) + ageAtReceipt
      : 0;
  const rate =
    Number.isFinite(snapshot.playbackRate) && snapshot.playbackRate > 0
      ? snapshot.playbackRate
      : 0;
  return Math.max(
    0,
    Math.min(snapshot.durationMs, snapshot.positionMs + elapsed * rate),
  );
}
