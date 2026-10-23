# ALTAIS architecture

This document summarises the architecture of ALTAIS. Decisions marked
*spike* are confirmed or revised at the end of phase F0.

## Overview

ALTAIS is a modular monolith: one Rust core (`altais-core`) with clearly
separated modules, a thin UI on top of it, and one small server service. The
UI sends commands (create note, move block, complete task) and queries to the
core; the core returns view models and change events.

```
 Desktop | Mobile | Web | Clipper | CLI/MCP
 ─────────────────────────────────────────────  React + TypeScript, ProseMirror
 Bridge: Tauri commands (desktop, mobile) · WASM bindings (web, clipper)
 ─────────────────────────────────────────────
 altais-core (Rust)
   model     docstore   crypto     sync
   storage   blobstore  index      interop
   jobs      query      plugins    ai-gateway
 ─────────────────────────────────────────────
 Workers: OCR (Tesseract) · PDF (PDFium) · local AI (llama.cpp, whisper.cpp)
 Transports: ALTAIS Direct (iroh, default) · WebDAV / folder · ALTAIS Relay (optional)
```

## Processes and isolation

- **UI process** (WebView): no access to files or keys; sees only decrypted
  data handed over by the core.
- **Core process** (Tauri main process, Tokio runtime): keys in protected
  memory, database, sync.
- **Worker processes**: parse untrusted input (PDF, images, web HTML, AI
  models). A crash or exploit in a parser cannot reach the editor or the keys.

## Data model

Everything is an **object in a space**, and every object has its own CRDT
document. A per-space **manifest** document holds the notebook tree (a movable
tree CRDT, so concurrent moves never create cycles), tags, types and saved
searches. Blocks inside an object have stable IDs that survive edits, moves and
export.

The public representation is the **ALTAIS Document Model** (ADM), specified in
[`spec/`](../spec/). The CRDT document is the only editable source of truth;
ADM JSON, HTML, Markdown, plain text and the search index are derived.

Built-in object types: note, task, attachment, web clip, tag, saved search;
later journal day, person, project, event and user-defined types. A task has a
single identity: a note shows it through a `task_ref` block, so completing it
in one place completes it everywhere.

## Storage

- The archive key is random; Windows keeps it so the archive opens without
  a password, an optional password replaces that copy, and a recovery key
  always works ([ADR-0008](adr/0008-archive-key-protection.md)).
- One SQLCipher database per device: objects (CRDT snapshots), outbox, sync
  cursors, blob index, link and task views, FTS5 full-text index and
  sqlite-vec vectors. The index is therefore encrypted at rest for free.
- Search: an FTS5 index inside the encrypted database, holding no readable
  copy of the text. Titles and bodies are folded before indexing and the
  query the same way: lowercase, diacritics removed (also ł, đ, ø, ß), Russian
  ё as е, and every Chinese or Japanese character as its own word, so a
  two-character word is found inside a sentence. Words match as prefixes;
  quoted text and Chinese words match as written; the title counts five times
  as much as the body.
- Attachments: content-addressed (BLAKE3), split with FastCDC, encrypted with
  a random per-file key. Originals are never modified.
- A local edit, its encrypted envelope in the outbox and the index dirty flag
  are written in **one transaction**. "Saved on this device" is shown only
  after the commit.

## Sync

Sync is an **end-to-end encrypted log of changes per space** plus encrypted
snapshots. The server assigns sequence numbers, stores and forwards; it never
learns which object a change belongs to.

Envelope fields visible to the server: protocol version, space ID, change ID
(UUIDv7, for idempotency), device ID, key epoch, nonce, Ed25519 signature.
Encrypted: object ID, change kind and the Loro update (zstd-compressed). The
header is bound as AAD.

Flow: local transaction → `PUSH` → Relay assigns `seq`, fsyncs, `ACK` →
`NOTIFY` other devices → `PULL since=<cursor>` → verify, decrypt, merge (Loro
merges are commutative and idempotent) → cursor saved in the same transaction.
Attachments transfer separately in resumable chunks. Trusted devices upload
shallow encrypted snapshots; old changes are compacted after a retention
window.

The flow above is shown for a server; the same log travels over every
transport (see [ADR-0002](adr/0002-sync-transports.md)):

- **ALTAIS Direct** (default): devices connect directly by public key over the
  internet or the local network (iroh 1.0, NAT traversal, encrypted relay
  fallback). No account, cloud or server.
- **Always-on node**: a home PC, NAS or Raspberry Pi acting as a sync peer.
- **WebDAV mailbox and shared folder**: every device writes only to its own
  subfolder of immutable log segments, so file conflicts cannot occur. Works
  with Koofr, Nextcloud, Synology, Syncthing, OneDrive or a USB drive.
- **Server-based sync** (planned, F5): its form (self-hosted ALTAIS Relay,
  hosted service or storage server) is decided later; the architecture keeps
  it possible from the start (ADR-0002).

Changes for the note open in the editor wait in a local inbox and are applied
on **Update now**, after a pause in typing, or when the program closes or
opens, so text never moves under the cursor; other notes take changes
immediately (see [ADR-0006](adr/0006-incoming-changes-wait-for-the-open-note.md)).

Relays and storage providers see only metadata (IP addresses, timing, sizes),
never content. No blockchain is used; see [ADR-0003](adr/0003-no-blockchain.md).

| Situation | Resolution |
| --- | --- |
| Same note, concurrent text edits | Loro rich-text CRDT; both versions in history |
| Concurrent notebook moves | Movable tree CRDT; always a tree without cycles |
| Title, due date, task status | Last writer by logical clock (not wall clock); concurrent value flagged |
| Delete vs concurrent edit | Object stays in trash with the edit; restorable |
| Two versions of an attachment | Both kept as separate immutable versions |
| Membership and permissions | Not a CRDT: a signed linear chain verified by clients and the Relay |

