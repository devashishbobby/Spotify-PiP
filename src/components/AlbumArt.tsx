import { useState } from "react";
import { Icon } from "./Icon";
export function AlbumArt({
  url,
  demo,
  title,
}: {
  url: string | null;
  demo: boolean;
  title: string;
}) {
  const [failedUrl, setFailedUrl] = useState<string | null>(null);
  return (
    <div className={`album-art ${demo ? "album-art--demo" : ""}`}>
      {url && failedUrl !== url ? (
        <img
          src={url}
          alt={`Artwork for ${title}`}
          onError={() => setFailedUrl(url)}
        />
      ) : demo ? (
        <svg
          viewBox="0 0 96 96"
          role="img"
          aria-label="Original abstract dusk artwork"
        >
          <defs>
            <linearGradient id="dusk" x2=".8" y2="1">
              <stop stopColor="#c7ad96" />
              <stop offset=".45" stopColor="#967b72" />
              <stop offset="1" stopColor="#302830" />
            </linearGradient>
            <linearGradient id="light" x2="0" y2="1">
              <stop stopColor="#eee1ba" />
              <stop offset="1" stopColor="#c8b6a6" stopOpacity="0" />
            </linearGradient>
          </defs>
          <rect width="96" height="96" fill="url(#dusk)" />
          <circle cx="65" cy="30" r="19" fill="#e2ceac" opacity=".82" />
          <path d="M0 70 35 33l30 37 31-20v46H0z" fill="#624c50" />
          <path d="m0 83 42-30 54 36v7H0z" fill="#372e3b" />
          <path d="M46 0h8v96h-8z" fill="url(#light)" opacity=".4" />
          <path
            d="M0 76h96M0 81h96M0 86h96M0 91h96"
            stroke="#c4b6ac"
            opacity=".13"
          />
        </svg>
      ) : (
        <Icon name="music" />
      )}
    </div>
  );
}
