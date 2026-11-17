import { emptySnapshot } from "../lib/mediaState";
import type { MediaSnapshot, MediaStatus } from "../types/media";
// Fictional metadata; no Spotify or native transport calls.
export const mockTracks = [
  {
    title: "Soft focus",
    artist: "The Quiet Hours",
    album: "After the city",
    durationMs: 222_000,
  },
  {
    title: "Somewhere, slowly",
    artist: "Lowlight Assembly",
    album: "Open windows",
    durationMs: 196_000,
  },
  {
    title: "A little less gravity",
    artist: "Sunday Satellites",
    album: "Blue hour",
    durationMs: 248_000,
  },
];
export function mockSnapshot(
  index = 0,
  status: MediaStatus = "playing",
): MediaSnapshot {
  if (status !== "playing" && status !== "paused") return emptySnapshot(status);
  return {
    ...emptySnapshot(status),
    ...mockTracks[index],
    positionMs: 86_000,
    canPlayPause: true,
    canNext: true,
    canPrevious: true,
    canSeek: true,
    volumeLevel: 0.75,
    volumeMuted: false,
    canSetVolume: true,
  };
}
