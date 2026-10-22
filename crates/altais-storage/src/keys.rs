//! Protection of the database key (ADR-0008).
//!
//! The archive is encrypted with a random 256-bit **database key**. That key
//! is never derived from anything the user types; instead it is stored, or
//! "wrapped" (encrypted), in up to three places:
//!
//! - the **operating system's credential store** (Windows Credential
//!   Manager), so the archive opens without a password;
//! - a **password slot**: the key encrypted with a key derived from the
//!   user's password (Argon2id). When a password is set, the copy in the
//!   operating system is removed, so the password is then required;
//! - a **recovery slot**: the key encrypted with the recovery key shown once
//!   when the archive is created. It always exists, so a forgotten password
//!   or a new computer does not mean lost notes.
//!
//! The slots live in `keys.json` next to the database. Without one of the
//! three secrets the file is useless.

use std::fmt;
use std::sync::Arc;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{XChaCha20Poly1305, XNonce};
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use crate::StorageError;

/// Current format of `keys.json`.
pub const KEY_FILE_FORMAT: u32 = 1;

/// Name under which ALTAIS stores database keys in the OS credential store.
pub const KEYSTORE_SERVICE: &str = "ALTAIS archive key";

/// Argon2id cost for password slots: 64 MiB, 3 passes, 1 lane
/// (about 0.2 s per unlock on a current PC; see `examples/kdf_time.rs`).
pub const PASSWORD_KDF: KdfParams = KdfParams {
    memory_kib: 64 * 1024,
    iterations: 3,
    parallelism: 1,
};

fn random<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).expect("the operating system provides random numbers");
    bytes
}

/// The key SQLCipher encrypts the archive with.
#[derive(Clone, Zeroize, ZeroizeOnDrop)]
pub struct DbKey([u8; 32]);

impl DbKey {
    /// A new random key.
    pub fn generate() -> Self {
        Self(random())
    }

    /// The raw key bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// SQLCipher's raw-key form: `x'…'` with 64 hex digits.
    pub(crate) fn sqlcipher_pragma_value(&self) -> Zeroizing<String> {
        let mut s = Zeroizing::new(String::with_capacity(67));
        s.push_str("x'");
        for b in self.0 {
            s.push_str(&format!("{b:02x}"));
        }
        s.push('\'');
        s
    }

    fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; 32]>::try_from(bytes).ok().map(Self)
    }
}

impl fmt::Debug for DbKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DbKey(…)")
    }
}

// ---------------- Recovery key ----------------

/// Crockford base32 alphabet (no I, L, O, U).
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// 256 random bits the user writes down or prints when the archive is
/// created. Written as 52 base32 digits plus 2 check digits, in groups of 6:
/// `ALTAIS-XXXXXX-XXXXXX-…`.
#[derive(Clone, PartialEq, Eq, Zeroize, ZeroizeOnDrop)]
pub struct RecoveryKey([u8; 32]);

impl RecoveryKey {
    /// A new random recovery key.
    pub fn generate() -> Self {
        Self(random())
    }

    fn digits(bytes: &[u8; 32]) -> Vec<u8> {
        // 256 bits → 52 five-bit digits (the last digit carries 1 bit + padding).
        let mut out = Vec::with_capacity(54);
        let mut acc: u32 = 0;
        let mut bits = 0;
        for &b in bytes {
            acc = (acc << 8) | u32::from(b);
            bits += 8;
            while bits >= 5 {
                bits -= 5;
                out.push(((acc >> bits) & 31) as u8);
            }
        }
        if bits > 0 {
            out.push(((acc << (5 - bits)) & 31) as u8);
        }
        out
    }

    fn checksum(digits: &[u8]) -> [u8; 2] {
        let hash = blake3::hash(digits);
        let h = hash.as_bytes();
        [h[0] & 31, h[1] & 31]
    }

    /// The printable form.
    pub fn display(&self) -> Zeroizing<String> {
        let mut digits = Self::digits(&self.0);
        let check = Self::checksum(&digits);
        digits.extend_from_slice(&check);
        let mut s = Zeroizing::new(String::from("ALTAIS"));
        for (i, d) in digits.iter().enumerate() {
            if i % 6 == 0 {
                s.push('-');
            }
            s.push(ALPHABET[*d as usize] as char);
        }
        digits.zeroize();
        s
    }

