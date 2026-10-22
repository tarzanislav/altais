//! Blocks: the units of content inside an object.

use serde::{Deserialize, Serialize};

use crate::ids::{BlobRef, BlockId, ObjectId};

/// Block types this version of the ADM understands.
pub(crate) const KNOWN_BLOCK_TYPES: &[&str] = &["paragraph", "heading", "task_ref", "attachment"];

/// A run of text with optional inline marks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// The text itself.
    pub s: String,
    /// Inline formatting applied to the whole run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub marks: Vec<Mark>,
}

impl Span {
    /// Creates an unformatted run of text.
    pub fn plain(s: impl Into<String>) -> Self {
        Self {
            s: s.into(),
            marks: Vec::new(),
        }
    }
}

/// Inline formatting without parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimpleMark {
    /// Bold text.
    Bold,
    /// Italic text.
    Italic,
    /// Underlined text.
    Underline,
    /// Struck-through text.
    Strike,
    /// Inline code.
    Code,
    /// Highlighted text.
    Highlight,
}

/// Inline formatting of a [`Span`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Mark {
    /// A mark without parameters, written as a string such as `"bold"`.
    Simple(SimpleMark),
    /// A hyperlink, written as `{"link": "<url>"}`.
    Link {
        /// Target URL or `altais://` link.
        link: String,
    },
}

/// A block whose type this version of the ADM understands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum KnownBlock {
    /// A paragraph of text.
    Paragraph {
        /// Stable block ID.
        bid: BlockId,
        /// Text runs.
        #[serde(default)]
        text: Vec<Span>,
    },
    /// A heading, level 1 to 4.
    Heading {
        /// Stable block ID.
        bid: BlockId,
        /// Heading level, 1 to 4.
        level: u8,
        /// Text runs.
        #[serde(default)]
        text: Vec<Span>,
    },
    /// A reference that displays a task object inside a note.
    ///
    /// The task has a single identity: completing it here completes it everywhere.
    TaskRef {
        /// Stable block ID.
        bid: BlockId,
        /// The task object shown by this block.
        task: ObjectId,
    },
    /// An attachment, referenced by the hash of its original bytes.
    Attachment {
        /// Stable block ID.
        bid: BlockId,
        /// Content address of the original file.
        blob: BlobRef,
        /// Original file name.
        name: String,
        /// MIME type, for example `application/pdf`.
        mime: String,
        /// Size of the original file in bytes.
        size: u64,
    },
}

impl KnownBlock {
    /// Returns the stable block ID.
    pub fn bid(&self) -> &BlockId {
        match self {
            Self::Paragraph { bid, .. }
            | Self::Heading { bid, .. }
            | Self::TaskRef { bid, .. }
            | Self::Attachment { bid, .. } => bid,
        }
    }

    /// Returns the value of the `t` field for this block.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Paragraph { .. } => "paragraph",
            Self::Heading { .. } => "heading",
            Self::TaskRef { .. } => "task_ref",
            Self::Attachment { .. } => "attachment",
        }
    }
}

/// One block of content.
///
/// Blocks of an unknown type are kept as raw JSON and written back unchanged,
/// so content created by a newer version of ALTAIS is never lost.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Block {
    /// A block this version understands.
    Known(KnownBlock),
    /// A block of an unknown type, preserved verbatim.
    Unknown(serde_json::Value),
}

impl Block {
    /// Returns the block ID, if present.
    pub fn bid(&self) -> Option<&str> {
        match self {
            Self::Known(block) => Some(block.bid().as_str()),
            Self::Unknown(value) => value.get("bid").and_then(serde_json::Value::as_str),
        }
    }

    /// Returns the value of the `t` field, if present.
    pub fn type_name(&self) -> Option<&str> {
        match self {
            Self::Known(block) => Some(block.type_name()),
            Self::Unknown(value) => value.get("t").and_then(serde_json::Value::as_str),
        }
    }
}

impl From<KnownBlock> for Block {
    fn from(value: KnownBlock) -> Self {
        Self::Known(value)
    }
}
