# Contributing to ALTAIS

Thank you for your interest in ALTAIS. This document explains how to propose
changes.

## Before you start

- For anything larger than a small fix, open an issue first so we can agree on
  the approach.
- Code, comments, commit messages, issues and specifications are written in
  English. The user interface is localised into Slovak, Czech, English, Hungarian,
  Polish, Russian and Simplified Chinese (see `docs/adr/0004-languages-and-localisation.md`).
- Security problems are never reported in public issues; see
  [`SECURITY.md`](SECURITY.md).

## Developer Certificate of Origin

ALTAIS uses the [Developer Certificate of Origin](https://developercertificate.org/)
(DCO) instead of a contributor licence agreement. By adding a sign-off line to
a commit you certify that you wrote the change or otherwise have the right to
submit it under the project's licence.

Add the sign-off automatically with `git commit -s`. It looks like this:

```
Signed-off-by: Your Name <you@example.com>
```

Commits without a sign-off cannot be merged; CI checks every commit of a pull
request (automated updates from Dependabot are exempt).

## Licences of contributions

- Code is contributed under the [Apache License 2.0](LICENSE).
- Contributions to [`spec/`](spec/) are dedicated to the public domain under
  [CC0 1.0](spec/LICENSE.md).
- Do not copy code, designs, icons or texts from other note-taking
  applications. ALTAIS is an independent implementation.
- New dependencies must use a licence allowed in [`deny.toml`](deny.toml).
  CI rejects anything else.

## Development workflow

1. Fork the repository and create a branch from `main`.
2. Make your change with tests.
3. Run the checks locally:

   ```sh
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

4. Open a pull request. Describe what changed and why, and link the issue.

## Guidelines

- **No silent data loss.** Importers, sync and migrations must account for
  every item: processed, or reported with a reason.
- **Originals are immutable.** Never modify attachment bytes; derived data
  (OCR, previews, indexes) must be rebuildable.
- **Preserve the unknown.** Code that reads ADM data must keep fields and
  block types it does not understand.
- **Keep secrets out of logs.** Never log keys, passwords, note content or
  titles.
- Prefer small, focused pull requests. Each one should leave `main` working.

## Code of conduct

Participation in this project is governed by the
[Code of Conduct](CODE_OF_CONDUCT.md).
