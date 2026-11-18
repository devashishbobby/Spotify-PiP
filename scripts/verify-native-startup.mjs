// Opt-in packaged-app smoke test. Changes only SpotifyPiP's startup choice;
// leaves it enabled and the release app open in Settings for user review.
import { chromium } from '@playwright/test';
import { spawn } from 'node:child_process';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';

const root = fileURLToPath(new URL('../', import.meta.url));
const executable = resolve(root, 'src-tauri/target/release/spotify-pip.exe');
const endpoint = 'http://127.0.0.1:9239';
let app;
let browser;

async function launch(args) {
  app = spawn(executable, args, {
    cwd: root,
    detached: true,
    windowsHide: true,
    stdio: 'ignore',
    env: {
      ...process.env,
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--remote-debugging-port=9239 --remote-debugging-address=127.0.0.1',
    },
  });
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      const response = await fetch(`${endpoint}/json/version`);
      if (response.ok) break;
    } catch {}
    if (app.exitCode !== null) throw new Error(`App exited: ${app.exitCode}`);
    await delay(200);
  }
  browser = await chromium.connectOverCDP(endpoint);
  let page;
  for (let attempt = 0; attempt < 50; attempt++) {
    page = browser.contexts().flatMap(context => context.pages()).find(candidate => /tauri\.localhost|^tauri:/.test(candidate.url()));
    if (page) break;
    await delay(100);
  }
  assert.ok(page, 'Release WebView was found');
  await page.waitForFunction(() => !!window.__TAURI_INTERNALS__);
  return page;
}

async function invoke(page, name) {
  return page.evaluate(command => window.__TAURI_INTERNALS__.invoke(command), name);
}

async function stop() {
  await browser?.close(); // Disconnect CDP before stopping only our child process.
  const stopped = new Promise(resolve => app.once('exit', resolve));
  app.kill();
  await stopped;
  await delay(700);
}

try {
  let page = await launch(['--startup']);
  assert.equal((await invoke(page, 'get_desktop_status')).visible, false, 'Sign-in launch stays hidden');
  assert.equal(await invoke(page, 'get_startup_enabled'), true, 'First launch defaults to startup enabled');

  const second = spawn(executable, [], { windowsHide: true, stdio: 'ignore' });
  const secondExit = await new Promise(resolve => second.once('exit', resolve));
  assert.equal(secondExit, 0, 'Second launch restores the existing instance');
  for (let attempt = 0; attempt < 50; attempt++) {
    if ((await invoke(page, 'get_desktop_status')).visible) break;
    await delay(100);
  }
  assert.equal((await invoke(page, 'get_desktop_status')).visible, true);
  await page.getByRole('button', { name: 'Player settings', exact: true }).click();
  let startup = page.getByRole('switch', { name: 'Start when Windows boots up' });
  await startup.uncheck();
  assert.equal(await invoke(page, 'get_startup_enabled'), false, 'Native switch removes registration');
  await stop();

  page = await launch([]);
  assert.equal((await invoke(page, 'get_desktop_status')).visible, true, 'Normal launch opens player');
  assert.equal(await invoke(page, 'get_startup_enabled'), false, 'Disabled startup survives process restart');
  await page.getByRole('button', { name: 'Player settings', exact: true }).click();
  startup = page.getByRole('switch', { name: 'Start when Windows boots up' });
  assert.equal(await startup.isChecked(), false);
  await startup.check();
  assert.equal(await invoke(page, 'get_startup_enabled'), true, 'Native switch restores registration');
  await page.screenshot({ path: resolve(root, 'test-results/native-startup-settings.png'), omitBackground: true });
  console.log(JSON.stringify({
    passed: ['hidden sign-in launch', 'default enabled', 'single-instance restore', 'native disable', 'disabled across process restart', 'visible normal launch', 'native enable'],
    pid: app.pid,
    screenshot: 'test-results/native-startup-settings.png',
  }, null, 2));
  await browser.close();
  app.unref();
} catch (error) {
  await browser?.close().catch(() => {});
  app?.kill();
  throw error;
}
