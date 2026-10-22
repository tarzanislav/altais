import { defineConfig, devices } from "@playwright/test";

// Node global, declared here so the spike needs no @types/node.
declare const process: { env: Record<string, string | undefined> };

// PW_CHROMIUM_PATH lets a machine with a preinstalled Chromium skip the download.
const executablePath = process.env.PW_CHROMIUM_PATH || undefined;

export default defineConfig({
  testDir: "e2e",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["github"], ["list"]] : "list",
  use: {
    baseURL: "http://127.0.0.1:5179",
    trace: "retain-on-failure",
  },
  projects: [{ name: "chromium", use: { ...devices["Desktop Chrome"], launchOptions: { executablePath } } }],
  webServer: {
    // Test the production build. The dev server optimises dependencies on
    // first use and then reloads the page, which wipes input mid-test.
    command: "npx vite build && npx vite preview --host 127.0.0.1 --port 5179 --strictPort",
    url: "http://127.0.0.1:5179",
    reuseExistingServer: !process.env.CI,
    timeout: 60_000,
  },
});
