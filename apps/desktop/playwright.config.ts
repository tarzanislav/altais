import { defineConfig, devices } from "@playwright/test";

declare const process: { env: Record<string, string | undefined> };
const executablePath = process.env.PW_CHROMIUM_PATH || undefined;

// The interface runs in a plain browser with the in-memory backend (src/api.ts).
export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["list"]] : "list",
  use: { baseURL: "http://127.0.0.1:5180", locale: "sk-SK", trace: "retain-on-failure" },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], locale: "sk-SK", launchOptions: { executablePath } } }],
  webServer: {
    command: "npx vite build && npx vite preview --host 127.0.0.1 --port 5180 --strictPort",
    url: "http://127.0.0.1:5180",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
