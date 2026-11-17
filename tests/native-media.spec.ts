import { expect, test, type Page } from "@playwright/test";
import type { MediaSnapshot } from "../src/types/media";

function snapshot(overrides: Partial<MediaSnapshot> = {}): MediaSnapshot {
  return {
    source: "none",
    status: "spotify_not_running",
    title: null,
    artist: null,
    album: null,
    artworkDataUrl: null,
    positionMs: null,
    durationMs: null,
    seekStartMs: null,
    seekEndMs: null,
    lastTimelineUpdateUtcMs: null,
    snapshotAtUtcMs: Date.now(),
    playbackRate: 1,
    canPlayPause: false,
    canNext: false,
    canPrevious: false,
    canSeek: false,
    volumeLevel: null,
    volumeMuted: null,
    canSetVolume: false,
    errorCode: null,
    errorMessage: null,
    revision: 1,
    sessionGeneration: 0,
    ...overrides,
  };
}
const playing = (overrides: Partial<MediaSnapshot> = {}) =>
  snapshot({
    source: "spotify",
    status: "playing",
    title: "Native track A",
    artist: "Native artist",
    album: "Native album",
    positionMs: 10_000,
    durationMs: 180_000,
    canPlayPause: true,
    canNext: true,
    canPrevious: true,
    sessionGeneration: 7,
    ...overrides,
  });

async function installBridge(page: Page, initial: MediaSnapshot, race = false) {
  await page.addInitScript(
    ({ initial, race }) => {
      // Fake IPC adapter only. The app runs its normal native hook with mock mode disabled.
      const scope = window as unknown as Record<string, any>;
      let current = initial;
      let count = 0;
      const callbacks = new Map<number, (event: unknown) => void>();
      const listeners = new Map<number, { event: string; handler: number }>();
      const calls: { name: string; args: Record<string, unknown> }[] = [];
      let pending: ((value: { accepted: boolean }) => void) | null = null;
      const desktop = {
        visible: true,
        alwaysOnTop: true,
        shortcut: "Ctrl+Alt+M",
        shortcutError: null,
      };
      function emit(event: string, payload: unknown) {
        for (const [id, entry] of listeners)
          if (entry.event === event)
            callbacks.get(entry.handler)?.({ id, event, payload });
      }
      scope.isTauri = true;
      scope.__TAURI_EVENT_PLUGIN_INTERNALS__ = {
        unregisterListener(_event: string, id: number) {
          listeners.delete(id);
        },
      };
      scope.__TAURI_INTERNALS__ = {
        metadata: { currentWindow: { label: "main" } },
        transformCallback(callback: (event: unknown) => void) {
          const id = ++count;
          callbacks.set(id, callback);
          return id;
        },
        async invoke(name: string, args: Record<string, any> = {}) {
          if (name === "plugin:event|listen") {
            const id = ++count;
            listeners.set(id, { event: args.event, handler: args.handler });
            return id;
          }
          if (name === "plugin:event|unlisten") {
            listeners.delete(args.eventId);
            return;
          }
          if (name === "get_desktop_status") return desktop;
          if (name === "get_media_snapshot") {
            scope.__testMedia.subscribedBeforeInitial = [
              ...listeners.values(),
            ].some((entry) => entry.event === "media://snapshot-updated");
            if (race) {
              const stale = current;
              current = {
                ...current,
                source: "spotify",
                status: "paused",
                title: "Newer native track",
                artist: "Native artist",
                revision: 10,
                sessionGeneration: 1,
              };
              emit("media://snapshot-updated", current);
              await new Promise((resolve) => setTimeout(resolve, 40));
              return stale;
            }
            return current;
          }
          if (name === "media_refresh") return current;
          if (
            [
              "media_play_pause",
              "media_next",
              "media_previous",
              "media_seek",
              "media_set_volume",
              "media_set_muted",
            ].includes(name)
          ) {
            calls.push({ name, args });
            if (scope.__testMedia.rejectNext) {
              const error = scope.__testMedia.rejectNext;
              scope.__testMedia.rejectNext = null;
              throw error;
            }
            return new Promise((resolve) => {
              pending = resolve;
            });
          }
          return;
        },
      };
      scope.__testMedia = {
        subscribedBeforeInitial: false,
        rejectNext: null,
        calls,
        publish(next: MediaSnapshot, notify = true) {
          current = { ...next, snapshotAtUtcMs: Date.now() };
          if (notify) emit("media://snapshot-updated", current);
        },
        resolve(next?: MediaSnapshot) {
          if (next) current = { ...next, snapshotAtUtcMs: Date.now() };
          pending?.({ accepted: true, ...(next ? { snapshot: current } : {}) });
          pending = null;
        },
        visible(visible: boolean) {
          desktop.visible = visible;
          emit("desktop://status", { ...desktop });
        },
        listenerCount() {
          return [...listeners.values()].filter(
            (entry) => entry.event === "media://snapshot-updated",
          ).length;
        },
      };
    },
    { initial, race },
  );
}
async function publish(page: Page, value: MediaSnapshot, notify = true) {
  await page.evaluate(
    ({ value, notify }) => (window as any).__testMedia.publish(value, notify),
    { value, notify },
  );
}

