export interface AmbientPalette {
  colors: readonly [string, string, string];
  hues: readonly [number, number, number];
  saturation: readonly [number, number, number];
  lightness: readonly [number, number, number];
  shade: number;
}
export interface AmbientTheme { identity: string; palette: AmbientPalette; backdrop: string | null }
export const neutralTheme: AmbientTheme = {
  identity: 'neutral', backdrop: null,
  palette: { colors: ['hsl(0 0% 10%)', 'hsl(0 0% 7%)', 'hsl(0 0% 6%)'], hues: [0,0,0], saturation: [0,0,0], lightness: [.1,.07,.06], shade: .72 },
};
const SIDE = 24;
const MAX_DATA_URL = 2_800_000;
const cache = new Map<string, AmbientTheme>(); // Two tiny downsampled backdrops; never original images.
const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));

function hsl(r: number, g: number, b: number): [number, number, number] {
  const high = Math.max(r,g,b), low = Math.min(r,g,b), delta = high-low, l = (high+low)/2;
  if (!delta) return [0,0,l];
  const s = delta/(1-Math.abs(2*l-1));
  const h = high === r ? ((g-b)/delta)%6 : high === g ? (b-r)/delta+2 : (r-g)/delta+4;
  return [(h*60+360)%360,s,l];
}
export function derivePalette(pixels: Uint8ClampedArray): AmbientPalette {
  const bins = new Map<number,{r:number;g:number;b:number;weight:number}>();
  let brightness = 0, opaque = 0;
  for (let i=0; i+3<Math.min(pixels.length,SIDE*SIDE*4); i+=4) {
    if (pixels[i+3] < 180) continue;
    const [r,g,b] = [pixels[i]/255,pixels[i+1]/255,pixels[i+2]/255];
    const [,,l] = hsl(r,g,b);
    brightness += .2126*r+.7152*g+.0722*b; opaque++;
    const [,s] = hsl(r,g,b);
    if (l < .055 || l > .87 || s < .10) continue;
    const key = ((pixels[i]>>5)<<6)|((pixels[i+1]>>5)<<3)|(pixels[i+2]>>5);
    const weight = .4+s;
    const bin = bins.get(key) ?? {r:0,g:0,b:0,weight:0};
    bin.r += r*weight; bin.g += g*weight; bin.b += b*weight; bin.weight += weight; bins.set(key,bin);
  }
  if (!bins.size) return { ...neutralTheme.palette, shade: opaque && brightness/opaque > .65 ? .84 : .72 };
  const ranked = [...bins.values()].sort((a,b)=>b.weight-a.weight).map((bin)=>({ hsl:hsl(bin.r/bin.weight,bin.g/bin.weight,bin.b/bin.weight), weight:bin.weight }));
  const chosen = [ranked[0]];
  for (const candidate of ranked.slice(1)) {
    if (candidate.weight < ranked[0].weight*.12) continue;
    if (chosen.every((item) => Math.min(Math.abs(item.hsl[0]-candidate.hsl[0]),360-Math.abs(item.hsl[0]-candidate.hsl[0])) >= 28)) chosen.push(candidate);
    if (chosen.length === 3) break;
  }
  while (chosen.length < 3) chosen.push(chosen[0]);
  const hues = chosen.map((item)=>Math.round(item.hsl[0])) as [number,number,number];
  const saturation = chosen.map((item)=>clamp(item.hsl[1]*.6,.12,.42)) as [number,number,number];
  const lightness = chosen.map((item,index)=>clamp(item.hsl[2]*.3,.10,.20)-(index===2?.02:0)) as [number,number,number];
  const colors = hues.map((h,index)=>`hsl(${h} ${Math.round(saturation[index]*100)}% ${Math.round(lightness[index]*100)}%)`) as [string,string,string];
  return { colors,hues,saturation,lightness,shade: opaque && brightness/opaque > .65 ? .84 : .60 };
}

export function artworkIdentity(artwork: string): string {
  let a=2166136261,b=5381;
  for(let i=0;i<artwork.length;i++) { a=Math.imul(a^artwork.charCodeAt(i),16777619); b=Math.imul(b,33)^artwork.charCodeAt(i); }
  return `${artwork.length}-${a>>>0}-${b>>>0}`;
}
export function validArtwork(artwork: string | null): artwork is string {
  return !!artwork && artwork.length <= MAX_DATA_URL && /^data:image\/(?:jpeg|png);base64,[a-zA-Z0-9+/=]+$/.test(artwork);
}
export async function extractAmbientTheme(artwork: string, signal: AbortSignal): Promise<AmbientTheme> {
  if (!validArtwork(artwork) || signal.aborted) return neutralTheme;
  const identity = artworkIdentity(artwork);
  const cached = cache.get(identity);
  if (cached) return cached;
  const image = new Image(); image.decoding = 'async';
  let timer: ReturnType<typeof setTimeout> | undefined;
  const release = () => { image.src = ''; };
  signal.addEventListener('abort', release, {once:true});
  try {
    image.src = artwork;
    await Promise.race([image.decode(), new Promise<never>((_,reject)=>{ timer=setTimeout(()=>reject(new Error('Artwork decode timed out')),3000); })]);
    if (signal.aborted || !image.naturalWidth || !image.naturalHeight || image.naturalWidth>4096 || image.naturalHeight>4096 || image.naturalWidth*image.naturalHeight>16_000_000) return neutralTheme;
    const canvas = document.createElement('canvas'); canvas.width=SIDE; canvas.height=SIDE;
    const context = canvas.getContext('2d',{willReadFrequently:true});
    if (!context) return neutralTheme;
    context.drawImage(image,0,0,SIDE,SIDE);
    const palette = derivePalette(context.getImageData(0,0,SIDE,SIDE).data);
    const theme = { identity, palette, backdrop: canvas.toDataURL('image/png') };
    canvas.width=0; canvas.height=0;
    if (signal.aborted) return neutralTheme;
    cache.set(identity,theme);
    while(cache.size>2) cache.delete(cache.keys().next().value!);
    return theme;
  } catch { return neutralTheme; }
  finally { if(timer) clearTimeout(timer); signal.removeEventListener('abort',release); release(); }
}

export class AmbientThemeRequest {
  private revision=0;
  private controller: AbortController | null=null;
  constructor(private decode=extractAmbientTheme) {}
  async request(artwork: string | null): Promise<AmbientTheme | undefined> {
    this.controller?.abort(); const revision=++this.revision;
    const controller=new AbortController(); this.controller=controller;
    const theme=validArtwork(artwork) ? await this.decode(artwork,controller.signal).catch(()=>neutralTheme) : neutralTheme;
    return !controller.signal.aborted && revision===this.revision ? theme : undefined;
  }
  cancel() { this.controller?.abort(); this.revision++; }
}