## Languages

UI languages: Slovak, Czech, English, Hungarian, Polish, Russian and
Simplified Chinese (Traditional Chinese later), with Project Fluent messages
and ICU4X for dates, numbers and sorting. Search folds diacritics and letters
such as Polish `ł`, so queries without diacritics still match; Chinese is
indexed with the FTS5 trigram tokenizer. Each note may carry a BCP 47 `lang`
tag that selects tokenization, OCR language and spell checking. Details:
[ADR-0004](adr/0004-languages-and-localisation.md).

## Cryptography

- Account root key: 256 random bits created on the device; wrapped by a
  password-derived key (Argon2id), by a recovery code, or transferred to a new
  device by QR pairing.
- Space key per epoch, wrapped to each member with hybrid X-Wing
  (X25519 + ML-KEM-768); rotated when a member is removed.
- Derived via HKDF: operation key (XChaCha20-Poly1305), file keys, keyed
  BLAKE3 deduplication key, and a write-only **capture key** (HPKE, RFC 9180)
  that lets a clipper or e-mail gateway add items without being able to read
  the archive.
- Libraries: RustCrypto (`chacha20poly1305`, `x25519-dalek`, `ed25519-dalek`,
  `ml-kem`, `argon2`, `hkdf`), `blake3`, `hpke`. No custom primitives. An
  independent audit is required before release 1.0.

## Technology and licence policy

All required components use MIT, Apache-2.0, BSD, ISC, Zlib, Unicode,
public-domain or MPL-2.0 licences. No AGPL, GPL or non-commercial dependency is
allowed on the required path; CI enforces this with `cargo-deny`
(see [`deny.toml`](../deny.toml)).

| Layer | Choice | Fallback |
| --- | --- | --- |
| Core and server | Rust ([ADR-0001](adr/0001-rust-core-with-web-ui.md)) | — |
| Device-to-device sync | iroh 1.0 | WebDAV mailbox |
| Desktop and mobile shell | Tauri 2 | Electron |
| UI | React + TypeScript | SolidJS |
| Editor | ProseMirror with an ALTAIS schema | Lexical |
| CRDT | Loro (gate G0 passed, [ADR-0007](adr/0007-crdt-engine-loro.md)) | Yrs |
| Local database | SQLite + SQLCipher, FTS5, sqlite-vec | Tantivy for search |
| Wire format | CBOR (RFC 8949) with a CDDL schema | Protocol Buffers |
| PDF / OCR | PDF.js, PDFium, Tesseract 5 | ONNX Runtime + PaddleOCR models |
| Plugins | Wasmtime, WebAssembly component model | — |

Avoided because of licences: MuPDF, Poppler and Ghostscript (AGPL/GPL),
SingleFile (AGPL), MinIO (AGPL), Redis (RSAL/SSPL/AGPL), Sentry self-hosted
(FSL), Tiptap Pro, CKEditor 5 and TinyMCE 7+, Llama and Gemma model weights.

## Evernote import

`altais-interop` streams an ENEX file note by note and converts ENML into ADM
blocks: headings, paragraphs with inline marks, bulleted, numbered and check
lists (Evernote 11 `--en-todo` lists and legacy `<en-todo>`), tables, quotes,
code blocks, rich links as bookmarks, and attachments matched by their MD5 hash
and stored under their BLAKE3 hash. Every note is accounted for in a report:

- **imported**, **partial** (some content kept only in raw form, such as a
  damaged body or a missing attachment), or the import stops with an error that
  says after how many notes;
- encrypted text stays locked as an `evernote_encrypted` block; Evernote tasks,
  note attributes and resource metadata are kept under `x-evernote`;
- presentational styles (fonts, sizes) and hidden editor settings are ignored
  and reported; no text is dropped.

## Testing strategy

- Unit and property-based tests (`proptest`).
- A **deterministic network and disk simulator** (`altais-sim`): thousands of
  random scenarios with crashes, outages, duplicates and reordering must end
  with identical state on every device and no acknowledged edit lost. Every
  failure is reproducible from its seed.
- Fuzzing of the ENEX importer and the sync protocol (`cargo-fuzz`).
- UI tests with Playwright on Windows and Linux.

## Planned crates

```
crates/
  altais-model/     ADM types and validation            (F0, present)
  altais-crypto/    keys, envelopes, X-Wing, HPKE        (F3)
  altais-docstore/  Loro adapter behind a DocEngine trait (F1)
  altais-crdt-bench/ gate G0: Loro vs Yrs, not shipped   (F0, present)
  altais-storage/   SQLCipher, key protection, schema    (F1, present)
  altais-blob/      FastCDC, encrypted chunks, integrity (F1)
  altais-index/     sqlite-vec, query language           (F2; FTS5 search is in altais-storage)
  altais-sync/      outbox, inbox, cursors, transports   (F3)
  altais-interop/   ENEX import (F0, present); ADM/HTML/Markdown export (F2)
  altais-core/      facade used by the app and the CLI   (F1, present)
  altais-relay/     optional self-hosted server (Axum)   (F5)
  altais-sim/       deterministic simulator              (F3)
  altais-cli/       `altais` tool: init, Evernote import, search (F1, present)
apps/
  editor-spike/     ProseMirror + Loro prototype         (F0, present)
  desktop/          Tauri 2 + React, Windows installer   (F1, present)
  clipper/          browser extension, core in WASM      (F4)
```
