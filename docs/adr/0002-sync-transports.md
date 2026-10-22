# ADR-0002: Sync transports, with direct device-to-device sync as the default

- Status: accepted
- Date: 2026-10-22

## Context

Sync is an end-to-end encrypted, append-only log of changes per space plus
encrypted snapshots (see `docs/architecture.md`). Because every transport
only moves opaque encrypted bytes, the log can travel over different
transports. Users must not need an account, a cloud subscription or their own
server, and no single provider may become a dependency. Security has priority
over convenience.

## Decision

All transports implement one `Transport` interface and carry the same
encrypted log, so they can be combined:

| Transport | How it works | Phase |
| --- | --- | --- |
| **ALTAIS Direct** (default) | Devices find each other by public key and connect directly over the internet or the local network, with NAT traversal; if a direct path fails, a relay forwards encrypted packets. Built on iroh 1.0 (MIT OR Apache-2.0). | F3 |
| Always-on node | A home PC, NAS or Raspberry Pi running ALTAIS as a sync peer, so devices can sync when the others are off. | F3 |
| WebDAV mailbox | Encrypted log segments stored at any WebDAV provider (for example Koofr, Nextcloud, a Synology NAS). | F3 |
| Shared folder | Each device writes immutable log segments into its own subfolder; works with Syncthing, OneDrive, Google Drive, Dropbox or a USB drive. | F3 |
| ALTAIS Relay | Optional self-hosted server (Rust, Axum) for families and organisations. | F5 |

## Security notes

- Content, titles, tags and attachments are encrypted on the device for every
  transport. Relays and storage providers see metadata only: IP addresses,
  timing and sizes.
- Users who do not want public relays to see their IP address can run their
  own iroh relay or restrict sync to the local network.
- iroh is part of the scope of the external security audit before 1.0.

## Consequences

- Release 1.0 does not need any server operated by the project.
- The folder and WebDAV transports share one implementation (per-device
  segment files), which keeps the code small.
- Direct sync needs two devices online at the same time; the always-on node
  or a WebDAV mailbox covers the rest.
