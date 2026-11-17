import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  testMatch: ["player.spec.ts", "control-preservation.spec.ts", "ambient-palette.spec.ts"],
  fullyParallel: true,
  workers: 2,
  use: {
    channel: "msedge",
    baseURL: "http://localhost:1422",
    viewport: { width: 360, height: 208 },
  },
  webServer: {
    command: "npm run dev -- --port 1422",
    url: "http://localhost:1422",
    env: { VITE_MOCK_MEDIA: "true" },
    reuseExistingServer: false,
  },
});