test("mock-free transitions clear stale data and recover through Retry", async ({
  page,
}) => {
  await installBridge(page, snapshot());
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Open Spotify to begin" }),
  ).toBeVisible();
  await expect(page.getByText("DEMO PREVIEW")).toHaveCount(0);
  await publish(page, playing({ revision: 2 }));
  await expect(
    page.getByRole("heading", { name: "Native track A" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeEnabled();
  await publish(page, playing({ status: "paused", revision: 3 }));
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
  const paused = await page
    .getByRole("progressbar")
    .getAttribute("aria-valuetext");
  await page.waitForTimeout(1100);
  await expect(page.getByRole("progressbar")).toHaveAttribute(
    "aria-valuetext",
    paused!,
  );
  await publish(page, snapshot({ status: "disconnected", revision: 4 }));
  await expect(
    page.getByRole("heading", { name: "Spotify disconnected" }),
  ).toBeVisible();
  await expect(page.getByText("Native track A")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Next track" })).toHaveCount(0);
  await expect(page.getByRole("progressbar")).toHaveAttribute(
    "aria-valuetext",
    "Timeline unavailable",
  );
  await publish(
    page,
    snapshot({
      status: "error",
      errorMessage: "Windows media access temporarily failed.",
      revision: 5,
    }),
  );
  await expect(
    page.getByText("Windows media access temporarily failed."),
  ).toBeVisible();
  await publish(page, playing({ revision: 6 }), false);
  await page.getByRole("button", { name: "Retry", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Native track A" }),
  ).toBeVisible();
  expect(
    await page.evaluate(() => (window as any).__testMedia.listenerCount()),
  ).toBe(1);
  expect(
    await page.evaluate(
      () => (window as any).__testMedia.subscribedBeforeInitial,
    ),
  ).toBe(true);
});

test("initial snapshot races and out-of-order events cannot overwrite newer native data", async ({
  page,
}) => {
  await installBridge(page, snapshot(), true);
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Newer native track" }),
  ).toBeVisible();
  await publish(page, playing({ title: "Stale track", revision: 3 }));
  await expect(
    page.getByRole("heading", { name: "Newer native track" }),
  ).toBeVisible();
  await expect(page.getByText("Stale track")).toHaveCount(0);
  expect(
    await page.evaluate(() => (window as any).__testMedia.listenerCount()),
  ).toBe(1);
});

test("pending controls serialize requests and accepted commands do not invent state", async ({
  page,
}) => {
  await installBridge(page, playing());
  await page.goto("/");
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole("button", { name: "Next track" })).toBeDisabled();
  expect(await page.evaluate(() => (window as any).__testMedia.calls)).toEqual([
    { name: "media_play_pause", args: { expectedSession: 7 } },
  ]);
  await page.evaluate(() => (window as any).__testMedia.resolve());
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toHaveCount(0);
  await publish(page, playing({ status: "paused", revision: 2 }));
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
  await page.evaluate(() => {
    (window as any).__testMedia.rejectNext = {
      code: "stale_session",
      message: "Spotify session changed. Please retry.",
    };
  });
  await page.getByRole("button", { name: "Next track" }).click();
  await expect(
    page.getByRole("button", { name: "Playback request failed · Retry" }),
  ).toBeVisible();
  await expect(
    page.getByRole("heading", { name: "Native track A" }),
  ).toBeVisible();
});

test("unknown timeline and unsupported controls stay honest", async ({
  page,
}) => {
  await installBridge(
    page,
    playing({
      positionMs: null,
      durationMs: null,
      canNext: false,
      canPrevious: false,
      canPlayPause: false,
    }),
  );
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Native track A" }),
  ).toBeVisible();
  await expect(page.getByRole("button", { name: "Next track" })).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Pause", exact: true }),
  ).toBeDisabled();
  await expect(page.getByRole("slider")).toHaveCount(0);
  await expect(page.getByRole("progressbar")).toHaveAttribute(
    "aria-valuetext",
    "Timeline unavailable",
  );
  await expect(page.locator(".album-art img")).toHaveCount(0);
  await expect(page.locator(".album-art svg")).toBeVisible();
});

