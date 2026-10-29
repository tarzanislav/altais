# ALTAIS desktop app

The Windows (and later Linux) application: a Tauri 2 window around a React
interface. Every operation is handled by `altais-core` in Rust; the interface
only shows and edits.

What it does now:

- first run: creates the encrypted archive (key kept by Windows, optional
  password) and shows the recovery key, which must be confirmed as saved;
- opens the archive (automatically, or with the password / recovery key);
- notebooks with note counts, all notes, new notebooks;
- search that ignores case and diacritics (also Chinese);
- the editor from the editor spike: formatting, headings, lists, checklists,
  tables, code, Word paste; notes save themselves 0.6 s after the last change;
- import of Evernote exports (`.enex`, several at once, one notebook per file)
  with a report of what was imported only partly;
- attachments: pictures are shown in the note, other files as a card; every
  attachment can be saved with "Save as…", and files can be added with 📎;
- moving notes to the trash;
- settings: set, change or remove the password, and create a new recovery
  key (the old one stops working; the new one must be confirmed as saved).

The archive lives in `%APPDATA%\io.github.tarzanislav.altais\archive`.

## Getting the installer

Every pull request and every change on `main` builds a Windows installer in
CI: open the run under **Actions**, then download **altais-windows-installer**
under *Artifacts*. The installer is not signed yet, so Windows SmartScreen
asks for confirmation ("More info" → "Run anyway").

## Development

```sh
npm ci
npm run dev          # the interface in a browser, with an in-memory stand-in backend
npm test             # unit tests
npm run test:e2e     # browser tests of the interface (Playwright)
npx tauri dev        # the real app (needs Rust; on Linux the WebKitGTK libraries)
npx tauri build      # installer
```

`src-tauri` is a separate Cargo workspace: Tauri needs Rust 1.90 and the
system WebView libraries, which the core crates must not depend on.
