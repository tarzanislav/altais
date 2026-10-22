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
    /// A block or list item has no `bid` field.
    #[error("block #{index} (or a block nested in it) has no block id")]
    MissingBlockId {
        /// Position of the top-level block in `content`.
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
    #[error(
        "block #{index} (or a block nested in it) has type {t:?} but does not match its schema"
    )]
    MalformedKnownBlock {
        /// Position of the top-level block in `content`.
        index: usize,
        /// The block type.
        t: String,
    },
    /// The `lang` field is not a BCP 47 language tag such as `sk` or `zh-Hans`.
    #[error("invalid language tag {0:?}")]
    InvalidLanguageTag(String),
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
    if let Some(lang) = doc.lang.as_deref().filter(|l| !is_language_tag(l)) {
        errors.push(ValidationError::InvalidLanguageTag(lang.to_owned()));
    }

    let mut seen = HashSet::new();
    for (index, block) in doc.content.iter().enumerate() {
        check_block(block, index, &mut seen, &mut errors);
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Checks one block and, recursively, the blocks nested inside it.
///
/// `index` is the position of the top-level block that contains `block`.
fn check_block(
    block: &Block,
    index: usize,
    seen: &mut HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    match block {
        Block::Known(KnownBlock::Heading { bid, level, .. }) if !(1..=4).contains(level) => {
            errors.push(ValidationError::InvalidHeadingLevel {
                bid: bid.to_string(),
                level: *level,
            });
        }
        Block::Known(KnownBlock::List { items, .. }) => {
            for item in items {
                note_bid(Some(item.bid.as_str()), index, seen, errors);
            }
        }
        Block::Unknown(_) => {
            if let Some(t) = known_type_of_unknown(block) {
                errors.push(ValidationError::MalformedKnownBlock { index, t });
            }
        }
        Block::Known(_) => {}
    }

    note_bid(block.bid(), index, seen, errors);

    for child in block.children() {
        check_block(child, index, seen, errors);
    }
}

/// Records a block ID and reports it when it is missing or repeated.
fn note_bid(
    bid: Option<&str>,
    index: usize,
    seen: &mut HashSet<String>,
    errors: &mut Vec<ValidationError>,
) {
    match bid {
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

/// Checks the shape of a BCP 47 tag: a 2–3 letter language subtag followed by
/// optional subtags of 1–8 letters or digits (for example `sk`, `zh-Hans`, `pt-BR`).
fn is_language_tag(tag: &str) -> bool {
    let mut parts = tag.split('-');
    let primary = parts
        .next()
        .is_some_and(|p| (2..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphabetic()));
    primary
        && parts.all(|p| (1..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// An unknown block whose `t` is a known type failed to parse as that type.
fn known_type_of_unknown(block: &Block) -> Option<String> {
    block
        .type_name()
        .filter(|t| KNOWN_BLOCK_TYPES.contains(t))
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::is_language_tag;

    #[test]
    fn language_tags() {
        for ok in [
            "sk", "cs", "en", "hu", "pl", "ru", "zh-Hans", "zh-Hant", "pt-BR",
        ] {
            assert!(is_language_tag(ok), "{ok} should be accepted");
        }
        for bad in [
            "",
            "s",
            "slovak",
            "sk_SK",
            "zh-",
            "sk-toolongsubtag",
            "slovenčina",
        ] {
            assert!(!is_language_tag(bad), "{bad} should be rejected");
        }
    }
}
