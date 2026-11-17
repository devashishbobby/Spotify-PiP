import type { ReactNode } from "react";
export type IconName =
  | "play"
  | "pause"
  | "previous"
  | "next"
  | "close"
  | "settings"
  | "music"
  | "retry"
  | "back"
  | "volume"
  | "muted";
const paths: Record<IconName, ReactNode> = {
  play: <path d="m9 5 11 7-11 7z" fill="currentColor" stroke="none" />,
  pause: (
    <path d="M8 5h3v14H8zM15 5h3v14h-3z" fill="currentColor" stroke="none" />
  ),
  previous: (
    <>
      <path d="M6 5v14" />
      <path d="m18 5-10 7 10 7z" fill="currentColor" stroke="none" />
    </>
  ),
  next: (
    <>
      <path d="M18 5v14" />
      <path d="m6 5 10 7-10 7z" fill="currentColor" stroke="none" />
    </>
  ),
  close: <path d="m7 7 10 10M17 7 7 17" />,
  settings: (
    <>
      <path d="M4 7h16M4 17h16" />
      <circle cx="9" cy="7" r="2" fill="currentColor" />
      <circle cx="15" cy="17" r="2" fill="currentColor" />
    </>
  ),
  music: (
    <>
      <path d="M10 17V6l9-2v11M10 9l9-2" />
      <ellipse cx="7" cy="17" rx="3" ry="2" />
      <ellipse cx="16" cy="15" rx="3" ry="2" />
    </>
  ),
  retry: (
    <>
      <path d="M20 7v5h-5M19 12a7 7 0 1 0-2 5M20 7l-3-3" />
    </>
  ),
  back: <path d="m14 6-6 6 6 6" />,
  volume: (
    <>
      <path d="M11 5 6 9H3v6h3l5 4zM15 8a6 6 0 0 1 0 8M18 5a10 10 0 0 1 0 14" />
    </>
  ),
  muted: (
    <>
      <path d="M11 5 6 9H3v6h3l5 4zM16 9l6 6M22 9l-6 6" />
    </>
  ),
};
export function Icon({ name }: { name: IconName }) {
  return (
    <svg
      aria-hidden="true"
      width="20"
      height="20"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.6"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      {paths[name]}
    </svg>
  );
}
