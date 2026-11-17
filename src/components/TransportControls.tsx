import type { MediaSnapshot } from "../types/media";
import { Icon } from "./Icon";
import type { ReactNode } from "react";
export function TransportControls({
  snapshot,
  playPause,
  skip,
  pending = false,
  volumeControl,
}: {
  snapshot: MediaSnapshot;
  playPause: () => void;
  skip: (direction: number) => void;
  pending?: boolean;
  volumeControl?: ReactNode;
}) {
  const playing = snapshot.status === "playing";
  return (
    <div className="transport" aria-label="Playback controls">
      <button
        className="icon-button skip"
        title="Previous track"
        aria-label="Previous track"
        disabled={pending || !snapshot.canPrevious}
        onClick={() => skip(-1)}
      >
        <Icon name="previous" />
      </button>
      <button
        className="play-button"
        title={playing ? "Pause" : "Play"}
        aria-label={playing ? "Pause" : "Play"}
        disabled={pending || !snapshot.canPlayPause}
        onClick={playPause}
      >
        <Icon name={playing ? "pause" : "play"} />
      </button>
      <button
        className="icon-button skip"
        title="Next track"
        aria-label="Next track"
        disabled={pending || !snapshot.canNext}
        onClick={() => skip(1)}
      >
        <Icon name="next" />
      </button>
      {volumeControl}
    </div>
  );
}
