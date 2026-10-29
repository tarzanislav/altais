# ADR-0009: Attachments in encrypted files

- Status: accepted (approved by the author, 2026-10-29)
- Date: 2026-10-29

## Context

Until now the content of attachments was kept inside the SQLCipher database.
An Evernote archive of 1 000 – 10 000 notes can hold gigabytes of images and
PDFs. One huge database file is slow to back up and copy, every change
rewrites pages of it, and file-based sync tools would have to transfer the
whole file again.

## Decision

- Every piece of content is **one file under `blobs/`** in the archive folder.
  Identical content is stored once (it is addressed by its BLAKE3 hash).
- **File name**: a keyed BLAKE3 hash of the content address,
  `blobs/<2 hex>/<62 hex>`. A plain hash would let anyone holding a known
  file check whether it is in the archive; the keyed name reveals nothing.
- **File content**: `ALTB`, format byte `1`, a random 24-byte nonce and the
  content encrypted with **XChaCha20-Poly1305**. The format and the content
  address are authenticated with it, so a file cannot be swapped for another
  one; after decrypting, the content is hashed and compared with its address
  and the size recorded in the database.
- **Keys**: derived from the archive's database key with BLAKE3
  `derive_key` (separate contexts for names and content). No new key,
  password or key-file slot; changing the password or the recovery key does
  not touch the attachments.
- A file is written to a temporary name, flushed and then renamed, so it
  appears complete or not at all. The database records that the content
  exists only after its file is on disk.
- The database keeps the list of content (address and size) so a note
  referring to missing content is still refused when saved.
- **Older archives**: schema version 3. When an archive with content in the
  database is opened, the content is moved into files one item at a time
  (an interruption simply continues next time), then the database is
  compacted with `VACUUM`. Older versions of ALTAIS refuse a version-3
  archive instead of showing empty attachments.
- A missing or damaged file is reported as an error ("the attachment's file
  is missing or damaged"), never returned as empty content.

## Consequences

- The database stays small; backup and folder sync copy only new files.
- What an attacker with the folder can see: the number of attachments and
  their approximate sizes. Names, types and contents stay hidden.
- Content is encrypted in one piece in memory, which is fine for typical
  attachments (Evernote allows at most 200 MB per note). Splitting into
  chunks (FastCDC) with per-chunk encryption comes with sync (F3), where it
  also enables resumable transfer; the format byte allows a second format.
- Content no longer referred to by any note (not even one in the trash) is
  deleted when a note is deleted for good or the trash is emptied; content
  stored in the current session is kept, because the note it was added to
  may not be saved yet. Deleted database pages are overwritten
  (`secure_delete`) and the write-ahead log is checkpointed.