test("real-mode scrubbing sends one millisecond request when released", async ({
  page,
}) => {
  await installBridge(
    page,
    playing({
      status: "paused",
      canSeek: true,
      seekStartMs: 0,
      seekEndMs: 180_000,
    }),
  );
  await page.goto("/");
  const slider = page.getByRole("slider", { name: "Seek track" });
  const box = await slider.boundingBox();
  expect(box).not.toBeNull();
  await page.mouse.move(box!.x + box!.width / 4, box!.y + box!.height / 2);
  await page.mouse.down();
  await page.mouse.move(box!.x + box!.width * 0.75, box!.y + box!.height / 2, {
    steps: 8,
  });
  expect(
    await page.evaluate(() => (window as any).__testMedia.calls.length),
  ).toBe(0);
  await page.mouse.up();
  const calls = await page.evaluate(() => (window as any).__testMedia.calls);
  expect(calls).toHaveLength(1);
  expect(calls[0].name).toBe("media_seek");
  expect(calls[0].args.expectedSession).toBe(7);
  expect(calls[0].args.positionMs).toBeGreaterThan(100_000);
  expect(calls[0].args.positionMs).toBeLessThan(160_000);
  await expect(slider).toBeDisabled();
  await expect(slider).toHaveValue(String(calls[0].args.positionMs));
  await page.evaluate(
    (confirmed) => (window as any).__testMedia.resolve(confirmed),
    playing({
      status: "paused",
      canSeek: true,
      positionMs: calls[0].args.positionMs,
      revision: 2,
    }),
  );
  await expect(slider).toHaveValue(String(calls[0].args.positionMs));
});

test("display interpolation stops while hidden and does not create subscriptions", async ({
  page,
}) => {
  await page.clock.install({ time: Date.now() });
  await installBridge(page, playing());
  await page.goto("/");
  await expect(
    page.getByRole("heading", { name: "Native track A" }),
  ).toBeVisible();
  const before = await page
    .getByRole("progressbar")
    .getAttribute("aria-valuetext");
  await page.clock.fastForward(2500);
  await expect(page.getByRole("progressbar")).not.toHaveAttribute(
    "aria-valuetext",
    before!,
  );
  await page.evaluate(() => (window as any).__testMedia.visible(false));
  const hidden = await page
    .getByRole("progressbar")
    .getAttribute("aria-valuetext");
  await page.clock.fastForward(2500);
  await expect(page.getByRole("progressbar")).toHaveAttribute(
    "aria-valuetext",
    hidden!,
  );
  expect(
    await page.evaluate(() => (window as any).__testMedia.listenerCount()),
  ).toBe(1);
});

test("Spotify volume uses scoped IPC and applies only authoritative returned volume/mute", async ({
  page,
}) => {
  await installBridge(
    page,
    playing({ volumeLevel: 0.75, volumeMuted: false, canSetVolume: true }),
  );
  await page.goto("/");
  await page
    .getByRole("button", { name: "Spotify volume", exact: true })
    .click();
  const slider = page.getByRole("slider", { name: "Spotify volume level" });
  await slider.press("Home");
  await expect(slider).toBeDisabled();
  expect(await page.evaluate(() => (window as any).__testMedia.calls)).toEqual([
    { name: "media_set_volume", args: { level: 0, expectedSession: 7 } },
  ]);
  await page.evaluate(
    (confirmed) => (window as any).__testMedia.resolve(confirmed),
    playing({
      volumeLevel: 0,
      volumeMuted: false,
      canSetVolume: true,
      revision: 2,
    }),
  );
  await expect(slider).toHaveValue("0");
  await expect(slider).toBeEnabled();
  await page.getByRole("button", { name: "Mute Spotify", exact: true }).click();
  await page.evaluate(
    (confirmed) => (window as any).__testMedia.resolve(confirmed),
    playing({
      volumeLevel: 0,
      volumeMuted: true,
      canSetVolume: true,
      revision: 3,
    }),
  );
  await expect(
    page.getByRole("button", { name: "Unmute Spotify", exact: true }),
  ).toBeVisible();
});

test("failed seek rolls back the pending target without inventing playback progress", async ({
  page,
}) => {
  await installBridge(
    page,
    playing({
      status: "paused",
      canSeek: true,
      seekStartMs: 0,
      seekEndMs: 180_000,
    }),
  );
  await page.goto("/");
  await page.evaluate(() => {
    (window as any).__testMedia.rejectNext = {
      code: "seek_unconfirmed",
      message: "Spotify has not confirmed the new position yet.",
    };
  });
  const slider = page.getByRole("slider", { name: "Seek track" });
  await slider.focus();
  await slider.press("End");
  await expect(
    page.getByRole("button", { name: "Playback request failed · Retry" }),
  ).toBeVisible();
  await expect(slider).toHaveValue("10000");
  await expect(slider).toBeEnabled();
});
