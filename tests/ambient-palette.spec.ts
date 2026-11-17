import { expect, test } from '@playwright/test';

test('palette derivation is bounded, color-adaptive and neutral for unusable pixels',async({page})=>{
  await page.goto('/');
  const results=await page.evaluate(async()=>{
    const {derivePalette}=await import('/src/lib/ambientPalette.ts');
    const sample=(r:number,g:number,b:number,a=255)=>new Uint8ClampedArray(Array.from({length:576},()=>[r,g,b,a]).flat());
    return {red:derivePalette(sample(220,25,35)),blue:derivePalette(sample(25,65,220)),gold:derivePalette(sample(225,160,30)),white:derivePalette(sample(255,255,255)),black:derivePalette(sample(0,0,0)),transparent:derivePalette(sample(220,10,10,0)),invalid:derivePalette(new Uint8ClampedArray())};
  });
  expect(results.red.hues[0]).toBeGreaterThan(330);
  expect(results.blue.hues[0]).toBeGreaterThan(200); expect(results.blue.hues[0]).toBeLessThan(260);
  expect(results.gold.hues[0]).toBeGreaterThan(25); expect(results.gold.hues[0]).toBeLessThan(60);
  for(const palette of Object.values(results)) {
    for(const l of palette.lightness) { expect(l).toBeGreaterThanOrEqual(.06); expect(l).toBeLessThanOrEqual(.20); }
    for(const s of palette.saturation) { expect(s).toBeGreaterThanOrEqual(0); expect(s).toBeLessThanOrEqual(.42); }
  }
  expect(results.white.shade).toBe(.84); expect(results.black.saturation).toEqual([0,0,0]);
  expect(results.transparent).toEqual(results.invalid);
});
test('superseded decodes cannot publish a stale theme, and invalid inputs go neutral',async({page})=>{
  await page.goto('/');
  const result=await page.evaluate(async()=>{
    const {AmbientThemeRequest,neutralTheme,validArtwork}=await import('/src/lib/ambientPalette.ts');
    const deferred: Array<(value:typeof neutralTheme)=>void>=[];
    const loader=new AmbientThemeRequest(()=>new Promise((resolve)=>deferred.push(resolve)));
    const first=loader.request('data:image/png;base64,AAAA');
    const second=loader.request('data:image/png;base64,BBBB');
    deferred[1]({...neutralTheme,identity:'new'}); const current=await second;
    deferred[0]({...neutralTheme,identity:'old'}); const stale=await first;
    const fallback=await loader.request('https://external.example/cover.jpg');
    return {current:current?.identity,stale,fallback:fallback?.identity,invalid:validArtwork('data:image/png;base64,'+'A'.repeat(2_800_001))};
  });
  expect(result).toEqual({current:'new',stale:undefined,fallback:'neutral',invalid:false});
});
