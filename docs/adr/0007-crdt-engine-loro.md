# ADR-0007: CRDT engine — Loro, behind the DocEngine interface

- Status: accepted
- Date: 2026-10-23

## Context

The architecture planned Loro as the CRDT for note content and notebooks,
with Yrs (the Rust port of Yjs) as the fallback, subject to gate G0. The
measurements are in [docs/benchmarks/crdt-g0.md](../benchmarks/crdt-g0.md).

## Decision

**Loro stays the CRDT engine.** Gate G0 is passed.

Speed does not decide: in the structure the editor uses, both engines cost
1–2 µs per keystroke and open a 10,000-word note in about 1–2 ms, and both
handle a 10,000-note archive in about a second. The deciding factors:

- **Robustness on long texts.** A long code block or a paragraph without
  breaks is a single text. Yrs slows down with the square of its length (7 s
  for an editing session and 96 ms to open a 140 kB text); Loro stays at
  80 ms and 2 ms.
- **Features ALTAIS is designed around:** a movable tree CRDT for notebooks
  (Yrs has none; we would have to build one), history trimming with shallow
  snapshots (half the size of Yrs's compact form), checkout of old versions
  for note history, and rich-text formatting that merges concurrent
  formatting well.

## Conditions

1. **Keep the `DocEngine` interface** (`altais-docstore`) so that Yrs stays a
   possible replacement; the benchmark crate already runs both behind one
   interface.
2. **Batch typing into sync messages** (one message per pause in typing), so
   Loro's larger per-change messages (about 90 bytes against Yrs's 14) do not
   matter.
3. **Unmaintained dependencies.** Loro pulls in `im`, `bitmaps`,
   `sized-chunks` and `atomic-polyfill`, which RustSec marks unmaintained (not
   vulnerable). Before Loro becomes a dependency of the app (F1), check
   upstream for a replacement or open an issue, and list any remaining
   exceptions in `deny.toml` with a reason, as the benchmark does.
4. **Own the editor binding.** loro-prosemirror 0.4 had two cursor problems
   and Loro 1.17's `importBatch` erases undo history (both worked around in
   the editor spike). Report them upstream; if the binding stays unreliable,
   write ALTAIS's own small binding (about 1,000 lines) behind the same
   interface.

## Consequences

- F1 builds `altais-docstore` on Loro.
- Yrs is not used in the app; the benchmark keeps it measurable, and the
  comparison can be re-run when either engine releases a major version.
