import { defineConfig } from "vitest/config";

export default defineConfig(({ mode }) => ({
  resolve: {
    // In the browser, use the base64 build of Loro: it embeds its WebAssembly,
    // so no bundler plugin is needed. The exact-match alias makes
    // loro-prosemirror load the same module (two copies of Loro cannot share
    // documents). Unit tests run in Node, which loads the Node build for both.
    alias: mode === "test" ? [] : [{ find: /^loro-crdt$/, replacement: "loro-crdt/base64" }],
  },
  server: {
    fs: { allow: ["../.."] }, // the page loads the ADM example from spec/
  },
  // Loro's WebAssembly is embedded as base64 (about 3 MB), hence the large chunk.
  build: { target: "es2022", chunkSizeWarningLimit: 6000 },
  test: {
    environment: "node",
    include: ["tests/**/*.test.ts"],
  },
}));
