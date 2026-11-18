import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { demoMode, useMediaSession } from "../hooks/useMediaSession";
import {
  browserStatus,
  command,
  desktop,
  subscribe,
  type DesktopStatus,
} from "../lib/desktop";
import type { MediaStatus } from "../types/media";
import { AlbumArt } from "./AlbumArt";
import { EmptyState } from "./EmptyState";
import { Icon } from "./Icon";
import { ProgressTrack } from "./ProgressTrack";
import { TransportControls } from "./TransportControls";
import { VolumeControl } from "./VolumeControl";
import { StartupSetting } from "./StartupSetting";
import "../styles/startup-settings.css";
import { AmbientBackdrop } from './AmbientBackdrop';
import { AlbumPerimeterVisualizer } from './AlbumPerimeterVisualizer';
import { useWindowMotion } from '../hooks/useWindowMotion';

export function PlayerWindow() {
  const motionRef = useWindowMotion();
  const [native, setNative] = useState<DesktopStatus>({
    ...browserStatus,
    shortcutError: desktop ? null : browserStatus.shortcutError,
    visible: !desktop,
  });
  const [settings, setSettings] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const settingsButton = useRef<HTMLButtonElement>(null);
  const backButton = useRef<HTMLButtonElement>(null);
  const media = useMediaSession(native.visible);
  const active = ["playing", "paused"].includes(media.snapshot.status);

  async function run(name: string, args?: Record<string, unknown>) {
    try {
      await command(name, args);
      setError(null);
    } catch (reason) {
      setError(String(reason));
    }
  }
  useEffect(() => {
    if (desktop)
      void command<DesktopStatus>("get_desktop_status")
        .then((status) => {
          setNative(status);
          if (status.shortcutError) setSettings(true);
        })
        .catch((reason: unknown) => setError(String(reason)));
    const cleanups = [
      subscribe<DesktopStatus>("desktop://status", setNative),
      subscribe<boolean>("desktop://settings", setSettings),
      subscribe<string>("desktop://error", setError),
    ];
    return () => cleanups.forEach((cleanup) => cleanup());
  }, []);
  useEffect(() => {
    if (settings) backButton.current?.focus();
  }, [settings]);
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      if (settings) {
        setSettings(false);
        settingsButton.current?.focus();
      } else if (desktop) void run("hide_player");
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [settings]);
  function drag(event: React.PointerEvent) {
    if (
      !desktop ||
      event.button !== 0 ||
      (event.target as HTMLElement).closest("button")
    )
      return;
    void getCurrentWindow()
      .startDragging()
      .catch((reason: unknown) => setError(String(reason)));
  }
  return (
    <main ref={motionRef} className="player" aria-label="SpotifyPiP mini player">
      <AmbientBackdrop artwork={media.snapshot.artworkDataUrl} status={media.snapshot.status} visible={native.visible} />
      <header className="player-header" onPointerDown={drag}>
        <div className="header-label">
          <span>
            {settings
              ? "PLAYER SETTINGS"
              : media.snapshot.status === "paused"
                ? "PAUSED"
                : active
                  ? "NOW PLAYING"
                  : "SPOTIFY PIP"}
          </span>
        </div>
        <div className="window-actions">
          <button
            ref={settingsButton}
            className={`icon-button header-button ${settings ? "selected" : ""}`}
            aria-label="Player settings"
            title="Player settings"
            onClick={() => setSettings(!settings)}
          >
            <Icon name="settings" />
          </button>
          <button
            className="icon-button header-button"
            aria-label="Hide player"
            title="Hide player"
            disabled={!desktop}
            onClick={() => void run("hide_player")}
          >
            <Icon name="close" />
          </button>
        </div>
      </header>
      {settings ? (
        <section className="settings settings--startup" aria-label="Player settings">
          <div className="settings-heading">
            <button
              ref={backButton}
              className="icon-button"
              aria-label="Back to player"
              onClick={() => {
                setSettings(false);
                settingsButton.current?.focus();
              }}
            >
              <Icon name="back" />
            </button>
            <h1>Make yourself at home.</h1>
          </div>
          <div className="setting-row">
            <label htmlFor="always-on-top">Always on top</label>
            <input
              id="always-on-top"
              type="checkbox"
              role="switch"
              checked={native.alwaysOnTop}
              disabled={!desktop}
              onChange={(event) =>
                void run("set_always_on_top", { enabled: event.target.checked })
              }
            />
          </div>
          <StartupSetting />
          <div className="setting-row">
            <span>Show / hide</span>
            <kbd>
              {native.shortcut ?? (desktop ? "Unavailable" : "Desktop only")}
            </kbd>
          </div>
          <div className="settings-bottom">
            <button
              className="text-button"
              disabled={!desktop}
              onClick={() => void run("reset_window_position")}
            >
              Reset position
            </button>
            {demoMode && (
              <select
                aria-label="Demo playback state"
                value={media.snapshot.status}
                onChange={(event) =>
                  media.setDemoState(event.target.value as MediaStatus)
                }
              >
                <option value="playing">Demo: playing</option>
                <option value="paused">Demo: paused</option>
                <option value="loading">Demo: loading</option>
                <option value="spotify_not_running">Demo: empty</option>
                <option value="idle">Demo: waiting</option>
                <option value="disconnected">Demo: disconnected</option>
                <option value="error">Demo: error</option>
              </select>
            )}
          </div>
          {native.shortcutError && (
            <p className="shortcut-error">{native.shortcutError}</p>
          )}
        </section>
      ) : (
        <>
          <section
            className={`player-body ${active ? "" : "player-body--empty"}`}
          >
            <div className="album-visualizer-anchor">
            <AlbumArt
              key={`art-${media.snapshot.title ?? "empty"}`}
              url={media.snapshot.artworkDataUrl}
              demo={demoMode && active}
              title={media.snapshot.title ?? ""}
            />
            <AlbumPerimeterVisualizer artwork={media.snapshot.artworkDataUrl} status={media.snapshot.status} visible={native.visible} />
            </div>
            {active ? (
              <div
                className="track-content"
                key={`track-${media.snapshot.title}`}
              >
                <div className="track-copy">
                  <h1 title={media.snapshot.title ?? ""}>
                    {media.snapshot.title}
                  </h1>
                  <p title={media.snapshot.artist ?? ""}>
                    {media.snapshot.artist}
                  </p>
                </div>
                <TransportControls
                  snapshot={media.snapshot}
                  playPause={media.playPause}
                  skip={media.skip}
                  pending={media.pending}
                  volumeControl={
                    <VolumeControl
                      level={media.snapshot.volumeLevel}
                      muted={media.snapshot.volumeMuted}
                      available={media.snapshot.canSetVolume}
                      pending={media.pending}
                      setVolume={media.setVolume}
                      setMuted={media.setMuted}
                      refresh={media.refresh}
                    />
                  }
                />
              </div>
            ) : (
              <EmptyState
                status={media.snapshot.status}
                message={media.snapshot.errorMessage}
                demo={demoMode}
                retry={
                  demoMode ? () => media.setDemoState("playing") : media.refresh
                }
              />
            )}
          </section>
          <ProgressTrack
            key={`timeline-${media.snapshot.sessionGeneration}-${media.snapshot.title}`}
            position={media.snapshot.positionMs}
            duration={media.snapshot.durationMs}
            canSeek={media.snapshot.canSeek}
            seekStart={media.snapshot.seekStartMs}
            seekEnd={media.snapshot.seekEndMs}
            pending={media.pending}
            onSeek={media.seek}
            seekPreview={media.seekPreview}
          />
        </>
      )}
      <footer className="player-footer">
        <span className="mode-label">
          {demoMode ? (
            <>
              <span className="demo-dot" /> DEMO PREVIEW
            </>
          ) : (
            null
          )}
        </span>
        <span
          className="footer-hint"
          role="status"
          title={media.actionError ?? undefined}
        >
          {media.actionError ? (
            <button className="footer-retry" onClick={media.refresh}>
              Playback request failed · Retry
            </button>
          ) : demoMode ? (
            "Local playback only"
          ) : media.pending ? (
            "Updating playback…"
          ) : media.snapshot.source === "spotify" ? (
            "Spotify desktop"
          ) : (
            "Waiting for Spotify"
          )}
        </span>
      </footer>
      {error && (
        <div className="notice" role="alert">
          <span>{error}</span>
          <button className="text-button" onClick={() => setError(null)}>
            Dismiss
          </button>
        </div>
      )}
    </main>
  );
}
