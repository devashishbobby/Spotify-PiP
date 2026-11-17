import { useEffect, useState, type CSSProperties } from 'react';
import type { MediaStatus } from '../types/media';
import { useAmbientTheme } from '../hooks/useAmbientTheme';
import { neutralTheme, type AmbientTheme } from '../lib/ambientPalette';
import '../styles/ambient-theme.css';

export function AmbientBackdrop({ artwork, status, visible }: { artwork: string | null; status: MediaStatus; visible: boolean }) {
  const theme=useAmbientTheme(artwork,status);
  const [layers,setLayers]=useState<AmbientTheme[]>([neutralTheme]);
  useEffect(()=>{
    setLayers((previous)=>previous[previous.length-1]?.identity===theme.identity ? previous : [previous[previous.length-1] ?? neutralTheme,theme]);
    const timer=window.setTimeout(()=>setLayers([theme]),620);
    return ()=>window.clearTimeout(timer);
  },[theme]);
  return <div className="ambient-backdrop" aria-hidden="true" data-ambient-id={theme.identity} data-visible={visible} data-motion={status==='playing' && visible && theme.backdrop ? 'running' : 'stopped'}>
    {layers.map((layer,index)=><div key={layer.identity}
      className={`ambient-scene ${layers.length>1 ? index===layers.length-1 ? 'ambient-scene--incoming' : 'ambient-scene--outgoing' : ''}`}
      data-artwork={!!layer.backdrop}
      data-theme-id={layer.identity}
      style={{ '--ambient-a':layer.palette.colors[0], '--ambient-b':layer.palette.colors[1], '--ambient-c':layer.palette.colors[2], '--ambient-shade':layer.palette.shade,
        '--ambient-light-a':`hsl(${layer.palette.hues[0]} ${Math.round(layer.palette.saturation[0]*100)}% 58%)`,
        '--ambient-light-b':`hsl(${layer.palette.hues[1]} ${Math.round(layer.palette.saturation[1]*100)}% 48%)` } as CSSProperties}>
      {layer.backdrop && <div className="ambient-artwork" style={{ backgroundImage:`url("${layer.backdrop}")` }} />}
      <div className="ambient-tint" />
      <div className="ambient-visualizer">
        <div className="ambient-glow ambient-glow--one" /><div className="ambient-glow ambient-glow--two" />
        <svg className="ambient-ribbon" viewBox="0 0 360 208" preserveAspectRatio="none"><path d="M-30 144C38 95 81 177 150 142S251 96 390 141" /><path d="M-30 158C34 135 100 174 180 150S300 133 390 151" /></svg>
      </div>
    </div>)}
    <div className="ambient-glass-edge" />
  </div>;
}
