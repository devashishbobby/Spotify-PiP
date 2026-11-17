import { useEffect, useRef, useState, type CSSProperties } from "react";
import { Icon } from "./Icon";

export function VolumeControl({
  level,
  muted,
  available,
  pending,
  setVolume,
  setMuted,
  refresh,
}: {
  level: number | null;
  muted: boolean | null;
  available: boolean;
  pending: boolean;
  setVolume: (level: number) => void;
  setMuted: (muted: boolean) => void;
  refresh: () => void;
}) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState<number | null>(null);
  const [submitted, setSubmitted] = useState<number | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const slider = useRef<HTMLInputElement>(null);
  const dragging = useRef(false);
  const value = useRef<number | null>(null);
  const shown = draft ?? (pending ? submitted : null) ?? level ?? 0;

  useEffect(() => {
    if (!pending) setSubmitted(null);
  }, [pending]);
  useEffect(() => {
    if (!open) return;
    slider.current?.focus();
    const dismiss = (event: PointerEvent) => {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", dismiss);
    return () => document.removeEventListener("pointerdown", dismiss);
  }, [open]);

  function commit(next: number) {
    setSubmitted(next);
    setVolume(next);
  }
  return (
    <div
      ref={root}
      className="volume-control"
      onKeyDown={(event) => {
        if (event.key === "Escape" && open) {
          event.preventDefault();
          event.stopPropagation();
          setOpen(false);
          trigger.current?.focus();
        }
      }}
    >
      <button
        ref={trigger}
        className={`icon-button skip volume-trigger ${open ? "selected" : ""}`}
        aria-label="Spotify volume"
        aria-expanded={open}
        aria-controls="spotify-volume-popup"
        title={
          available
            ? "Spotify app volume"
            : "Spotify app volume unavailable — start playback and retry"
        }
        disabled={!available || level === null}
        onClick={() => {
          if (!open) refresh();
          setOpen(!open);
        }}
      >
        <Icon name={muted || level === 0 ? "muted" : "volume"} />
      </button>
      {open && (
        <section
          id="spotify-volume-popup"
          className="volume-popup"
          role="group"
          aria-label="Spotify app volume"
        >
          <div className="volume-heading">
            <span>SPOTIFY VOLUME</span>
            <span aria-live="polite">{Math.round(shown * 100)}%</span>
          </div>
          <div className="volume-row">
            <button
              className="icon-button volume-mute"
              title={muted ? "Unmute Spotify" : "Mute Spotify"}
              aria-label={muted ? "Unmute Spotify" : "Mute Spotify"}
              disabled={pending || !available}
              onClick={() => setMuted(!muted)}
            >
              <Icon name={muted ? "muted" : "volume"} />
            </button>
            <input
              ref={slider}
              className="seek-slider volume-slider"
              type="range"
              min={0}
              max={100}
              step={1}
              aria-label="Spotify volume level"
              aria-valuetext={`${Math.round(shown * 100)} percent`}
              value={Math.round(shown * 100)}
              disabled={pending || !available}
              style={{ "--progress": `${shown * 100}%` } as CSSProperties}
              onPointerDown={() => {
                dragging.current = true;
                value.current = shown;
              }}
              onChange={(event) => {
                const next = Number(event.target.value) / 100;
                if (dragging.current) {
                  value.current = next;
                  setDraft(next);
                } else commit(next);
              }}
              onPointerUp={() => {
                if (dragging.current && value.current !== null)
                  commit(value.current);
                dragging.current = false;
                value.current = null;
                setDraft(null);
              }}
              onPointerCancel={() => {
                dragging.current = false;
                value.current = null;
                setDraft(null);
              }}
              onBlur={() => {
                dragging.current = false;
                value.current = null;
                setDraft(null);
              }}
            />
          </div>
          <span className="volume-caption">Spotify only</span>
        </section>
      )}
    </div>
  );
}
