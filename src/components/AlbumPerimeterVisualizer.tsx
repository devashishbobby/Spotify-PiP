import type { CSSProperties } from 'react';
import type { MediaStatus } from '../types/media';
import { useAmbientTheme } from '../hooks/useAmbientTheme';
import '../styles/album-perimeter.css';

const sides = ['top', 'right', 'bottom', 'left'] as const;
const bars = Array.from({ length: 16 }, (_, index) => index);

/** Decorative simulated motion. GSMTC exposes no audio samples or beat timing. */
export function AlbumPerimeterVisualizer({ artwork, status, visible }: {
  artwork: string | null; status: MediaStatus; visible: boolean;
}) {
  const theme = useAmbientTheme(artwork, status);
  const active = (status === 'playing' || status === 'paused') && !!theme.backdrop;
  const state = active ? status : 'off';
  return (
    <div className="album-perimeter" aria-hidden="true" data-state={state}
      data-visible={visible} data-theme-id={theme.identity}
      style={{ '--perimeter-color': `hsl(${theme.palette.hues[0]} ${Math.round(theme.palette.saturation[0] * 100)}% 64%)` } as CSSProperties}>
      {sides.map((side) => (
        <div className="perimeter-edge" data-side={side} key={side}>
          {bars.map((index) => (
            <span className="perimeter-bar" key={index} style={{
              '--bar-position': `${5 + index * 6}%`,
              '--bar-delay': `${-((index * 7) % 17) * .13}s`,
              '--bar-duration': `${1.25 + (index % 5) * .17}s`,
              '--bar-peak': .55 + ((index * 11) % 9) * .055,
            } as CSSProperties}>
              <span className="perimeter-pulse" />
            </span>
          ))}
        </div>
      ))}
    </div>
  );
}
