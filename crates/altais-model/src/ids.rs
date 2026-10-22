//! Identifiers used by the ADM.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Error returned when an identifier does not have the required form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The value is not a valid UUID.
    #[error("invalid object id: {0:?}")]
    InvalidObjectId(String),
    /// The value is not 4 to 32 characters of `A-Z`, `a-z`, `0-9`, `_` or `-`.
    #[error("invalid block id: {0:?}")]
    InvalidBlockId(String),
    /// The value is not `blake3:` followed by 64 lowercase hex digits.
    #[error("invalid blob reference: {0:?}")]
    InvalidBlobRef(String),
}

/// Stable identity of an object (note, task, attachment, ...).
///
/// New identifiers are UUIDv7, so they sort by creation time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObjectId(Uuid);

impl ObjectId {
    /// Creates a new time-ordered identifier (UUIDv7).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    /// Returns the underlying UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for ObjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl FromStr for ObjectId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|_| IdError::InvalidObjectId(s.to_owned()))
    }
}

/// Stable identity of a block inside one object.
///
/// Block IDs survive edits, moves and export. They make links to a block,
/// citations and embedded tasks possible.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlockId(String);

impl BlockId {
    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn is_valid(s: &str) -> bool {
        (4..=32).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }
}

impl TryFrom<String> for BlockId {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if Self::is_valid(&value) {
            Ok(Self(value))
        } else {
            Err(IdError::InvalidBlockId(value))
        }
    }
}

impl From<BlockId> for String {
    fn from(value: BlockId) -> Self {
        value.0
    }
}

impl FromStr for BlockId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl fmt::Display for BlockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Content address of an attachment: `blake3:` followed by 64 lowercase hex digits.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct BlobRef(String);

impl BlobRef {
    const PREFIX: &'static str = "blake3:";

    /// Returns the full reference, including the `blake3:` prefix.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns the 64 hex digits of the hash.
    pub fn hex(&self) -> &str {
        &self.0[Self::PREFIX.len()..]
    }

    fn is_valid(s: &str) -> bool {
        match s.strip_prefix(Self::PREFIX) {
            Some(hex) => {
                hex.len() == 64
                    && hex
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            }
            None => false,
        }
    }
}

impl TryFrom<String> for BlobRef {
    type Error = IdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if Self::is_valid(&value) {
            Ok(Self(value))
        } else {
            Err(IdError::InvalidBlobRef(value))
        }
    }
}

impl From<BlobRef> for String {
    fn from(value: BlobRef) -> Self {
        value.0
    }
}

impl FromStr for BlobRef {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s.to_owned())
    }
}

impl fmt::Display for BlobRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_object_ids_are_uuid_v7() {
        let id = ObjectId::new();
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn object_id_round_trips_through_text() {
        let id = ObjectId::new();
        let parsed: ObjectId = id.to_string().parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn block_id_rules() {
        assert!("k7Qp2x".parse::<BlockId>().is_ok());
        assert!("a_b-1".parse::<BlockId>().is_ok());
        assert!("abc".parse::<BlockId>().is_err(), "too short");
        assert!("a".repeat(33).parse::<BlockId>().is_err(), "too long");
        assert!("has space".parse::<BlockId>().is_err());
        assert!("diakritika-č".parse::<BlockId>().is_err());
    }

    #[test]
    fn blob_ref_rules() {
        let hex = "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08";
        let r: BlobRef = format!("blake3:{hex}").parse().unwrap();
        assert_eq!(r.hex(), hex);
        assert!(format!("sha256:{hex}").parse::<BlobRef>().is_err());
        assert!(
            format!("blake3:{}", hex.to_uppercase())
                .parse::<BlobRef>()
                .is_err()
        );
        assert!("blake3:abc".parse::<BlobRef>().is_err());
    }
}
