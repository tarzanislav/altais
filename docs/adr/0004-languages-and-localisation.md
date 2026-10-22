# ADR-0004: Languages and localisation

- Status: accepted
- Date: 2026-10-22

## Context

ALTAIS is a public project. Its user interface must be available in Slovak,
Czech, English, Hungarian, Polish, Russian and Chinese, and notes in all of
these languages must be searchable and recognisable by OCR. Several of them
need more than translated strings: Polish and Russian have three or more
plural forms, Hungarian and Czech have special sorting rules, letters such as
Polish `ł` are not removed by simple diacritic folding, and Chinese text has
no spaces between words.

## Decision

| Area | Decision |
| --- | --- |
| UI languages | `sk`, `cs`, `en`, `hu`, `pl`, `ru`, `zh-Hans` (Simplified Chinese); `zh-Hant` later. English is the source language of all strings. |
| Message format | [Project Fluent](https://projectfluent.org/) (Mozilla; crates under Apache-2.0 / MIT): correct plurals and grammatical cases; strings are never assembled by concatenation. |
| Dates, numbers, sorting | ICU4X (Unicode-3.0 licence): locale-aware formatting and collation (for example Czech `ch` after `h`). Dates without time stay civil dates. |
| Full-text search | SQLite FTS5 `unicode61` with an ALTAIS folding table on top of diacritic removal (`ł→l`, `ø→o`, `ß→ss`, Hungarian `ő ű`), so a search without diacritics finds text with them. Chinese and other text without spaces is indexed with FTS5's `trigram` tokenizer; dictionary segmentation can be added later. |
| Language of a note | Optional `lang` field (BCP 47 tag) in the ADM; chooses tokenization, OCR language and spell checking. Detected automatically when missing. |
| OCR | Tesseract language data `slk`, `ces`, `eng`, `hun`, `pol`, `rus`, `chi_sim`, `chi_tra`; downloaded on demand to keep the installer small. |
| Input and display | Editor supports IME composition (Chinese input); fonts come from the operating system, no bundled CJK fonts; layout uses logical CSS properties so right-to-left languages can be added later. |
| Translations | Fluent files in the repository, one per language; community translation through a hosted service may be added later. |

## Consequences

- Every user-visible string goes through Fluent from the first UI commit.
- Search and OCR tests include samples in all seven languages.
- Adding a language means adding one Fluent file and, if needed, OCR data.