    /// Reads a recovery key as typed: case, spaces, dashes and the letters
    /// I/L (for 1) and O (for 0) are tolerated; a typo is detected by the
    /// check digits.
    pub fn parse(input: &str) -> Result<Self, StorageError> {
        // The "ALTAIS" prefix is optional and may be typed in any case.
        let mut rest = input.trim();
        if rest
            .get(..6)
            .is_some_and(|p| p.eq_ignore_ascii_case("altais"))
        {
            rest = &rest[6..];
        }
        let cleaned: String = rest
            .chars()
            .filter(|c| !c.is_whitespace() && *c != '-')
            .collect();
        let mut digits = Vec::with_capacity(54);
        for c in cleaned.chars() {
            let c = match c.to_ascii_uppercase() {
                'I' | 'L' => '1',
                'O' => '0',
                c => c,
            };
            let value = ALPHABET
                .iter()
                .position(|&a| a as char == c)
                .ok_or(StorageError::BadRecoveryKey)?;
            digits.push(value as u8);
        }
        if digits.len() != 54 {
            return Err(StorageError::BadRecoveryKey);
        }
        let (body, check) = digits.split_at(52);
        if Self::checksum(body) != [check[0], check[1]] {
            return Err(StorageError::BadRecoveryKey);
        }
        let mut bytes = [0u8; 32];
        let mut acc: u32 = 0;
        let mut bits = 0;
        let mut i = 0;
        for &d in body {
            acc = (acc << 5) | u32::from(d);
            bits += 5;
            if bits >= 8 && i < 32 {
                bits -= 8;
                bytes[i] = (acc >> bits) as u8;
                i += 1;
            }
        }
        digits.zeroize();
        Ok(Self(bytes))
    }
}

impl fmt::Debug for RecoveryKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RecoveryKey(…)")
    }
}

// ---------------- Key file ----------------

/// Argon2id parameters stored with a password slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KdfParams {
    /// Memory in KiB.
    pub memory_kib: u32,
    /// Passes over the memory.
    pub iterations: u32,
    /// Lanes.
    pub parallelism: u32,
}

/// One encrypted copy of the database key.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Slot {
    /// Key encrypted with an Argon2id key derived from the password.
    Password {
        /// Argon2id salt (hex).
        salt: String,
        /// Argon2id cost.
        kdf: KdfParams,
        /// XChaCha20-Poly1305 nonce (hex).
        nonce: String,
        /// Encrypted key and tag (hex).
        ciphertext: String,
    },
    /// Key encrypted with the recovery key.
    Recovery {
        /// XChaCha20-Poly1305 nonce (hex).
        nonce: String,
        /// Encrypted key and tag (hex).
        ciphertext: String,
    },
}

/// Contents of `keys.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyFile {
    /// Format version.
    pub format: u32,
    /// Random identifier of this archive; names the OS credential.
    pub archive_id: String,
    /// Whether a copy of the key is kept in the OS credential store.
    pub os_keystore: bool,
    /// Encrypted copies of the key.
    pub slots: Vec<Slot>,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Result<Vec<u8>, StorageError> {
    if !s.len().is_multiple_of(2) {
        return Err(StorageError::DamagedKeyFile("odd hex length".into()));
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|_| StorageError::DamagedKeyFile("not hex".into()))
        })
        .collect()
}

fn aad(kind: &str, archive_id: &str) -> Vec<u8> {
    format!("altais-key-slot/v1/{kind}/{archive_id}").into_bytes()
}

fn seal(wrapping_key: &[u8; 32], key: &DbKey, aad: &[u8]) -> (String, String) {
    let cipher = XChaCha20Poly1305::new(wrapping_key.into());
    let nonce: [u8; 24] = random();
    let ciphertext = cipher
        .encrypt(
            &XNonce::from(nonce),
            Payload {
                msg: key.as_bytes(),
                aad,
            },
        )
        .expect("encryption of 32 bytes cannot fail");
    (hex(&nonce), hex(&ciphertext))
}

fn open(wrapping_key: &[u8; 32], nonce: &str, ciphertext: &str, aad: &[u8]) -> Option<DbKey> {
    let nonce: [u8; 24] = unhex(nonce).ok()?.try_into().ok()?;
    let ciphertext = unhex(ciphertext).ok()?;
    let cipher = XChaCha20Poly1305::new(wrapping_key.into());
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                &XNonce::from(nonce),
                Payload {
                    msg: &ciphertext,
                    aad,
                },
            )
            .ok()?,
    );
    DbKey::from_slice(&plain)
}

fn password_key(
    password: &str,
    salt: &[u8],
    kdf: KdfParams,
) -> Result<Zeroizing<[u8; 32]>, StorageError> {
    let params = Params::new(kdf.memory_kib, kdf.iterations, kdf.parallelism, Some(32))
        .map_err(|e| StorageError::DamagedKeyFile(format!("password parameters: {e}")))?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, out.as_mut())
        .map_err(|e| StorageError::DamagedKeyFile(format!("password derivation: {e}")))?;
    Ok(out)
}

