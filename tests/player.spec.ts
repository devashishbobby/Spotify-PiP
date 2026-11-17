import { expect, test } from "@playwright/test";

test("renders without browser or React errors", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  await page.reload();
  await expect(page.getByRole("heading", { name: "Soft focus" })).toBeVisible();
  await page.getByRole("button", { name: "Next track" }).click();
  await expect(
    page.getByRole("heading", { name: "Somewhere, slowly" }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test.beforeEach(async ({ page }) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { name: "Soft focus" })).toBeVisible();
});

test("demo transport, title truncation, and interactive seeking", async ({
  page,
}) => {
  await page.getByRole("button", { name: "Pause", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
  const pausedTime = await page
    .getByRole("slider")
    .getAttribute("aria-valuetext");
  await page.waitForTimeout(1100);
  await expect(page.getByRole("slider")).toHaveAttribute(
    "aria-valuetext",
    pausedTime!,
  );
  await page.getByRole("button", { name: "Next track" }).click();
  await expect(
    page.getByRole("heading", { name: "Somewhere, slowly" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Play", exact: true }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Previous track" }).click();
  await expect(page.getByRole("heading", { name: "Soft focus" })).toBeVisible();
  const seek = page.getByRole("slider", { name: "Seek track" });
  await seek.focus();
  await seek.press("Home");
  await seek.press("ArrowRight");
  await expect(seek).toHaveValue("1000");
  await expect(seek).toHaveAttribute("aria-valuetext", "0:01 of 3:42");
  const bounds = await seek.boundingBox();
  expect(bounds).not.toBeNull();
  await seek.click({
    position: { x: bounds!.width / 2, y: bounds!.height / 2 },
  });
  const soughtPosition = Number(await seek.inputValue());
  expect(soughtPosition).toBeGreaterThan(100_000);
  expect(soughtPosition).toBeLessThan(122_000);
  await expect(page.getByText("DEMO PREVIEW")).toBeVisible();
});

for (const [state, message] of [
  ["loading", "Finding the rhythm…"],
  ["spotify_not_running", "Open Spotify to begin"],
  ["idle", "Waiting for playback"],
  ["disconnected", "Spotify disconnected"],
  ["error", "A brief interruption"],
] as const) {
  test(`${state} clears metadata and does not expose playback controls`, async ({
    page,
  }) => {
    await page.getByRole("button", { name: "Player settings" }).click();
    await page
      .getByRole("combobox", { name: "Demo playback state" })
      .selectOption(state);
    await page.getByRole("button", { name: "Back to player" }).click();
    await expect(page.getByText(message)).toBeVisible();
    await expect(page.getByRole("heading", { name: "Soft focus" })).toHaveCount(
      0,
    );
    await expect(page.getByRole("button", { name: "Next track" })).toHaveCount(
      0,
    );
    await expect(page.getByRole("progressbar")).toHaveAttribute(
      "aria-valuetext",
      "Timeline unavailable",
    );
    if (state === "disconnected" || state === "error") {
      await page.getByRole("button", { name: "Retry demo" }).click();
      await expect(
        page.getByRole("heading", { name: "Soft focus" }),
      ).toBeVisible();
    }
  });
}

for (const size of [
  { width: 320, height: 208 },
  { width: 360, height: 208 },
  { width: 480, height: 280 },
]) {
  test(`fits ${size.width} × ${size.height} without clipping`, async ({
    page,
  }) => {
    await page.setViewportSize(size);
    await page.getByRole("button", { name: "Pause", exact: true }).click();
    for (const label of [
      "Previous track",
      "Next track",
      "Play",
      "Player settings",
    ]) {
      const control = page.getByRole("button", { name: label, exact: true });
      // Chromium reports 0.99999958 for a fully visible transformed round button.
      await expect(control).toBeInViewport({ ratio: 0.99999 });
      const rect = await control.boundingBox();
      expect(rect).not.toBeNull();
      expect(rect!.x).toBeGreaterThanOrEqual(-0.01);
      expect(rect!.y).toBeGreaterThanOrEqual(-0.01);
      expect(rect!.x + rect!.width).toBeLessThanOrEqual(size.width + 0.01);
      expect(rect!.y + rect!.height).toBeLessThanOrEqual(size.height + 0.01);
    }
    const dimensions = await page.evaluate(() => ({
      width: document.documentElement.scrollWidth,
      height: document.documentElement.scrollHeight,
      viewportWidth: innerWidth,
      viewportHeight: innerHeight,
    }));
    expect(dimensions.width).toBe(dimensions.viewportWidth);
    expect(dimensions.height).toBe(dimensions.viewportHeight);
    if (size.width === 360)
      await page.screenshot({
        path: "test-results/player-preview.png",
        omitBackground: true,
      });
    await page.getByRole("button", { name: "Player settings" }).click();
    await expect(page.getByRole("combobox")).toBeInViewport({ ratio: 1 });
  });
}

test("keyboard focus returns from settings; reduced motion disables animation", async ({
  page,
}) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await expect(page.locator(".status-dot")).toHaveCSS("animation-name", "none");
  await page.getByRole("button", { name: "Player settings" }).click();
  await expect(
    page.getByRole("button", { name: "Back to player" }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: "Player settings" }),
  ).toBeFocused();
  await expect(
    page.getByRole("button", { name: "Hide player" }),
  ).toBeDisabled();
});

test("volume popup fits the minimum player and supports keyboard/mute controls", async ({
  page,
}) => {
  await page.setViewportSize({ width: 320, height: 208 });
  await page
    .getByRole("button", { name: "Spotify volume", exact: true })
    .click();
  const slider = page.getByRole("slider", { name: "Spotify volume level" });
  await expect(slider).toBeFocused();
  await expect(slider).toBeInViewport({ ratio: 1 });
  await expect(slider).toHaveValue("75");
  await expect(
    page.getByRole("group", { name: "Spotify app volume" }),
  ).toBeInViewport({ ratio: 1 });
  await page.screenshot({
    path: "test-results/volume-preview.png",
    omitBackground: true,
  });
  await slider.press("Home");
  await slider.press("ArrowRight");
  await expect(slider).toHaveValue("1");
  await page.getByRole("button", { name: "Mute Spotify", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Unmute Spotify", exact: true }),
  ).toBeVisible();
  await slider.focus();
  await slider.press("Escape");
  await expect(slider).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Spotify volume", exact: true }),
  ).toBeFocused();
  await page
    .getByRole("button", { name: "Spotify volume", exact: true })
    .click();
  await expect(slider).toBeVisible();
  await page
    .getByRole("button", { name: "Spotify volume", exact: true })
    .click();
  await expect(slider).toHaveCount(0);
});
