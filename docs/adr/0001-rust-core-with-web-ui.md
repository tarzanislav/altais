# ADR-0001: Rust core with a web-technology UI

- Status: accepted
- Date: 2026-10-22

## Context

ALTAIS needs one implementation of its data logic (CRDT documents,
encryption, storage, sync, import) for Windows, Linux, macOS, Android, iOS, a
web client and a browser clipper. Its content is rich text that mostly
arrives as HTML: Evernote ENML, web clippings, text pasted from Word and
browsers. The code must be safe when parsing untrusted files, and every
required dependency must be free for private and commercial use.

## Options considered

| Criterion | Rust core + web UI (Tauri) | C# / .NET (Avalonia or MAUI) | Kotlin Multiplatform | Flutter + Rust core | Electron + TypeScript |
| --- | --- | --- | --- | --- | --- |
| One core on all targets incl. browser extension | yes, small WASM | partly: MAUI on Linux is a preview on .NET 11; WASM is heavy | Android strong, desktop on JVM, web in beta | yes, web heavier | desktop and web; mobile separate |
| Rich-text editor and HTML fidelity | excellent (ProseMirror in a web engine) | weak: no comparable editor; Avalonia WebView open-sourced only with v12 | weak | medium | excellent |
| CRDT libraries (Loro, Yrs, Automerge) | native | via Rust bindings | via Rust bindings | native (Rust) | via WASM |
| Post-quantum crypto on every platform | yes, pure Rust | ML-KEM in .NET 10 depends on the OS, absent on macOS | partly | yes (Rust) | JS libraries |
| Memory safety for untrusted input | yes, no GC | yes | yes | yes | yes, weaker process isolation |
| Footprint | small | medium | heavier on desktop | medium | large |
| Readability for the maintainer | harder | best (used in EviTA) | good | two languages | one language |

## Decision

Rust for the core and the server; React + TypeScript with ProseMirror for the
UI, packaged with Tauri 2. If Tauri does not meet the requirements on mobile,
the fallback for the mobile UI is Flutter on top of the same Rust core
(the architecture used by AppFlowy). The core does not change in either case.

## Consequences

- The data logic exists once and runs natively and as WASM.
- Notes stay HTML-shaped end to end, which keeps Evernote imports and web
  clippings faithful.
- Rust is harder to read for the maintainer. Pull requests therefore explain
  behaviour in plain language and ship tests and a test build, so review
  focuses on behaviour; cryptography and sync additionally get an independent
  review and an external audit before release 1.0.