impl KeyFile {
    /// A key file for a new archive with only the recovery slot.
    pub fn new(key: &DbKey, recovery: &RecoveryKey) -> Self {
        let archive_id = hex(&random::<16>());
        let (nonce, ciphertext) = seal(&recovery.0, key, &aad("recovery", &archive_id));
        Self {
            format: KEY_FILE_FORMAT,
            archive_id,
            os_keystore: false,
            slots: vec![Slot::Recovery { nonce, ciphertext }],
        }
    }

    /// Whether a password is set.
    pub fn has_password(&self) -> bool {
        self.slots
            .iter()
            .any(|s| matches!(s, Slot::Password { .. }))
    }

    /// Sets (or replaces) the password slot.
    pub fn set_password(
        &mut self,
        key: &DbKey,
        password: &str,
        kdf: KdfParams,
    ) -> Result<(), StorageError> {
        if password.is_empty() {
            return Err(StorageError::EmptyPassword);
        }
        let salt: [u8; 16] = random();
        let wrapping = password_key(password, &salt, kdf)?;
        let (nonce, ciphertext) = seal(&wrapping, key, &aad("password", &self.archive_id));
        self.slots.retain(|s| !matches!(s, Slot::Password { .. }));
        self.slots.push(Slot::Password {
            salt: hex(&salt),
            kdf,
            nonce,
            ciphertext,
        });
        Ok(())
    }

    /// Removes the password slot.
    pub fn remove_password(&mut self) {
        self.slots.retain(|s| !matches!(s, Slot::Password { .. }));
    }

    /// Replaces the recovery slot with one for a new recovery key.
    pub fn set_recovery(&mut self, key: &DbKey, recovery: &RecoveryKey) {
        let (nonce, ciphertext) = seal(&recovery.0, key, &aad("recovery", &self.archive_id));
        self.slots.retain(|s| !matches!(s, Slot::Recovery { .. }));
        self.slots.push(Slot::Recovery { nonce, ciphertext });
    }

    /// The database key from the password, or `WrongPassword`.
    pub fn unlock_with_password(&self, password: &str) -> Result<DbKey, StorageError> {
        for slot in &self.slots {
            if let Slot::Password {
                salt,
                kdf,
                nonce,
                ciphertext,
            } = slot
            {
                let wrapping = password_key(password, &unhex(salt)?, *kdf)?;
                return open(
                    &wrapping,
                    nonce,
                    ciphertext,
                    &aad("password", &self.archive_id),
                )
                .ok_or(StorageError::WrongPassword);
            }
        }
        Err(StorageError::NoPasswordSet)
    }

    /// The database key from the recovery key, or `WrongRecoveryKey`.
    pub fn unlock_with_recovery(&self, recovery: &RecoveryKey) -> Result<DbKey, StorageError> {
        for slot in &self.slots {
            if let Slot::Recovery { nonce, ciphertext } = slot {
                return open(
                    &recovery.0,
                    nonce,
                    ciphertext,
                    &aad("recovery", &self.archive_id),
                )
                .ok_or(StorageError::WrongRecoveryKey);
            }
        }
        Err(StorageError::DamagedKeyFile("no recovery slot".into()))
    }

    /// Reads `keys.json`.
    pub fn from_json(json: &str) -> Result<Self, StorageError> {
        let file: Self =
            serde_json::from_str(json).map_err(|e| StorageError::DamagedKeyFile(e.to_string()))?;
        if file.format != KEY_FILE_FORMAT {
            return Err(StorageError::DamagedKeyFile(format!(
                "unknown format {}",
                file.format
            )));
        }
        Ok(file)
    }

    /// Writes `keys.json`.
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("key file serializes")
    }
}

// ---------------- OS credential store ----------------

/// Where the database key is kept so the archive opens without a password.
pub trait KeyStore: Send + Sync {
    /// Stores the key for an archive.
    fn store(&self, archive_id: &str, key: &DbKey) -> Result<(), StorageError>;
    /// Loads the key for an archive, if present.
    fn load(&self, archive_id: &str) -> Result<Option<DbKey>, StorageError>;
    /// Removes the key for an archive (no error if absent).
    fn remove(&self, archive_id: &str) -> Result<(), StorageError>;
}

/// A key store backed by a keyring-core credential store.
pub struct OsKeyStore {
    store: Arc<keyring_core::CredentialStore>,
}

impl OsKeyStore {
    /// The operating system's store: Windows Credential Manager on Windows.
    /// On other systems there is none yet (a password is then required).
    pub fn system() -> Result<Self, StorageError> {
        #[cfg(windows)]
        {
            let store = windows_native_keyring_store::Store::new()
                .map_err(|e| StorageError::KeyStore(e.to_string()))?;
            Ok(Self { store })
        }
        #[cfg(not(windows))]
        {
            Err(StorageError::KeyStore(
                "no operating-system credential store is supported on this system yet".into(),
            ))
        }
    }

