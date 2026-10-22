//! ALTAIS Document Model (ADM).
//!
//! The ADM is the public, library-independent representation of ALTAIS data.
//! It is used for export, import, backup and interchange. Inside the
//! application the editable source of truth is a CRDT document; the ADM is
//! always derived from it and never edited in parallel.
//!
//! The format is specified in `spec/adm/` and dedicated to the public domain
//! (CC0-1.0), so anyone can write a compatible reader or writer.
//!
//! Design rules that this crate enforces:
//!
//! * identities are stable IDs (UUIDv7 for objects, short IDs for blocks),
//!   never names or file paths;
//! * attachments are referenced by content hash and are immutable;
//! * unknown block types are preserved verbatim, so an older reader never
//!   silently drops content written by a newer one.

mod block;
mod document;
mod ids;
mod validate;

pub use block::{Block, KnownBlock, Mark, SimpleMark, Span};
pub use document::{Document, ObjectType, Provenance, ProvenanceKind};
pub use ids::{BlobRef, BlockId, IdError, ObjectId};
pub use validate::{ValidationError, validate};

/// ADM version written by this crate.
pub const ADM_VERSION: &str = "0.1";
