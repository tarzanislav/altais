# ADR-0003: No blockchain in the core

- Status: accepted
- Date: 2026-10-22

## Context

Blockchains offer tamper evidence and replication without a central server.
These properties are attractive for a long-lived archive, so public chains
(Bitcoin, Ethereum), permissioned chains (Hyperledger Fabric, Hyperledger
Besu) and related systems (IPFS/Filecoin, Arweave, Holochain) were evaluated.

## Decision

ALTAIS does not use a blockchain for storage or sync.

- **Offline-first:** every blockchain needs consensus among a majority of
  nodes before a write is final. With two or three personal devices this would
  block writing a note without a connection. CRDTs accept writes at any time
  and merge later.
- **Privacy:** public chains are visible to everyone forever; even encrypted
  entries reveal when and how much was written, and encrypted data can be
  harvested for future decryption.
- **Deletion:** data on a chain cannot be removed, which conflicts with the
  user's control over their own data and with GDPR.
- **Attack surface and cost:** consensus software, smart contracts and wallet
  keys add complexity and fees without benefit for a single user's devices.

ALTAIS keeps the useful ideas without a chain: every change is signed by its
device, logs are append-only per device (as in Hypercore and Secure
Scuttlebutt), attachments are content-addressed with BLAKE3, a Merkle manifest
detects corruption, and membership changes form a signed linear chain.

## Possible later extension (P2)

Optional proof of existence: only a salted BLAKE3 hash of a chosen note is
anchored to a public timestamping service, never its content. Off by default,
enabled per note, with a warning that the public network sees the time of the
entry. The core never depends on it.
