//! The encrypted local archive of ALTAIS.
//!
//! An archive is a folder with two files:
//!
//! - `archive.db`: a SQLCipher database (SQLite encrypted with AES-256 and
//!   HMAC-SHA512 page authentication) holding notebooks, notes, tags and
//!   attachments;
//! - `keys.json`: encrypted copies of the database key (see [`keys`] and
//!   ADR-0008).
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
mod error;
pub mod keys;
mod schema;

pub use archive::{Archive, NoteSummary, Notebook, Unlock};
pub use error::StorageError;
pub use keys::{DbKey, KdfParams, KeyStore, OsKeyStore, RecoveryKey};
