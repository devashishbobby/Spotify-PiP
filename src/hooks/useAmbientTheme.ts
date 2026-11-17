import { useEffect, useRef, useState } from 'react';
import type { MediaStatus } from '../types/media';
import { AmbientThemeRequest, neutralTheme, type AmbientTheme } from '../lib/ambientPalette';

export function useAmbientTheme(artwork: string | null, status: MediaStatus) {
  const [theme,setTheme]=useState<AmbientTheme>(neutralTheme);
  const request=useRef(new AmbientThemeRequest());
  const currentArtwork=status==='playing' || status==='paused' ? artwork : null;
  useEffect(()=>{
    let disposed=false;
    void request.current.request(currentArtwork).then((next)=>{ if(!disposed && next) setTheme(next); });
    return ()=>{ disposed=true; request.current.cancel(); };
  },[currentArtwork]);
  return currentArtwork ? theme : neutralTheme;
}
