import { expect, test, type Page } from '@playwright/test';
import { createHash } from 'node:crypto';
import { readFileSync, mkdirSync } from 'node:fs';
import { nativeFixture } from './helpers/nativeFixture';

async function ready(page: Page) {
  await nativeFixture(page);
  await expect(page.locator('.album-perimeter')).toHaveAttribute('data-state','paused');
  await expect(page.locator('.ambient-scene')).toHaveCount(1);
}

test('Phase 3 backdrop, controls, media implementation and native geometry match protected hashes after newline normalization',()=>{
  const files=JSON.parse(readFileSync('tests/fixtures/phase3-protected-sha256.json','utf8').replace(/^\uFEFF/,'')) as {path:string;sha256:string}[];
  expect(files).toHaveLength(18);
  for(const file of files)expect(createHash('sha256').update(readFileSync(file.path,'utf8').replace(/\r\n/g,'\n')).digest('hex'),file.path).toBe(file.sha256);
});

for(const scale of [1,1.25,1.5])for(const size of [{width:320,height:208},{width:360,height:208},{width:480,height:280}]) {
  test(`four-sided peaks stay outside the cover and inside the ${size.width}×${size.height} window at ${scale*100}% device scale`,async({browser})=>{
    const context=await browser.newContext({viewport:size,deviceScaleFactor:scale,baseURL:'http://localhost:1423'});
    try {
      const page=await context.newPage(); await ready(page);
      await page.getByRole('button',{name:'Play',exact:true}).click();
      const perimeter=page.locator('.album-perimeter');
      await expect(perimeter).toHaveAttribute('data-state','playing');
      await expect(perimeter.locator('.perimeter-bar')).toHaveCount(64);
      for(const side of ['top','right','bottom','left'])await expect(perimeter.locator(`.perimeter-edge[data-side="${side}"] .perimeter-bar`)).toHaveCount(16);
      // Inspect maximum travel, including the unscaled pulse, independently of phase.
      await page.locator('.perimeter-pulse').evaluateAll((elements)=>{for(const element of elements){(element as HTMLElement).style.animation='none';(element as HTMLElement).style.transform='none';}});
      const result=await page.evaluate(()=>{
        const box=(element:Element)=>{const r=element.getBoundingClientRect();return {x:r.x,y:r.y,right:r.right,bottom:r.bottom,width:r.width,height:r.height};};
        const art=box(document.querySelector('.album-art')!);
        const boundary=box(document.querySelector('.album-perimeter')!);
        const pulses=[...document.querySelectorAll('.perimeter-pulse')].map((element)=>({side:element.closest('.perimeter-edge')!.getAttribute('data-side'),...box(element)}));
        const obstacles=[...document.querySelectorAll('.track-copy,button,.timeline input,.timeline > span')].map(box);
        return {art,boundary,pulses,obstacles,anchor:box(document.querySelector('.album-visualizer-anchor')!),pointer:getComputedStyle(document.querySelector('.album-perimeter')!).pointerEvents};
      });
      expect(result.anchor.width).toBe(size.height>=240?96:88);expect(result.anchor.height).toBe(result.art.height);
      expect(result.anchor.x).toBe(result.art.x);expect(result.anchor.y).toBe(result.art.y);expect(result.art.width).toBe(result.anchor.width);
      expect(result.boundary.x).toBeGreaterThanOrEqual(0);expect(result.boundary.y).toBeGreaterThanOrEqual(0);
      expect(result.boundary.right).toBeLessThanOrEqual(size.width);expect(result.boundary.bottom).toBeLessThanOrEqual(size.height);
      expect(result.pointer).toBe('none');
      for(const pulse of result.pulses) {
        expect(pulse.x).toBeGreaterThanOrEqual(result.boundary.x);expect(pulse.y).toBeGreaterThanOrEqual(result.boundary.y);
        expect(pulse.right).toBeLessThanOrEqual(result.boundary.right);expect(pulse.bottom).toBeLessThanOrEqual(result.boundary.bottom);
        if(pulse.side==='top')expect(pulse.bottom).toBeLessThanOrEqual(result.art.y-1);
        if(pulse.side==='bottom')expect(pulse.y).toBeGreaterThanOrEqual(result.art.bottom+1);
        if(pulse.side==='left')expect(pulse.right).toBeLessThanOrEqual(result.art.x-1);
        if(pulse.side==='right')expect(pulse.x).toBeGreaterThanOrEqual(result.art.right+1);
        for(const obstacle of result.obstacles)expect(pulse.x<obstacle.right && pulse.right>obstacle.x && pulse.y<obstacle.bottom && pulse.bottom>obstacle.y).toBe(false);
      }
      await expect(perimeter.locator('button,input,[tabindex]')).toHaveCount(0);
    } finally {await context.close();}
  });
}

