//! Encrypted attachment files (ADR-0009).
//!
//! Each piece of content is one file under `blobs/` in the archive folder:
//!
//! - **name**: `blobs/<2 hex>/<62 hex>`, a keyed BLAKE3 hash of the content
//!   address. Without the archive key the name says nothing about the
//!   content (a plain hash would reveal whether a known file is present).
//! - **content**: `ALTB`, format byte `1`, a random 24-byte nonce, then the
//!   content encrypted with XChaCha20-Poly1305. The content address is
//!   authenticated with it, so a file cannot be swapped for another one; on
//!   reading, the decrypted content is hashed and compared with the address.
//!
//! Both keys are derived from the archive's database key with BLAKE3
//! `derive_key`, so the attachments need no key of their own, and changing
//! the password or the recovery key does not touch them.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use zeroize::Zeroizing;

use crate::StorageError;
use crate::keys::DbKey;

/// Folder of the attachment files inside the archive folder.
pub const BLOB_DIR: &str = "blobs";

const MAGIC: &[u8; 4] = b"ALTB";
const FORMAT: u8 = 1;
const NONCE_LEN: usize = 24;
const HEADER_LEN: usize = MAGIC.len() + 1 + NONCE_LEN;

const NAME_CONTEXT: &str = "ALTAIS 2026-10 attachment file names v1";
const CONTENT_CONTEXT: &str = "ALTAIS 2026-10 attachment content v1";

/// Reads and writes encrypted attachment files.
pub struct BlobStore {
    dir: PathBuf,
    name_key: Zeroizing<[u8; 32]>,
    cipher: XChaCha20Poly1305,
}

impl std::fmt::Debug for BlobStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlobStore")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

fn header_aad(hash: &[u8; 32]) -> [u8; 4 + 1 + 32] {
    let mut aad = [0u8; 37];
    aad[..4].copy_from_slice(MAGIC);
    aad[4] = FORMAT;
    aad[5..].copy_from_slice(hash);
    aad
}

impl BlobStore {
    /// The store of the archive in `archive_dir`, encrypted under keys
    /// derived from `key`.
    pub fn new(archive_dir: &Path, key: &DbKey) -> Self {
        let name_key = Zeroizing::new(blake3::derive_key(NAME_CONTEXT, key.as_bytes()));
        let content_key = Zeroizing::new(blake3::derive_key(CONTENT_CONTEXT, key.as_bytes()));
        Self {
            dir: archive_dir.join(BLOB_DIR),
            name_key,
            cipher: XChaCha20Poly1305::new((&*content_key).into()),
        }
    }

    /// Path of the file holding the content with this BLAKE3 hash.
    pub fn path_of(&self, hash: &[u8; 32]) -> PathBuf {
        let name = blake3::keyed_hash(&self.name_key, hash).to_hex();
        self.dir.join(&name[..2]).join(&name[2..])
    }

