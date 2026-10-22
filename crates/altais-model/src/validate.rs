//! Structural validation of ADM documents.

use std::collections::HashSet;

use crate::block::{Block, KNOWN_BLOCK_TYPES, KnownBlock};
use crate::document::Document;

/// A rule that a document breaks.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// The `adm` field names a version this implementation cannot read.
    #[error("unsupported ADM version {0:?}; this implementation reads 0.x")]
    UnsupportedVersion(String),
    /// `updated` is earlier than `created`.
    #[error("updated time is earlier than created time")]
    UpdatedBeforeCreated,
    /// A block has no `bid` field.
    #[error("block #{index} has no block id")]
    MissingBlockId {
        /// Position of the block in `content`.
        index: usize,
    },
    /// Two blocks share the same ID.
    #[error("duplicate block id {bid:?}")]
    DuplicateBlockId {
        /// The repeated ID.
        bid: String,
    },
    /// A heading level is outside 1 to 4.
    #[error("heading {bid:?} has level {level}; allowed levels are 1 to 4")]
    InvalidHeadingLevel {
        /// ID of the heading block.
        bid: String,
        /// The level found.
        level: u8,
    },
    /// A block has a known type but does not match that type's fields.
    #[error("block #{index} has type {t:?} but does not match its schema")]
    MalformedKnownBlock {
        /// Position of the block in `content`.
        index: usize,
        /// The block type.
        t: String,
    },
}

/// Checks the rules that the JSON structure alone cannot express.
///
/// Returns every problem found, not only the first one.
pub fn validate(doc: &Document) -> Result<(), Vec<ValidationError>> {
    let mut errors = Vec::new();

    if !doc.adm.starts_with("0.") {
        errors.push(ValidationError::UnsupportedVersion(doc.adm.clone()));
    }
    if doc.updated < doc.created {
        errors.push(ValidationError::UpdatedBeforeCreated);
    }

    let mut seen = HashSet::new();
    for (index, block) in doc.content.iter().enumerate() {
        match block {
            Block::Known(KnownBlock::Heading { bid, level, .. }) if !(1..=4).contains(level) => {
                errors.push(ValidationError::InvalidHeadingLevel {
                    bid: bid.to_string(),
                    level: *level,
                });
            }
            Block::Unknown(_) => {
                if let Some(t) = known_type_of_unknown(block) {
                    errors.push(ValidationError::MalformedKnownBlock { index, t });
                }
            }
            Block::Known(_) => {}
        }

        match block.bid() {
            Some(bid) => {
                if !seen.insert(bid.to_owned()) {
                    errors.push(ValidationError::DuplicateBlockId {
                        bid: bid.to_owned(),
                    });
                }
            }
            None => errors.push(ValidationError::MissingBlockId { index }),
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// An unknown block whose `t` is a known type failed to parse as that type.
fn known_type_of_unknown(block: &Block) -> Option<String> {
    block
        .type_name()
        .filter(|t| KNOWN_BLOCK_TYPES.contains(t))
        .map(str::to_owned)
}