test('perimeter follows playback, visibility, missing art, reduced motion and the latest palette without remounting',async({page})=>{
  await ready(page);
  await page.evaluate(()=>{(document.querySelector('.perimeter-bar') as HTMLElement).dataset.persistence='same-node';});
  const root=page.locator('.album-perimeter'); const pulse=root.locator('.perimeter-pulse').first();
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(root).toHaveAttribute('data-state','playing'); await expect(pulse).toHaveCSS('animation-play-state','running');
  await page.evaluate(()=>{for(const name of ['blue','gold','red','blue','green'])(window as any).__ambientFixture.color(name);});
  const identity=await page.evaluate(async()=>{const {artworkIdentity}=await import('/src/lib/ambientPalette.ts');return artworkIdentity((window as any).__ambientFixture.identity());});
  await expect(root).toHaveAttribute('data-theme-id',identity);
  await expect(root.locator('.perimeter-bar').first()).toHaveAttribute('data-persistence','same-node');
  const expectedColor=await page.evaluate(async()=>{
    const {extractAmbientTheme}=await import('/src/lib/ambientPalette.ts');
    const theme=await extractAmbientTheme((window as any).__ambientFixture.identity(),new AbortController().signal);
    const sample=document.createElement('span');sample.style.color=`hsl(${theme.palette.hues[0]} ${Math.round(theme.palette.saturation[0]*100)}% 64%)`;
    document.body.append(sample);const color=getComputedStyle(sample).color;sample.remove();return color;
  });
  await expect.poll(()=>root.evaluate((element)=>getComputedStyle(element).color)).toBe(expectedColor);
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  await expect(root).toHaveAttribute('data-state','paused'); await expect(pulse).toHaveCSS('animation-play-state','paused'); await expect(pulse).toHaveCSS('opacity','0');
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await page.evaluate(()=>(window as any).__ambientFixture.visible(false));
  await expect(root).toHaveCSS('visibility','hidden');await expect(pulse).toHaveCSS('animation-play-state','paused');
  await page.evaluate(()=>(window as any).__ambientFixture.visible(true));
  await page.emulateMedia({reducedMotion:'reduce'});await expect(pulse).toHaveCSS('animation-name','none');
  await page.evaluate(()=>(window as any).__ambientFixture.publish({artworkDataUrl:null}));
  await expect(root).toHaveAttribute('data-state','off');await expect(root).toHaveAttribute('data-theme-id','neutral');
  await page.evaluate(()=>(window as any).__ambientFixture.publish({status:'disconnected',artworkDataUrl:null,title:null,artist:null}));
  await expect(page.getByText('Spotify disconnected',{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>(window as any).__ambientFixture.listeners())).toBe(1);
});

test('header labels follow playback without a status dot and original controls remain usable',async({page})=>{
  await ready(page);
  await expect(page.locator('.player-footer')).not.toContainText('SPOTIFY PIP');
  await expect(page.locator('.status-dot')).toHaveCount(0);
  await expect(page.locator('.header-label')).toHaveText('PAUSED');
  await page.getByRole('button',{name:'Play',exact:true}).click();
  await expect(page.locator('.header-label')).toHaveText('NOW PLAYING');
  await page.getByRole('button',{name:'Pause',exact:true}).click();
  await expect(page.locator('.header-label')).toHaveText('PAUSED');
  await page.mouse.move(1,1);
  await page.getByRole('button',{name:'Play',exact:true}).blur();
  await page.screenshot({path:'docs/validation/paused-header.png',omitBackground:true});
  await expect(page.getByRole('heading',{name:'Soft focus'})).toBeVisible();await expect(page.getByText('The Quiet Hours',{exact:true})).toBeVisible();
  for(const name of ['Player settings','Hide player','Previous track','Play','Next track','Spotify volume'])await expect(page.getByRole('button',{name,exact:true})).toBeVisible();
  await expect(page.getByRole('slider',{name:'Seek track'})).toHaveAttribute('aria-valuetext','1:26 of 3:42');
  await page.getByRole('button',{name:'Spotify volume',exact:true}).click();await expect(page.getByRole('slider',{name:'Spotify volume level'})).toBeFocused();
  await page.keyboard.press('Escape');await page.getByRole('button',{name:'Player settings',exact:true}).click();
  await expect(page.locator('.header-label')).toHaveText('PLAYER SETTINGS');
  await expect(page.getByRole('button',{name:'Back to player'})).toBeFocused();await page.keyboard.press('Escape');
  await expect(page.getByRole('button',{name:'Player settings',exact:true})).toBeFocused();
  await page.locator('.player-header').click({position:{x:160,y:20}});
  expect(await page.evaluate(()=>(window as any).__ambientFixture.calls)).toContain('plugin:window|start_dragging');
  await page.evaluate(()=>(window as any).__ambientFixture.publish({status:'spotify_not_running',source:'none',artworkDataUrl:null,title:null,artist:null}));
  await expect(page.getByText('Open Spotify to begin',{exact:true})).toBeVisible();
  await expect(page.getByRole('button',{name:'Retry',exact:true})).toBeVisible();
});

test('new decoration changes pixels only inside its clip and leaves the artwork and background outside intact',async({page})=>{
  await ready(page); await page.mouse.move(350,200);
  const ring=await page.locator('.album-perimeter').boundingBox();const art=await page.locator('.album-art').boundingBox();expect(ring).not.toBeNull();expect(art).not.toBeNull();
  const visible=(await page.screenshot({omitBackground:true})).toString('base64');
  await page.locator('.album-perimeter').evaluate((element)=>{(element as HTMLElement).style.visibility='hidden';});
  const hidden=(await page.screenshot({omitBackground:true})).toString('base64');
  const difference=await page.evaluate(async({visible,hidden,ring,art})=>{
    async function pixels(data:string){const image=new Image();image.src=`data:image/png;base64,${data}`;await image.decode();const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;const ctx=canvas.getContext('2d')!;ctx.drawImage(image,0,0);return {data:ctx.getImageData(0,0,image.width,image.height).data,width:image.width};}
    const [a,b]=await Promise.all([pixels(visible),pixels(hidden)]);let outside=0,interior=0,inside=0;
    for(let i=0;i<a.data.length;i+=4){const x=(i/4)%a.width,y=Math.floor(i/4/a.width);let change=0;for(let c=0;c<4;c++)change=Math.max(change,Math.abs(a.data[i+c]-b.data[i+c]));if(!change)continue;
      if(x<Math.floor(ring.x)||x>=Math.ceil(ring.x+ring.width)||y<Math.floor(ring.y)||y>=Math.ceil(ring.y+ring.height))outside++;
      if(x>=art.x&&x<art.x+art.width&&y>=art.y&&y<art.y+art.height)interior++;
      inside++;
    }return {outside,interior,inside};
  },{visible,hidden,ring:ring!,art:art!});
  expect(difference.outside).toBe(0);expect(difference.interior).toBe(0);expect(difference.inside).toBeGreaterThan(100);
  await page.locator('.album-perimeter').evaluate((element)=>{(element as HTMLElement).style.visibility='';});
  await page.getByRole('button',{name:'Play',exact:true}).click();
  mkdirSync('docs/validation',{recursive:true});await page.screenshot({path:'docs/validation/phase4-player.png',omitBackground:true});
});

test('window motion reverses rapid requests and acknowledges only the latest target without media actions',async({page})=>{
  await ready(page);
  const final=await page.evaluate(()=>{const fixture=(window as any).__ambientFixture;for(const visible of [false,true,false,true,false,true])fixture.transition(visible);return fixture.transition(false);});
  await expect.poll(()=>page.evaluate((revision)=>(window as any).__ambientFixture.requests.some((request:any)=>request.name==='window_transition_complete'&&request.args.revision===revision),final)).toBe(true);
  await expect(page.locator('.album-perimeter')).toHaveAttribute('data-visible','false');
  expect(await page.evaluate(()=>(window as any).__ambientFixture.requests.filter((request:any)=>request.name==='window_transition_complete').map((request:any)=>request.args.revision))).toEqual([final]);
  await page.evaluate(()=>(window as any).__ambientFixture.transition(true));
  await expect(page.locator('.album-perimeter')).toHaveAttribute('data-visible','true');
  await expect(page.locator('.player')).toHaveCSS('opacity','1');
  await expect.poll(()=>page.locator('.player').evaluate((element)=>getComputedStyle(element).transform)).toMatch(/^(none|matrix\(1, 0, 0, 1, 0, 0\))$/);
  await page.evaluate((revision)=>(window as any).__ambientFixture.event('desktop://transition',{revision,visible:false,animate:true,wasVisible:true}),final-1);
  await expect(page.locator('.player')).toHaveCSS('opacity','1');
  const calls=await page.evaluate(()=>(window as any).__ambientFixture.calls as string[]);
  expect(calls.filter((name)=>/^media_(play_pause|next|previous|seek|set_volume|set_muted)$/.test(name))).toEqual([]);
  expect(await page.evaluate(()=>(window as any).__ambientFixture.listeners())).toBe(1);
});

test('reduced motion completes native visibility requests without transform animation',async({page})=>{
  await page.emulateMedia({reducedMotion:'reduce'});await ready(page);
  await page.evaluate(()=>(window as any).__ambientFixture.transition(false));
  await expect(page.locator('.album-perimeter')).toHaveAttribute('data-visible','false');
  await expect(page.locator('.player')).toHaveCSS('opacity','0');
  expect(await page.evaluate(()=>(window as any).__ambientFixture.requests.some((request:any)=>request.name==='window_motion_ready'&&request.args.reducedMotion===true))).toBe(true);
  expect(await page.locator('.player').evaluate((element)=>element.getAnimations().length)).toBe(0);
  await page.evaluate(()=>(window as any).__ambientFixture.transition(true));await expect(page.locator('.player')).toHaveCSS('opacity','1');
});
