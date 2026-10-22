# Gate G0: Loro vs Yrs

Measured 2026-10-23 with `crates/altais-crdt-bench`. Reproduce with:

```sh
cargo run --release -p altais-crdt-bench --bin crdt-report   # this table (about 2 minutes)
cargo bench -p altais-crdt-bench                             # statistical timings of key operations
```

Both engines run the same deterministic edit traces with words in all
interface languages (Slovak, Czech, Hungarian, Polish, Russian, Chinese,
English). Tests check that both end with the same text as a plain-string
reference, keep it through snapshots and converge after concurrent edits.
Machine: a cloud VM; a desktop PC is likely faster, the ratios matter.

## Results

> Workload: long note of 10000 words typed keystroke by keystroke (64087 keystrokes), then 20000 edits
> (70 % typing a word, 20 % deleting, 10 % bold); offline: 2000 edits per device;
> archive: 10000 notes of 30–500 words. Median of 5 runs. CPU: Intel(R) Xeon(R) Processor @ 2.80GHz.

| Measurement | Loro 1.17.1 | Yrs 0.28.0 |
| --- | ---: | ---: |
| **The note as one long text** | | |
| Typing a long note, keystroke by keystroke | 93.2 ms | 9727 ms |
| … keystrokes per second | 688 k/s | 7 k/s |
| Editing session on it | 80.4 ms | 7226 ms |
| Text of the note (UTF-8) | 140.5 kB | 140.5 kB |
| Stored note with history (snapshot) | 384.6 kB | 485.2 kB |
| Compact form (Loro: shallow snapshot; Yrs: v2) | 158.5 kB | 325.5 kB |
| Opening the note (load + read text) | 1.7 ms | 95.9 ms |
| Saving the note (export snapshot) | 356 µs | 2.5 ms |
| One keystroke as a sync message | 89 B | 14 B |
| Offline edits of one device as a sync message | 27.0 kB | 56.1 kB |
| Merging those edits | 116 ms | 31.1 ms |
| Importing an archive (create + save each note) | 1178 ms | 394 ms |
| Archive on disk | 23.5 MB (1.15× text) | 21.5 MB (1.05× text) |
| Opening every note of the archive | 524 ms | 213 ms |
| **The same note as paragraphs (how the editor stores it)** | | |
| Typing the long note (one text per paragraph) | 111 ms | 64.1 ms |
| Stored note with history (snapshot) | 77.8 kB | 82.6 kB |
| Opening the note (load + read text) | 1.6 ms | 823 µs |

"One long text" puts the whole note into a single text, which is what a long
code block or a long paragraph without breaks becomes. "Paragraphs" is how the
editor stores a note (one text per paragraph, as loro-prosemirror and
y-prosemirror do). The archive notes are created the way the Evernote import
creates them: one insert per note, then a few edits.

Yrs runs with UTF-16 positions (needed for non-ASCII text). With byte
positions its single-text costs halve, but they still grow with the square of
the text length (4× the text ≈ 15× the time), so the gap to Loro stays large.

Per-keystroke sync messages: Yrs sends what the transaction changed (its
update event); Loro exports the changes since the previous version. Both would
be wrapped in an encrypted envelope of about 100 bytes, and ALTAIS batches
typing into one message per pause, so the difference does not matter in
practice.

## Interpretation

- **Both are fast enough for ALTAIS.** In the structure the editor uses,
  typing a 10,000-word note costs about 1–2 µs per keystroke in either engine,
  and opening it about 1–2 ms. A 10,000-note archive imports in about a second
  and opens completely in under 0.6 s with either.
- **Yrs** is about 2× faster in the paragraph structure, 3× faster at
  importing and opening the archive, 4× faster at merging, slightly smaller on
  disk for imported notes, and sends tiny per-keystroke messages.
- **Loro** is 100× faster in a single long text (Yrs grows quadratically:
  7 s for an editing session in a 140 kB text, 96 ms to open it), its compact
  form without history is half the size, and it has features ALTAIS planned
  around: a movable tree CRDT for notebooks, history trimming (shallow
  snapshots), checkout of old versions, and rich-text formatting that merges
  concurrent formatting well.
- **Maintenance signals.** Loro pulls in four crates marked unmaintained by
  RustSec (`im`, `bitmaps`, `sized-chunks`, and `atomic-polyfill` via
  `postcard`); Yrs pulls in one (`smallstr`). None is a vulnerability. The
  Yrs 0.28 also does not build with ALTAIS's minimum Rust version (1.88)
  and does not declare which version it needs; Loro builds with it. The
  editor spike also found two bugs around Loro (cursor handling in
  loro-prosemirror 0.4, undo history erased by `importBatch` in Loro 1.17);
  y-prosemirror is older and more widely used.

Decision: [ADR-0007](../adr/0007-crdt-engine-loro.md).
