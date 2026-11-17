import { expect, test } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import { nativeFixture as fixture } from './helpers/nativeFixture';


test('album-derived glass handles red, blue, gold, bright and monochrome covers',async({page})=>{
  await fixture(page); mkdirSync('test-results',{recursive:true});
  for(const color of ['red','blue','gold','white','mono']) {
    await page.evaluate((color)=>(window as any).__ambientFixture.color(color),color);
    const expected=await page.evaluate(async()=>{const {artworkIdentity}=await import('/src/lib/ambientPalette.ts');return artworkIdentity((window as any).__ambientFixture.identity());});
    await expect(page.locator('.ambient-backdrop')).toHaveAttribute('data-ambient-id',expected);
    await expect(page.locator('.ambient-scene').last()).toHaveAttribute('data-theme-id',expected);
    await expect.poll(()=>page.locator('.ambient-scene').count()).toBe(1);
    await expect(page.getByRole('button',{name:'Play',exact:true})).toHaveCSS('background-color','rgb(255, 255, 255)');
    await expect(page.getByRole('heading',{name:'Soft focus'})).toHaveCSS('color','rgb(255, 255, 255)');
    await expect(page.locator('.ambient-backdrop')).toHaveCSS('pointer-events','none');
    await page.screenshot({path:`test-results/phase3-${color}.png`,omitBackground:true});
  }
});

test('rapid artwork switches settle on the latest cover and missing/corrupt art clears stale ambience',async({page})=>{
  await fixture(page);
  await page.evaluate(()=>{for(const color of ['blue','gold','red','blue','green']) (window as any).__ambientFixture.color(color);});
  const expected=await page.evaluate(async()=>{const {artworkIdentity}=await import('/src/lib/ambientPalette.ts');return artworkIdentity((window as any).__ambientFixture.identity());});
  await expect(page.locator('.ambient-backdrop')).toHaveAttribute('data-ambient-id',expected);
  await expect.poll(()=>page.locator('.ambient-scene').count()).toBe(1);
  await page.evaluate(()=>(window as any).__ambientFixture.publish({artworkDataUrl:'data:image/png;base64,AAAA'}));
  await expect(page.locator('.ambient-backdrop')).toHaveAttribute('data-ambient-id','neutral');
  await page.evaluate(()=>(window as any).__ambientFixture.publish({status:'disconnected',artworkDataUrl:null,title:null,artist:null}));
  await expect.poll(()=>page.locator('.ambient-scene[data-artwork="true"]').count()).toBe(0);
  expect(await page.evaluate(()=>(window as any).__ambientFixture.listeners())).toBe(1);
});

test('decorative glass cannot intercept the protected controls or header drag',async({page})=>{
  await fixture(page);
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(page.getByRole('button',{name:'Pause',exact:true})).toBeVisible();
  await page.getByRole('button',{name:'Spotify volume',exact:true}).click();
  await expect(page.getByRole('slider',{name:'Spotify volume level'})).toBeVisible();
  await page.getByRole('slider',{name:'Spotify volume level'}).press('Escape');
  await page.locator('.player-header').click({position:{x:160,y:20}});
  const calls=await page.evaluate(()=>(window as any).__ambientFixture.calls);
  expect(calls).toContain('media_play_pause'); expect(calls).toContain('plugin:window|start_dragging');
  await expect(page.locator('.ambient-backdrop button,.ambient-backdrop input,.ambient-backdrop [tabindex]')).toHaveCount(0);
});
test('ambient motion follows playback and visibility and honors reduced motion',async({page})=>{
  await fixture(page);
  const glow=page.locator('.ambient-scene').last().locator('.ambient-glow--one');
  await expect(glow).toHaveCSS('animation-play-state','paused');
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(page.locator('.ambient-backdrop')).toHaveAttribute('data-motion','running');
  await expect(glow).toHaveCSS('animation-play-state','running');
  await page.evaluate(()=>(window as any).__ambientFixture.visible(false));
  await expect(glow).toHaveCSS('animation-play-state','paused');
  await expect(page.locator('.ambient-scene').last().locator('.ambient-visualizer')).toHaveCSS('visibility','hidden');
  await page.evaluate(()=>(window as any).__ambientFixture.visible(true));
  await page.emulateMedia({reducedMotion:'reduce'});
  await expect(glow).toHaveCSS('animation-name','none');
  await expect(page.locator('.ambient-backdrop')).not.toHaveAttribute('data-ambient-id','neutral');
  expect(await page.evaluate(()=>(window as any).__ambientFixture.listeners())).toBe(1);
});

test('ambient waves paint visibly above the artwork tint and actually move',async({page})=>{
  await fixture(page);
  await expect(page.locator('.ambient-scene[data-artwork="true"]')).toHaveCount(1);
  await expect(page.locator('.ambient-scene')).toHaveCount(1);
  const visualizer=page.locator('.ambient-visualizer');
  const withWaves=(await page.screenshot()).toString('base64');
  await visualizer.evaluate((element)=>{element.style.visibility='hidden';});
  const withoutWaves=(await page.screenshot()).toString('base64');
  const painted=await page.evaluate(async({withWaves,withoutWaves})=>{
    async function pixels(encoded:string) {
      const image=new Image(); image.src=`data:image/png;base64,${encoded}`; await image.decode();
      const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
      const context=canvas.getContext('2d')!;context.drawImage(image,0,0);
      return context.getImageData(120,140,220,18).data;
    }
    const [a,b]=await Promise.all([pixels(withWaves),pixels(withoutWaves)]);
    const differences=[];
    for(let i=0;i<a.length;i+=4)differences.push(Math.max(Math.abs(a[i]-b[i]),Math.abs(a[i+1]-b[i+1]),Math.abs(a[i+2]-b[i+2])));
    differences.sort((a,b)=>a-b);
    return {mean:differences.reduce((sum,value)=>sum+value,0)/differences.length,p95:differences[Math.floor(differences.length*.95)]};
  },{withWaves,withoutWaves});
  expect(painted.mean).toBeGreaterThan(1.2);
  expect(painted.p95).toBeGreaterThan(10);
  await visualizer.evaluate((element)=>{element.style.visibility='';});
  await page.getByRole('button',{name:'Play',exact:true}).click();
  const ribbon=page.locator('.ambient-ribbon');
  await ribbon.evaluate((element)=>{element.getAnimations()[0].currentTime=0;});
  const start=await ribbon.evaluate((element)=>getComputedStyle(element).transform);
  await ribbon.evaluate((element)=>{element.getAnimations()[0].currentTime=4000;});
  expect(await ribbon.evaluate((element)=>getComputedStyle(element).transform)).not.toBe(start);
  await page.screenshot({path:'test-results/phase3-playing.png',omitBackground:true});
});
