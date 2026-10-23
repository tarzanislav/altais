import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// Tauri serves the built files; the dev server is used by `tauri dev` and tests.
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    fs: { allow: ["../.."] }, // the in-memory backend loads an ADM example from spec/
  },
  build: { target: "es2022" },
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
});
