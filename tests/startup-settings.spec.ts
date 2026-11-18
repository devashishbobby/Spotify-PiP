import { expect, test } from '@playwright/test';
import { nativeFixture } from './helpers/nativeFixture';

test('startup defaults on, persists both choices, and keeps the confirmed choice on failure', async ({ page }) => {
  await nativeFixture(page);
  await page.getByRole('button', { name: 'Player settings', exact: true }).click();
  const startup = page.getByRole('switch', { name: 'Start when Windows boots up' });
  await expect(startup).toBeEnabled();
  await expect(startup).toBeChecked();
  await startup.uncheck();
  await expect(startup).not.toBeChecked();
  await page.reload();
  await page.getByRole('button', { name: 'Player settings', exact: true }).click();
  await expect(startup).not.toBeChecked();
  await startup.check();
  await expect(startup).toBeChecked();
  await page.reload();
  await page.getByRole('button', { name: 'Player settings', exact: true }).click();
  await expect(startup).toBeChecked();
  await page.evaluate(() => (window as any).__ambientFixture.failStartup(true));
  await startup.click();
  await expect(page.getByRole('alert')).toContainText('Windows startup access denied');
  await expect(startup).toBeChecked();
  await expect(startup).toBeEnabled();
});

for (const width of [320, 360, 480]) {
  test(`settings fit at ${width}px and support keyboard toggling`, async ({ page }) => {
    await page.setViewportSize({ width, height: width === 480 ? 280 : 208 });
    await nativeFixture(page);
    await page.getByRole('button', { name: 'Player settings', exact: true }).click();
    const startup = page.getByRole('switch', { name: 'Start when Windows boots up' });
    await expect(startup).toBeInViewport({ ratio: 1 });
    await expect(page.getByRole('button', { name: 'Reset position' })).toBeInViewport({ ratio: 1 });
    expect(await page.locator('.settings').evaluate(element => element.scrollHeight <= element.clientHeight)).toBe(true);
    await startup.focus();
    await page.keyboard.press('Space');
    await expect(startup).not.toBeChecked();
    await page.keyboard.press('Space');
    await expect(startup).toBeChecked();
    if (width === 360) await page.screenshot({ path: 'docs/validation/startup-settings.png', omitBackground: true });
  });
}
