# ALTAIS specifications

This directory holds the public specifications of ALTAIS: the formats and
protocols that must outlive any particular implementation.

| Specification | Status | Location |
| --- | --- | --- |
| ALTAIS Document Model (ADM) | 0.1 draft | [`adm/`](adm/) |
| Sync protocol (encrypted log, CDDL) | planned | `protocol/` |
| Query language | planned | `query/` |
| Backup format | planned | `backup/` |

## ALTAIS Document Model (ADM) 0.1

The ADM is the public JSON representation of one object (note, task,
attachment, ...). It is used for export, import, backup and interchange.
Inside the application the editable source of truth is a CRDT document; the
ADM is derived from it.

- Schema: [`adm/adm-0.1.schema.json`](adm/adm-0.1.schema.json) (JSON Schema 2020-12)
- Example: [`adm/examples/note.json`](adm/examples/note.json)
- Reference implementation: [`crates/altais-model`](../crates/altais-model)

Rules that every reader and writer must follow:

1. **Identity by ID.** Objects use lowercase UUIDs (UUIDv7 for new objects);
   blocks use short stable IDs unique within their object. Links never rely on
   titles or file names: `altais://o/<object-id>#<block-id>`.
2. **Preserve the unknown.** Unknown fields, object types and block types are
   kept and written back unchanged. A reader may show an unknown block as
   read-only, but must not drop it.
3. **Attachments are immutable.** An attachment is referenced by the BLAKE3
   hash of its original bytes (`blake3:<64 hex>`). A new version of a file is
   a new blob.
4. **Dates.** Timestamps are RFC 3339 in UTC. A date without a time (for
   example a task due date) is a civil date `YYYY-MM-DD`, never midnight UTC.
5. **Validation.** A document is valid when it matches the schema and, in
   addition, block IDs are unique and `updated` is not earlier than `created`.

## License

The specifications in this directory are dedicated to the public domain under
[CC0 1.0 Universal](https://creativecommons.org/publicdomain/zero/1.0/legalcode),
so that anyone may implement them without restriction. See
[`LICENSE.md`](LICENSE.md). Code in the rest of the repository is licensed
under the Apache License 2.0.
