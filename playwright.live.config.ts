import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  testMatch: ["native-media.spec.ts", "ambient-ui.spec.ts", "phase4-ui.spec.ts"],
  workers: 2,
  use: {
    channel: "msedge",
    baseURL: "http://localhost:1423",
    viewport: { width: 360, height: 208 },
  },
  webServer: {
    command: "npm run dev -- --port 1423",
    url: "http://localhost:1423",
    env: { VITE_MOCK_MEDIA: "false" },
    reuseExistingServer: false,
  },
});
