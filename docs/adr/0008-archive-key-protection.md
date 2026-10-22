# ADR-0008: Protection of the archive key

- Status: accepted (chosen by the author, 2026-10-23)
- Date: 2026-10-23

## Context

The local archive is a SQLCipher database. It needs a key; how that key is
protected decides whether the user has to type a password every time, and
what happens when a password is forgotten or the archive moves to another
computer. The author chose: **protected by the operating system, with an
optional password**, plus a recovery key.

## Decision

- The archive is encrypted with a **random 256-bit database key** (SQLCipher
  4: AES-256 with HMAC-SHA512 page authentication; raw key, no
  password-based derivation inside SQLCipher). The write-ahead log is
  encrypted too.
- Encrypted copies of that key live in `keys.json` next to the database:
  - **Operating system** (default): the key is kept in Windows Credential
    Manager (per-user, protected by Windows). The archive opens without a
    password.
  - **Password** (optional, in Settings): the key encrypted with
    XChaCha20-Poly1305 under a key derived by Argon2id (64 MiB, 3 passes,
    ~0.2 s per unlock). Setting a password **removes** the operating-system
    copy, so the password is then required; removing the password puts the
    copy back.
  - **Recovery key** (always): 256 random bits shown once when the archive is
    created, written as `ALTAIS-` plus 54 base32 digits in groups of six,
    including two check digits that catch typos. It opens the archive on a
    new computer or after a forgotten password, and can be replaced (the old
    one stops working).
- Each encrypted copy is bound to its archive (random archive ID in the
  associated data), so copies cannot be moved between archives.
- On systems without a supported credential store (Linux for now) a password
  is required; Linux Secret Service support is added later.
- Libraries: rusqlite with bundled SQLCipher and vendored OpenSSL, `argon2`,
  `chacha20poly1305`, `zeroize`, `getrandom`, `keyring-core` with
  `windows-native-keyring-store` (all MIT, Apache-2.0 or BSD-3).

## Consequences

- Day to day the archive opens like any program; a stolen disk or a copied
  archive folder is unreadable without Windows sign-in, the password or the
  recovery key.
- Someone using the unlocked, signed-in Windows account can open the archive
  unless a password is set; the Settings screen will say so.
- **Losing the recovery key and the password means losing the archive** if
  the Windows credential is also gone (new computer, reinstalled Windows).
  The first-run screen must make the user save the recovery key (print or
  file) and confirm it.
- Attachments are stored inside the encrypted database for now; the separate
  encrypted blob store (FastCDC, per-file keys) replaces this later in F1.