    /// Whether the content with this hash has a file.
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.path_of(hash).is_file()
    }

    /// Writes `data` (whose BLAKE3 hash is `hash`) unless its file exists.
    /// The file appears complete or not at all.
    pub fn put(&self, hash: &[u8; 32], data: &[u8]) -> Result<(), StorageError> {
        let path = self.path_of(hash);
        if path.is_file() {
            return Ok(());
        }
        let mut nonce = [0u8; NONCE_LEN];
        getrandom::fill(&mut nonce)
            .map_err(|e| StorageError::Data(format!("random numbers: {e}")))?;
        let aad = header_aad(hash);
        let ciphertext = self
            .cipher
            .encrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: data,
                    aad: &aad,
                },
            )
            .map_err(|_| StorageError::Data("attachment could not be encrypted".into()))?;

        let folder = path.parent().expect("blob path has a folder");
        fs::create_dir_all(folder)?;
        let mut suffix = [0u8; 8];
        getrandom::fill(&mut suffix)
            .map_err(|e| StorageError::Data(format!("random numbers: {e}")))?;
        let tmp = folder.join(format!(
            ".tmp-{}",
            suffix
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        ));
        let written = (|| -> std::io::Result<()> {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(MAGIC)?;
            file.write_all(&[FORMAT])?;
            file.write_all(&nonce)?;
            file.write_all(&ciphertext)?;
            file.sync_all()?;
            drop(file);
            fs::rename(&tmp, &path)
        })();
        if let Err(e) = written {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        Ok(())
    }

    /// The content with this hash: `None` if it has no file, an error if
    /// the file is damaged or does not belong to this archive.
    pub fn get(&self, hash: &[u8; 32]) -> Result<Option<Vec<u8>>, StorageError> {
        let path = self.path_of(hash);
        let bytes = match fs::read(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let damaged = || StorageError::DamagedBlob(format!("blake3:{}", hex(hash)));
        if bytes.len() < HEADER_LEN || &bytes[..4] != MAGIC || bytes[4] != FORMAT {
            return Err(damaged());
        }
        let nonce: [u8; NONCE_LEN] = bytes[5..HEADER_LEN].try_into().expect("length checked");
        let nonce = XNonce::from(nonce);
        let aad = header_aad(hash);
        let data = self
            .cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &bytes[HEADER_LEN..],
                    aad: &aad,
                },
            )
            .map_err(|_| damaged())?;
        if blake3::hash(&data).as_bytes() != hash {
            return Err(damaged());
        }
        Ok(Some(data))
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        std::env::temp_dir().join(format!("altais-blobs-{}", hex(&bytes)))
    }

    #[test]
    fn content_comes_back_and_the_file_reveals_nothing() {
        let dir = temp_dir();
        let store = BlobStore::new(&dir, &DbKey::generate());
        let data = b"Zmluva o prenajme - tajne".repeat(10);
        let hash = *blake3::hash(&data).as_bytes();
        store.put(&hash, &data).unwrap();
        store.put(&hash, &data).unwrap(); // second time: nothing to do

        assert_eq!(store.get(&hash).unwrap().unwrap(), data);
        let file = fs::read(store.path_of(&hash)).unwrap();
        assert!(!file.windows(7).any(|w| w == b"Zmluva "));
        let name = store.path_of(&hash);
        assert!(!name.to_string_lossy().contains(&hex(&hash)[..16]));
        // Only the one file, no leftovers.
        let entries: Vec<_> = fs::read_dir(name.parent().unwrap()).unwrap().collect();
        assert_eq!(entries.len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_content_is_none() {
        let dir = temp_dir();
        let store = BlobStore::new(&dir, &DbKey::generate());
        assert!(store.get(&[7; 32]).unwrap().is_none());
    }

    #[test]
    fn a_damaged_or_swapped_file_is_refused() {
        let dir = temp_dir();
        let store = BlobStore::new(&dir, &DbKey::generate());
        let a = b"first".to_vec();
        let b = b"second".to_vec();
        let ha = *blake3::hash(&a).as_bytes();
        let hb = *blake3::hash(&b).as_bytes();
        store.put(&ha, &a).unwrap();
        store.put(&hb, &b).unwrap();

        // Swapped: b's file under a's name.
        fs::copy(store.path_of(&hb), store.path_of(&ha)).unwrap();
        assert!(matches!(store.get(&ha), Err(StorageError::DamagedBlob(_))));

        // Flipped bit.
        let mut bytes = fs::read(store.path_of(&hb)).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        fs::write(store.path_of(&hb), bytes).unwrap();
        assert!(matches!(store.get(&hb), Err(StorageError::DamagedBlob(_))));

        // Another archive's key.
        let other = BlobStore::new(&dir, &DbKey::generate());
        assert!(other.get(&ha).unwrap().is_none()); // different names
        let _ = fs::remove_dir_all(dir);
    }
}
