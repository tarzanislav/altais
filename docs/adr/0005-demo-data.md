# ADR-0005: Demo data in an isolated, removable space

- Status: accepted
- Date: 2026-10-22

## Context

New users should be able to try ALTAIS with realistic content before they
commit their own notes. Afterwards the demo must disappear completely: no demo
note, tag, task, attachment, search entry, history entry or sync record may
remain among the user's own data. The original demo content should stay
available in the installation folder so it can be loaded again.

## Decision

### First run

On the first start, ALTAIS offers three choices:

1. **Try ALTAIS with demo data**
2. **Start with an empty archive**
3. **Import from Evernote** (ENEX)

The same "load demo data" action is available later in Settings.

### Isolation

- Demo data lives in its own **demo space**, stored in a **separate database
  file and blob folder** with its own space key. It never shares the user's
  database, search index or attachment store.
- The demo space is **local only**: it is never synced, never included in the
  user's backups or exports, and never offered to AI features or MCP clients.
- The demo space is clearly marked in the interface (a "Demo data" banner and
  colour), so demo content cannot be mistaken for the user's own.
- Content cannot be moved from the demo space into a user space. This keeps
  the guarantee that removing the demo leaves nothing behind.

### Removal

"Remove demo data" closes the demo space and deletes its database file, blob
folder, thumbnails, OCR results and logs. ALTAIS then verifies the result and
shows it: no file of the demo space remains in the data folder, and no record
in the user's spaces refers to the demo space. Removal is final and needs one
confirmation.

### Where the demo content comes from

- The demo content is a read-only package in the installation folder, for
  example `resources/demo/altais-demo-sk.altaispack` (ADM JSON plus attachment
  blobs, with a manifest and checksums). One package per UI language: `sk`,
  `cs`, `en`, `hu`, `pl`, `ru`, `zh-Hans`.
- Loading the demo copies the package into a new demo space; the package
  itself is never modified, so the demo can be loaded again at any time.
- All demo texts, images and PDFs are written for ALTAIS and released under
  CC0, so the package contains no third-party copyrighted material and no
  personal data.

### Content of the demo package

The demo package and the larger test corpus cover every kind of data that
note-taking software can hold; the full list is in the
[demo and test data catalog](../demo-data-catalog.md). The demo package has
about 80 notes in a few notebooks. Highlights:

- notes with headings, lists, tables, links between notes and blocks;
- tasks with due dates computed relative to the day the demo is loaded, so the
  "Today" and "Upcoming" views are never empty or out of date;
- tags, a saved search and a few inbox items;
- a web clipping, an image, and a scanned PDF with OCR text that search can
  find;
- short notes in several languages, including Chinese, to demonstrate search
  without diacritics and search in text without spaces;
- an example of the archive health view (one attachment marked as not yet
  downloaded).

## Consequences

- The same packages serve as test data for automated UI tests and for
  documentation screenshots.
- Tests must prove the removal guarantee: load the demo, use it, remove it,
  then assert that the data folder contains no demo files and that no record
  refers to the demo space.
- Phases: the package format and the English and Slovak packages in F1 (they
  are needed for tests); the first-run dialog, the other languages and removal
  in F2.
