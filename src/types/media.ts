export type MediaStatus =
  | "loading"
  | "spotify_not_running"
  | "idle"
  | "playing"
  | "paused"
  | "disconnected"
  | "error";
export interface MediaSnapshot {
  source: "spotify" | "none";
  status: MediaStatus;
  title: string | null;
  artist: string | null;
  album: string | null;
  artworkDataUrl: string | null;
  positionMs: number | null;
  durationMs: number | null;
  seekStartMs: number | null;
  seekEndMs: number | null;
  lastTimelineUpdateUtcMs: number | null;
  snapshotAtUtcMs: number;
  playbackRate: number;
  canPlayPause: boolean;
  canNext: boolean;
  canPrevious: boolean;
  canSeek: boolean;
  volumeLevel: number | null;
  volumeMuted: boolean | null;
  canSetVolume: boolean;
  errorMessage: string | null;
  errorCode: string | null;
  revision: number;
  sessionGeneration: number;
}
