//! The encrypted local archive of ALTAIS.
//!
//! An archive is a folder with two files and a folder:
//!
//! - `archive.db`: a SQLCipher database (SQLite encrypted with AES-256 and
//!   HMAC-SHA512 page authentication) holding notebooks, notes, tags and
//!   attachments;
//! - `keys.json`: encrypted copies of the database key (see [`keys`] and
//!   ADR-0008);
//! - `blobs/`: attachment content, one encrypted file per content (see
//!   [`blobstore`] and ADR-0009).
//!
//! ```no_run
//! use std::sync::Arc;
//! use altais_storage::{Archive, OsKeyStore, Unlock};
//!
//! let keystore = Arc::new(OsKeyStore::system()?);
//! let (archive, recovery) = Archive::create("C:/Users/me/ALTAIS".as_ref(), Some(keystore.clone()), None)?;
//! println!("Write down your recovery key: {}", recovery.display().as_str());
//! drop(archive);
//! let archive = Archive::open("C:/Users/me/ALTAIS".as_ref(), Some(keystore), Unlock::KeyStore)?;
//! # Ok::<(), altais_storage::StorageError>(())
//! ```

mod archive;
pub mod blobstore;
mod error;
pub mod fold;
pub mod keys;
mod schema;
mod text;

pub use archive::{Archive, NoteSummary, Notebook, SearchHit, Unlock};
pub use error::StorageError;
pub use keys::{DbKey, KdfParams, KeyStore, OsKeyStore, RecoveryKey};
pub use text::body_text as note_text;
