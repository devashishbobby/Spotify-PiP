import { useRef, useState, type CSSProperties } from "react";

export function formatTime(ms: number) {
  const seconds = Math.max(0, Math.floor(ms / 1000));
  return `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, "0")}`;
}
export function ProgressTrack({
  position,
  duration,
  canSeek = false,
  onSeek,
  seekStart = null,
  seekEnd = null,
  pending = false,
  seekPreview = null,
}: {
  position: number | null;
  duration: number | null;
  canSeek?: boolean;
  onSeek?: (positionMs: number) => void;
  seekStart?: number | null;
  seekEnd?: number | null;
  pending?: boolean;
  seekPreview?: number | null;
}) {
  const [draft, setDraft] = useState<number | null>(null);
  const dragging = useRef(false);
  const draftRef = useRef<number | null>(null);
  const displayedPosition = draft ?? seekPreview ?? position;
  const known = displayedPosition !== null && duration !== null && duration > 0;
  const percent = known
    ? Math.min(100, Math.max(0, (displayedPosition / duration) * 100))
    : 0;
  return (
    <div className="timeline">
      <span>{known ? formatTime(displayedPosition) : "—:—"}</span>
      {known && canSeek && onSeek ? (
        <input
          className="seek-slider"
          type="range"
          min={seekStart ?? 0}
          max={seekEnd ?? duration}
          step={1000}
          value={displayedPosition}
          disabled={pending}
          aria-label="Seek track"
          aria-valuetext={`${formatTime(displayedPosition)} of ${formatTime(duration)}`}
          title={seekPreview !== null ? "Seeking…" : "Seek track"}
          aria-busy={seekPreview !== null}
          style={{ "--progress": `${percent}%` } as CSSProperties}
          onPointerDown={() => {
            dragging.current = true;
            draftRef.current = position;
          }}
          onChange={(event) => {
            const value = Number(event.target.value);
            if (dragging.current) {
              draftRef.current = value;
              setDraft(value);
            } else onSeek(value);
          }}
          onPointerUp={() => {
            if (dragging.current && draftRef.current !== null)
              onSeek(draftRef.current);
            dragging.current = false;
            draftRef.current = null;
            setDraft(null);
          }}
          onPointerCancel={() => {
            dragging.current = false;
            draftRef.current = null;
            setDraft(null);
          }}
          onBlur={() => {
            dragging.current = false;
            draftRef.current = null;
            setDraft(null);
          }}
        />
      ) : (
        <div
          className="progress"
          role="progressbar"
          aria-label="Track progress"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={known ? Math.round(percent) : undefined}
          aria-valuetext={
            known
              ? `${formatTime(displayedPosition)} of ${formatTime(duration)}`
              : "Timeline unavailable"
          }
        >
          <div className="progress-fill" style={{ width: `${percent}%` }} />
        </div>
      )}
      <span>{known ? formatTime(duration) : "—:—"}</span>
    </div>
  );
}
