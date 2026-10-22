# Demo and test data catalog

ALTAIS ships two related data sets. Both must cover **every kind of data that
note-taking software can hold**, so that users can try every feature and
developers can test it.

| Set | Audience | Size | Where |
| --- | --- | --- | --- |
| **Demo package** | Users trying ALTAIS (see [ADR-0005](adr/0005-demo-data.md)) | about 80 notes, at most 20 MB per language | `resources/demo/altais-demo-<lang>.altaispack` |
| **Test corpus** | Automated tests, CI, performance and import tests | everything in the demo plus edge cases, stress data and import fixtures | `testdata/` (large files generated at test time, never committed) |

Columns below: **Demo** = included in the demo package; **Tests** = included in
the test corpus; **Phase** = when the feature that uses the data is built. The
demo package is built for each release and contains a data type only once the
feature exists, so users never see content the application cannot show yet.

## 1. Text and formatting

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Paragraphs, headings H1–H4 | yes | yes | F1 |
| Bulleted, numbered and nested lists (3 levels) | yes | yes | F1 |
| Checklists (each item a task object) | yes | yes | F2 |
| Quote, callout, divider | yes | yes | F1 |
| Code blocks in several languages (Rust, Python, SQL, JSON) | yes | yes | F1 |
| Tables, including merged cells and wide tables | yes | yes | F1, merged cells P1 |
| Collapsible (toggle) blocks | yes | yes | P1 |
| Mathematical formulas | yes | yes | P1 |
| Diagrams (Mermaid source) | yes | yes | P1 |
| Inline marks: bold, italic, underline, strikethrough, code, highlight, text colour, link | yes | yes | F1 |
| Emoji, combining characters, special symbols | yes | yes | F1 |
| Empty note, note with only a title | yes | yes | F1 |
| Very long note (5,000 blocks) | — | yes | F1 |

## 2. Links and references

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Link to another note | yes | yes | F1 |
| Link to a specific block | yes | yes | F1 |
| Backlinks (a note linked from several others) | yes | yes | F1 |
| Mentions of objects (`@` person, project) | yes | yes | P1 |
| Date chips in text | yes | yes | F2 |
| External web links and bookmark cards (no network access needed) | yes | yes | F1 |
| Broken link (target deleted) | — | yes | F1 |

## 3. Organisation

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Nested notebooks (3 levels) | yes | yes | F1 |
| Notebook stacks (from Evernote import) | — | yes | F1 |
| Tags, including nested tags and colours | yes | yes | F1 |
| Favourites and pinned notes | yes | yes | F2 |
| Saved searches using every query operator | yes | yes | F1 |
| Inbox items not yet sorted | yes | yes | F2 |
| Items in the trash, including one that can be restored | yes | yes | F2 |
| Archived notebook | yes | yes | F2 |
| Templates | yes | yes | P1 |
| 1,000 tags, 20 levels of nesting | — | yes | F1 |

## 4. Tasks and time

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Due date without time (civil date) | yes | yes | F2 |
| Due date and time with a time zone | yes | yes | F2 |
| Overdue, due today, upcoming, completed | yes | yes | F2 |
| Priorities | yes | yes | F2 |
| Reminders | yes | yes | F2 |
| Recurring tasks (fixed calendar and after completion) | yes | yes | P1 |
| Subtasks | yes | yes | P1 |
| Task shown inside a note (`task_ref`) and task without a note | yes | yes | F2 |
| Journal day | yes | yes | P1 |
| Calendar events | yes | yes | P2 |
| Tasks across a daylight-saving change and in other time zones | — | yes | F2 |

All demo dates are computed relative to the day the demo is loaded.

## 5. Attachments by file type

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| PDF with a text layer, multi-page | yes | yes | F1 |
| Scanned PDF without text (OCR) | yes | yes | F1 |
| Images: JPEG, PNG, WebP, GIF | yes | yes | F1 |
| Images: HEIC (phones), SVG (sanitised before display) | yes | yes | F1 |
| Photo with EXIF data, including location | yes | yes | F1 |
| Audio: MP3, M4A, OGG, WAV; recording with transcript | yes | yes | F1, transcript P1 |
| Video: MP4, WebM | yes | yes | F1 |
| Office documents: DOCX, XLSX, PPTX | yes | yes | F1 |
| OpenDocument: ODT, ODS | yes | yes | F1 |
| Plain text: TXT, Markdown, CSV, JSON, XML | yes | yes | F1 |
| E-mail (EML) with its own attachment | yes | yes | F1 |
| Contact card (VCF) and calendar file (ICS) | yes | yes | F1 |
| E-book (EPUB) and archive (ZIP) | yes | yes | F1 |
| Unknown binary file | yes | yes | F1 |
| Two versions of the same file; identical files (deduplication) | yes | yes | F1 |
| Large files (100 MB, 1 GB), generated at test time | — | yes | F1 |

