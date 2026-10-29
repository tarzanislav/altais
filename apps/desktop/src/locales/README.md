# Interface texts

One [Project Fluent](https://projectfluent.org) file per language (ADR-0004).
`sk.ftl` is the source; every other file has the same messages, which
`tests/i18n.test.ts` checks.

| File | Language | State |
|---|---|---|
| `sk.ftl` | Slovenčina | source |
| `cs.ftl` | Čeština | translated by Claude, review by a native speaker welcome |
| `en.ftl` | English | reviewed by the author |
| `hu.ftl` | Magyar | translated by Claude, review by a native speaker welcome |
| `pl.ftl` | Polski | translated by Claude, review by a native speaker welcome |
| `ru.ftl` | Русский | translated by Claude, review by a native speaker welcome |
| `zh-CN.ftl` | 简体中文 | translated by Claude, review by a native speaker welcome |

Adding or changing a text: change `sk.ftl` and `en.ftl`, then every other
file (if unsure, copy the English text; it is better than a guess), and add
the message id to `MESSAGES` in `src/i18n.ts` if it has no variables.

Numbers use Fluent's plural categories (`one`, `few`, `many`, `other`),
which differ by language: Slovak and Czech "1 poznámka, 2 poznámky,
5 poznámok", Polish and Russian also by the last digits (22 notatki,
25 notatek), Hungarian and Chinese do not change the noun.

The demo data exists in Slovak (also used for Czech) and English (all other
languages) for now.
