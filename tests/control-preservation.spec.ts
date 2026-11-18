import { expect, test, type Page } from '@playwright/test';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';

async function measure(page: Page) {
  return page.locator('button,input,select,.album-art,.track-copy h1,.track-copy p,.player-header,.player-body,.transport,.timeline,.player-footer,.settings').evaluateAll((elements) => elements.map((element) => {
    const rect = element.getBoundingClientRect();
    const style = getComputedStyle(element);
    const properties = ['color','background-color','border-radius','border-width','padding','font-family','font-size','font-weight','line-height','gap','outline-style','outline-color','outline-offset','box-shadow','fill','stroke','cursor','opacity','transform'];
    return {
      tag: element.tagName, className: element.className,
      label: element.getAttribute('aria-label'), title: element.getAttribute('title'), role: element.getAttribute('role'),
      disabled: element.hasAttribute('disabled'), text: element.tagName === 'BUTTON' ? element.textContent?.trim() : undefined,
      svg: element.querySelector('svg')?.outerHTML,
      rect: [rect.x,rect.y,rect.width,rect.height].map((value) => Math.round(value * 100) / 100),
      styles: Object.fromEntries(properties.map((property) => [property,style.getPropertyValue(property)])),
    };
  }));
}

for (const size of [{width:320,height:208},{width:360,height:208},{width:480,height:280}]) {
  test(`protected player controls match Phase 2 at ${size.width} × ${size.height}`, async ({ page }) => {
    await page.setViewportSize(size); await page.goto('/');
    await page.getByRole('button',{name:'Pause',exact:true}).click();
    await page.mouse.move(1,1);
    await page.getByRole('button',{name:'Play',exact:true}).blur();
    await page.waitForTimeout(240);
    const snapshots: Record<string, unknown> = { player: await measure(page) };
    if (size.width === 360) {
      mkdirSync(resolve('docs/validation'),{recursive:true});
      await page.screenshot({ path: process.env.UPDATE_CONTROL_BASELINE === '1' ? 'docs/validation/phase2-player.png' : 'test-results/phase4-controls-neutral.png', omitBackground: true });
    }
    const play = page.getByRole('button',{name:'Play',exact:true});
    await play.hover(); await page.waitForTimeout(200); snapshots.hover = await measure(page);
    await page.mouse.move(1,1); await play.focus(); await page.waitForTimeout(200); snapshots.focus = await measure(page);
    const box = await play.boundingBox(); expect(box).not.toBeNull();
    await page.mouse.move(box!.x+box!.width/2,box!.y+box!.height/2); await page.mouse.down();
    await page.waitForTimeout(200); snapshots.pressed = await measure(page);
    await page.mouse.move(1,1); await page.mouse.up();
    await page.getByRole('button',{name:'Spotify volume',exact:true}).click();
    await page.waitForTimeout(200); snapshots.volume = await measure(page);
    await page.getByRole('slider',{name:'Spotify volume level'}).press('Escape');
    const file = resolve('tests/fixtures',`phase2-controls-${size.width}.json`);
    if (process.env.UPDATE_CONTROL_BASELINE === '1') {
      mkdirSync(resolve('tests/fixtures'),{recursive:true});
      writeFileSync(file,JSON.stringify(snapshots,null,2)+'\n');
    } else {
      // Settings now intentionally include a third row. Their layout/behavior is
      // covered by startup-settings.spec.ts; keep every player snapshot protected.
      const { settings: _settings, ...playerBaseline } = JSON.parse(readFileSync(file,'utf8'));
      // Baseline JSON omits optional undefined fields, as do older Playwright versions.
      expect(JSON.parse(JSON.stringify(snapshots))).toEqual(playerBaseline);
    }
  });
}
