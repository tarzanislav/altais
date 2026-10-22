# Editor spike (phase F0)

A throwaway prototype that answers one question before the real editor is
built: **can ProseMirror + Loro hold every ADM block type, merge offline edits
from two devices and give each device its own undo?**

The page shows two editors ("device A" and "device B"). Each has its own Loro
document; they exchange Loro updates. Untick **Online** to hold the updates
back, edit both sides, then tick it again to watch the edits merge.

```sh
npm ci
npm run dev          # http://localhost:5173
npm test             # unit tests (Node)
npm run test:e2e     # browser tests (Playwright, Chromium)
npm run licenses     # licence allowlist check
```

On a machine that already has Chromium, `PW_CHROMIUM_PATH=/path/to/chrome`
skips the Playwright browser download.

## What is in it

| File | Purpose |
| --- | --- |
| `src/schema.ts` | ProseMirror schema mirroring ADM 0.1: paragraph, heading, list (bullet/ordered/check), table, quote, code, divider, bookmark, attachment, plus `unknown` for blocks the editor cannot show yet (kept verbatim) |
| `src/adm.ts` | ADM ⇄ ProseMirror conversion |
| `src/bids.ts` | Keeps every block ID present and unique (after Enter, paste, merge) |
| `src/paste.ts` | Turns Microsoft Word list paragraphs into real lists and drops Office markup |
| `src/editor.ts` | Editor setup: Loro sync and undo, shortcuts, checklist ticking, cursor kept on remote changes |
| `src/main.ts` | The two-device demo page |
| `tests/` | Round trip of `spec/adm/examples/*.json`, JSON Schema validation, block IDs, Loro merge, cursor on remote changes |
| `e2e/` | Typing, Enter, bold, checklist, offline merge, undo, Word paste, cursor stability |

## Findings

- **It works.** Both ADM examples round-trip unchanged through ProseMirror
  *and* through Loro, the output passes the ADM JSON Schema, concurrent
  offline edits merge to the same note on both devices, and undo only reverts
  the local device's change.
- **Plugin order matters.** The block-ID plugin must run before
  `LoroSyncPlugin`; otherwise the other device receives a block without an
  ID, assigns its own, and the two devices race (this broke the cursor).
- **`loro-prosemirror` is young (0.4.x).** It applies every remote change by
  replacing the whole ProseMirror document and then restores the cursor from
  a Loro cursor. That lookup fails when the remote change touched the
  paragraph holding the cursor (it clears its cache first, then looks the
  cursor up; the console shows `Cannot find the loroNode`). In Chromium 156
  the cursor then jumped to the start of the note. The spike fixes this on
  our side (`keepCursorOnRemoteChange` in `src/editor.ts`: compare the old
  and new document and move the cursor by the size of the change), with unit
  and browser tests. The binding is small (about 1,000 lines, MIT), so the
  real fix belongs upstream or in our own binding behind the same interface.
- **Known issue: a click right after a remote change can be undone.**
  loro-prosemirror restores the cursor in a timer after each remote change;
  if the user clicks in between, the timer moves the cursor back. With a
  collaborator typing continuously this would be noticeable. The browser
  tests wait for those timers before clicking. Fix: drop the timer restore
  (our cursor plugin already covers it), which needs a change in the binding.
- **Browser tests run against the production build.** The Vite dev server
  optimises dependencies on first use and reloads the page, which wiped
  typed text in CI.
- **Size.** The browser build embeds Loro's WebAssembly as base64: 5.1 MB of
  JavaScript, 1.8 MB gzipped. Fine inside the desktop app (loaded from disk);
  the web version should load the `.wasm` file separately.
- **Line breaks** inside a block are stored as `\n` in the ADM span text, with
  the formatting of the text around them.

## Not covered yet

Table editing commands (only display and Tab navigation), images inline,
`task_ref`, input methods for Chinese (IME), mobile, and performance on long
notes. Performance is the next step: the Loro vs Yrs benchmark (gate G0).

## Libraries

All runtime packages use the MIT licence; `npm run licenses` checks the whole
tree (103 packages) against the same allowlist as `deny.toml`.
