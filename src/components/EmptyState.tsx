import type { MediaStatus } from "../types/media";
import { Icon } from "./Icon";
const copy: Record<string, [string, string]> = {
  spotify_not_running: [
    "Open Spotify to begin",
    "Your music, a little closer.",
  ],
  idle: ["Waiting for playback", "Pick a song in Spotify to get started."],
  disconnected: [
    "Spotify disconnected",
    "Your music will be here when you reconnect.",
  ],
  error: ["A brief interruption", "Windows media service is unavailable."],
};
export function EmptyState({
  status,
  retry,
  message,
  demo = false,
}: {
  status: MediaStatus;
  retry?: () => void;
  message?: string | null;
  demo?: boolean;
}) {
  if (status === "loading")
    return (
      <div className="empty-content loading" role="status">
        <div className="skeleton skeleton-title" />
        <div className="skeleton skeleton-artist" />
        <span>Finding the rhythm…</span>
      </div>
    );
  const [title, description] = copy[status] ?? copy.idle;
  return (
    <div className="empty-content" role="status">
      <h1>{title}</h1>
      <p title={message ?? description}>{message ?? description}</p>
      {retry &&
        ["disconnected", "error", "spotify_not_running"].includes(status) && (
          <button className="retry-button" onClick={retry}>
            <Icon name="retry" /> {demo ? "Retry demo" : "Retry"}
          </button>
        )}
    </div>
  );
}