## 6. Captured content

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Web article (cleaned, readable version) | yes | yes | F4 |
| Full-page snapshot, selection, screenshot, bookmark only | yes | yes | F4 |
| Item added with the write-only capture key | yes | yes | F4 |
| E-mail capture | yes | yes | P1 |
| Item shared from a phone | yes | yes | F5 |

## 7. Object types and properties

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Note, task, attachment, web clip, tag, saved search | yes | yes | F1–F2 |
| Person, project, place (with coordinates) | yes | yes | P1–P2 |
| Custom type with properties of every kind: text, number, date, date and time, single choice, multiple choice, checkbox, URL, e-mail, phone, rating, relation | yes | yes | P2 |
| Provenance of every kind: manual, web, e-mail, import, AI, capture | yes | yes | F1 |

## 8. History, sync and archive health

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Note with several revisions and one restored revision | yes | yes | F2 |
| Concurrent edits that were merged, with both versions in history | yes | yes | F3 |
| Concurrent change of a due date, flagged for review | yes | yes | F3 |
| Attachment not yet downloaded, upload pending | yes (status only) | yes | F3 |
| OCR pending, import warning, integrity check result | yes | yes | F4 |
| Simulated crashes, outages, duplicates and reordering | — | yes | F3 |

## 9. Languages and scripts

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Notes in Slovak, Czech, English, Hungarian, Polish, Russian, Chinese (Simplified and Traditional) | yes | yes | F1 |
| Search without diacritics: `ľ ť ä ô`, `ř ů`, `ő ű`, `ł ż ś` | yes | yes | F1 |
| Chinese text without spaces | yes | yes | F1 |
| Note mixing several languages | yes | yes | F1 |
| Right-to-left text (Arabic, Hebrew), for future layouts | — | yes | F2 |
| Very long words, unusual Unicode, invalid UTF-8 in imports | — | yes | F1 |

## 10. Security and isolation

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Locked space with its own password | — | yes | F2 |
| Encrypted text from Evernote, imported as a locked block | — | yes | F1 |
| Demo space marked local-only; removal leaves nothing behind | yes | yes | F2 |
| Hostile input: scripts in HTML, malicious SVG, path traversal in file names, zip bomb, oversized image dimensions | — | yes | F1 |

## 11. Import fixtures and compatibility

| Data type | Demo | Tests | Phase |
| --- | --- | --- | --- |
| ENEX samples covering Evernote tables, to-dos, media, encrypted text, reminders and tasks | — | yes | F0–F1 |
| Malformed and truncated ENEX files | — | yes | F1 |
| Markdown folders (Obsidian and Joplin style), HTML export | — | yes | P1 |
| Unknown block type, unknown object type, unknown fields (preserved) | — | yes | F0 |
| ADM document from a newer minor version | — | yes | F0 |

## 12. Scale

| Data set | Demo | Tests | Phase |
| --- | --- | --- | --- |
| Reference archive: 10,000 notes, about 1 GB of text and 10 GB of attachments | — | generated | F1 |
| Stress archive: 50,000 notes | — | generated | F1 |

## Rules for all demo and test data

1. **Original and free.** Every text, image, audio and video file is created
   for ALTAIS and released under CC0. No third-party works, no stock media.
2. **No personal data.** Names, e-mail addresses and phone numbers are
   fictional and use reserved domains such as `example.com` or `example.sk`.
3. **Reproducible.** Media files are produced by scripts in the repository, so
   anyone can rebuild them; large and stress data sets are generated at test
   time and never committed.
4. **Small for users.** Media are shared between language packages; only texts
   are translated. Each demo package stays under 20 MB.
5. **Safe to open.** Hostile samples exist only in the test corpus and are
   never part of the demo package.
