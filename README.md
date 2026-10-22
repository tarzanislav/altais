# ALTAIS

**An open-source, local-first, end-to-end encrypted alternative to Evernote.**

ALTAIS is a personal archive for notes, documents, web clippings and tasks
that you can trust for decades: it works fully offline, syncs between your
devices without the server ever seeing your content, and keeps your data in an
open, documented format that stays readable without the application.

> **Status:** design and foundation phase (F0). There is no usable application
> yet. See the [roadmap](#roadmap).

## Why another notes app?

No existing application combines all four of these:

| | Evernote | Joplin | Notesnook | Obsidian | ALTAIS (goal) |
| --- | --- | --- | --- | --- | --- |
| Rich editor, attachments and OCR | yes | partly | partly | partly | yes |
| End-to-end encryption by default | no | optional | yes | paid sync only | yes |
| Client **and** server open source, free for commercial use | no | client only | yes | no | yes |
| Sync without depending on a vendor | no | yes | partly | yes | yes |

## Principles

1. **The local copy is the truth; the server is a courier.** It stores and
   forwards encrypted bytes and never decides anything about content.
2. **Originals are untouchable.** Attachments are stored bit for bit. Previews,
   OCR text, indexes and embeddings are derived and can always be rebuilt.
3. **One core, many surfaces.** Desktop, mobile, web, clipper and CLI share
   the same Rust core. No data logic is written twice.
4. **Security without a convenience penalty.** Encryption is invisible.
   Key recovery is set up when the archive is created, not after a loss.
5. **Every claim is verifiable.** "Saved", "synced" and "backed up" have a
   measurable meaning and evidence (checksums, server acknowledgement, test restore).
6. **The format outlives the app.** The data format and sync protocol are
   public domain (CC0); exports open in any web browser.
7. **Simple by default, powerful on demand.** Notes, notebooks and tasks first;
   object types, views and AI only when switched on.
8. **No paid dependency on the required path.** No core feature needs a paid
   licence, a paid API or a specific cloud provider.

## Architecture at a glance

- **`altais-core` (Rust)** — one core for all platforms: data model, CRDT
  documents (Loro), encrypted storage (SQLite + SQLCipher), full-text search
  (FTS5), attachments, sync engine, import and export.
- **UI** — React + TypeScript with a ProseMirror editor, packaged with Tauri 2
  for desktop and mobile; the core compiles to WebAssembly for the web client
  and the browser clipper.
- **Sync** — an end-to-end encrypted append-only log per space plus encrypted
  snapshots. By default devices sync **directly with each other** (no account,
  no cloud, no server); the same log can also travel through a WebDAV mailbox,
  a shared folder (Syncthing, OneDrive, USB drive) or an optional self-hosted
  ALTAIS Relay.
- **Cryptography** — XChaCha20-Poly1305, Ed25519 device signatures, hybrid
  post-quantum key wrapping (X25519 + ML-KEM-768), a write-only capture key for
  clippers and e-mail, BLAKE3 content addressing.

More detail: [`docs/architecture.md`](docs/architecture.md) and the
architecture decision records in [`docs/adr/`](docs/adr/).

## Roadmap

| Phase | Content | Gate |
| --- | --- | --- |
| **F0 Foundation** | Specifications, repository, CI, editor spike, Evernote ENEX parser | 20 real notes imported without loss |
| F1 Local archive | Full ENEX import, notebooks, full-text search, OCR, attachments | Whole archive accounted for; search under 500 ms |
| F2 Daily work | Tasks, history, trash, quick capture, export, backups | Four weeks of daily use without Evernote |
| F3 Sync | Keys, direct device-to-device sync, WebDAV and folder sync, device pairing, network simulator | One million simulated steps without divergence |
| F4 Release 1.0 | Web clipper, capture key, archive health view, accessibility, audit | All acceptance criteria met; audit without critical findings |
| F5 Beyond | Android, local AI with citations, shared spaces, self-hosted Relay, web client, plugins | per module |

Windows is the first platform; Linux builds run in CI from the start;
Android follows release 1.0, iOS after it.

## Repository layout

```
crates/altais-model/   ALTAIS Document Model: types and validation
spec/                  public specifications (CC0): ADM schema and examples
docs/                  architecture and design documents
```

## Building

Requires a stable Rust toolchain (see `rust-toolchain.toml`).

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

## Contributing

Contributions are welcome. Please read [`CONTRIBUTING.md`](CONTRIBUTING.md);
every commit needs a `Signed-off-by` line (Developer Certificate of Origin).
Report security issues privately as described in [`SECURITY.md`](SECURITY.md).

## License

- Code: [Apache License 2.0](LICENSE).
- Specifications in [`spec/`](spec/): public domain, [CC0 1.0](spec/LICENSE.md).
- The ALTAIS name and logo: see [`TRADEMARKS.md`](TRADEMARKS.md).

---

### Po slovensky

ALTAIS je otvorená alternatíva k Evernote: lokálny, koncovo šifrovaný archív
poznámok, dokumentov a úloh. Funguje bez internetu, synchronizuje sa medzi
zariadeniami bez toho, aby server videl obsah, a údaje ukladá v otvorenom
formáte, ktorý sa dá čítať aj bez aplikácie. Kód je pod licenciou Apache 2.0,
špecifikácie formátu sú voľné dielo (CC0). Projekt je zatiaľ vo fáze návrhu.
