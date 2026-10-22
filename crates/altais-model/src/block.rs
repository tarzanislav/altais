//! Blocks: the units of content inside an object.

use serde::{Deserialize, Serialize};

use crate::ids::{BlobRef, BlockId, ObjectId};

/// Block types this version of the ADM understands.
pub(crate) const KNOWN_BLOCK_TYPES: &[&str] = &[
    "paragraph",
    "heading",
    "list",
    "table",
    "quote",
    "code",
    "divider",
    "bookmark",
    "task_ref",
    "attachment",
];

/// A run of text with optional inline marks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    /// The text itself. A line break inside a block is written as `\n`.
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
    /// Highlighted text in the default highlight colour.
    Highlight,
    /// Superscript.
    Superscript,
    /// Subscript.
    Subscript,
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
    /// Text colour, written as `{"color": "#rrggbb"}`.
    Color {
        /// CSS colour value.
        color: String,
    },
    /// Highlight in a named colour, written as `{"highlight": "purple"}`.
    HighlightColor {
        /// Colour name or CSS colour value.
        highlight: String,
    },
}

/// Kind of a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    /// Bulleted list.
    Bullet,
    /// Numbered list.
    Ordered,
    /// Checklist; each item has a `checked` state.
    Check,
}

/// One item of a [`KnownBlock::List`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListItem {
    /// Stable ID of the item.
    pub bid: BlockId,
    /// Checked state; present for checklist items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checked: Option<bool>,
    /// Content of the item; nested lists are blocks inside it.
    #[serde(default)]
    pub content: Vec<Block>,
}

/// One row of a [`KnownBlock::Table`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRow {
    /// Cells of the row, left to right.
    #[serde(default)]
    pub cells: Vec<TableCell>,
}

fn one() -> u32 {
    1
}

fn is_one(value: &u32) -> bool {
    *value == 1
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// One cell of a [`TableRow`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableCell {
    /// Content of the cell.
    #[serde(default)]
    pub content: Vec<Block>,
    /// Number of columns the cell spans.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub colspan: u32,
    /// Number of rows the cell spans.
    #[serde(default = "one", skip_serializing_if = "is_one")]
    pub rowspan: u32,
    /// Whether the cell is a header cell.
    #[serde(default, skip_serializing_if = "is_false")]
    pub header: bool,
}

/// A block whose type this version of the ADM understands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// A bulleted, numbered or check list.
    List {
        /// Stable block ID.
        bid: BlockId,
        /// Kind of list.
        kind: ListKind,
        /// Items in order.
        #[serde(default)]
        items: Vec<ListItem>,
    },
    /// A table.
    Table {
        /// Stable block ID.
        bid: BlockId,
        /// Rows from top to bottom.
        #[serde(default)]
        rows: Vec<TableRow>,
    },
    /// A quotation.
    Quote {
        /// Stable block ID.
        bid: BlockId,
        /// Quoted blocks.
        #[serde(default)]
        content: Vec<Block>,
    },
    /// Preformatted code.
    Code {
        /// Stable block ID.
        bid: BlockId,
        /// Programming language, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lang: Option<String>,
        /// The code, with line breaks as `\n`.
        text: String,
    },
    /// A horizontal divider.
    Divider {
        /// Stable block ID.
        bid: BlockId,
    },
    /// A link shown as a card, for example a video or web page preview.
    Bookmark {
        /// Stable block ID.
        bid: BlockId,
        /// Target URL.
        url: String,
        /// Title shown on the card, if known.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        title: Option<String>,
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
            | Self::List { bid, .. }
            | Self::Table { bid, .. }
            | Self::Quote { bid, .. }
            | Self::Code { bid, .. }
            | Self::Divider { bid }
            | Self::Bookmark { bid, .. }
            | Self::TaskRef { bid, .. }
            | Self::Attachment { bid, .. } => bid,
        }
    }

    /// Returns the value of the `t` field for this block.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Paragraph { .. } => "paragraph",
            Self::Heading { .. } => "heading",
            Self::List { .. } => "list",
            Self::Table { .. } => "table",
            Self::Quote { .. } => "quote",
            Self::Code { .. } => "code",
            Self::Divider { .. } => "divider",
            Self::Bookmark { .. } => "bookmark",
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

    /// Returns the blocks nested directly inside this block
    /// (quote content, list item content, table cell content).
    pub fn children(&self) -> Vec<&Block> {
        match self {
            Self::Known(KnownBlock::Quote { content, .. }) => content.iter().collect(),
            Self::Known(KnownBlock::List { items, .. }) => {
                items.iter().flat_map(|item| item.content.iter()).collect()
            }
            Self::Known(KnownBlock::Table { rows, .. }) => rows
                .iter()
                .flat_map(|row| row.cells.iter())
                .flat_map(|cell| cell.content.iter())
                .collect(),
            _ => Vec::new(),
        }
    }
}

impl From<KnownBlock> for Block {
    fn from(value: KnownBlock) -> Self {
        Self::Known(value)
    }
}