    /// keyring-core's in-memory mock store, for tests.
    pub fn mock() -> Self {
        let store = keyring_core::mock::Store::new().expect("mock store");
        Self { store }
    }

    fn entry(&self, archive_id: &str) -> Result<keyring_core::Entry, StorageError> {
        self.store
            .build(KEYSTORE_SERVICE, archive_id, None)
            .map_err(|e| StorageError::KeyStore(e.to_string()))
    }
}

impl KeyStore for OsKeyStore {
    fn store(&self, archive_id: &str, key: &DbKey) -> Result<(), StorageError> {
        self.entry(archive_id)?
            .set_secret(key.as_bytes())
            .map_err(|e| StorageError::KeyStore(e.to_string()))
    }

    fn load(&self, archive_id: &str) -> Result<Option<DbKey>, StorageError> {
        match self.entry(archive_id)?.get_secret() {
            Ok(secret) => {
                let secret = Zeroizing::new(secret);
                DbKey::from_slice(&secret)
                    .map(Some)
                    .ok_or_else(|| StorageError::KeyStore("stored key has the wrong length".into()))
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(e) => Err(StorageError::KeyStore(e.to_string())),
        }
    }

    fn remove(&self, archive_id: &str) -> Result<(), StorageError> {
        match self.entry(archive_id)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(StorageError::KeyStore(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAST: KdfParams = KdfParams {
        memory_kib: 1024,
        iterations: 1,
        parallelism: 1,
    };

    #[test]
    fn recovery_key_round_trips_and_tolerates_typing() {
        let key = RecoveryKey::generate();
        let shown = key.display();
        assert!(shown.starts_with("ALTAIS-"));
        assert_eq!(shown.len(), "ALTAIS".len() + 54 + 9);
        assert_eq!(RecoveryKey::parse(&shown).unwrap(), key);
        let typed = shown
            .to_lowercase()
            .replace('-', " ")
            .replace('0', "o")
            .replace('1', "l");
        assert_eq!(RecoveryKey::parse(&typed).unwrap(), key);
        let without_prefix = shown.trim_start_matches("ALTAIS-");
        assert_eq!(RecoveryKey::parse(without_prefix).unwrap(), key);
    }

    #[test]
    fn recovery_key_typo_is_detected() {
        let shown = RecoveryKey::generate().display().to_string();
        let mut chars: Vec<char> = shown.chars().collect();
        let i = 10;
        chars[i] = if chars[i] == '2' { '3' } else { '2' };
        let typo: String = chars.into_iter().collect();
        assert!(matches!(
            RecoveryKey::parse(&typo),
            Err(StorageError::BadRecoveryKey)
        ));
        assert!(matches!(
            RecoveryKey::parse("ALTAIS-123"),
            Err(StorageError::BadRecoveryKey)
        ));
    }

    #[test]
    fn slots_unlock_only_with_the_right_secret() {
        let key = DbKey::generate();
        let recovery = RecoveryKey::generate();
        let mut file = KeyFile::new(&key, &recovery);
        assert_eq!(
            file.unlock_with_recovery(&recovery).unwrap().as_bytes(),
            key.as_bytes()
        );
        assert!(matches!(
            file.unlock_with_recovery(&RecoveryKey::generate()),
            Err(StorageError::WrongRecoveryKey)
        ));
        assert!(matches!(
            file.unlock_with_password("x"),
            Err(StorageError::NoPasswordSet)
        ));

        file.set_password(&key, "Štrbské Pleso 2026", FAST).unwrap();
        let file = KeyFile::from_json(&file.to_json()).unwrap();
        assert_eq!(
            file.unlock_with_password("Štrbské Pleso 2026")
                .unwrap()
                .as_bytes(),
            key.as_bytes()
        );
        assert!(matches!(
            file.unlock_with_password("strbske pleso 2026"),
            Err(StorageError::WrongPassword)
        ));
    }

    #[test]
    fn a_slot_cannot_be_moved_to_another_archive() {
        let key = DbKey::generate();
        let recovery = RecoveryKey::generate();
        let a = KeyFile::new(&key, &recovery);
        let mut b = KeyFile::new(&key, &recovery);
        b.slots = a.slots.clone(); // same ciphertext, different archive id
        assert!(matches!(
            b.unlock_with_recovery(&recovery),
            Err(StorageError::WrongRecoveryKey)
        ));
    }

    #[test]
    fn mock_keystore_stores_and_removes() {
        let store = OsKeyStore::mock();
        let key = DbKey::generate();
        assert!(store.load("a1").unwrap().is_none());
        store.store("a1", &key).unwrap();
        assert_eq!(
            store.load("a1").unwrap().unwrap().as_bytes(),
            key.as_bytes()
        );
        store.remove("a1").unwrap();
        store.remove("a1").unwrap();
        assert!(store.load("a1").unwrap().is_none());
    }
}
