//! Objects and their metadata.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ADM_VERSION;
use crate::block::Block;
use crate::ids::ObjectId;

/// Type of an object.
///
/// Unknown types are preserved as [`ObjectType::Other`] so that newer data
/// survives a round trip through an older reader.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub enum ObjectType {
    /// A note.
    Note,
    /// A task.
    Task,
    /// An attachment and its metadata.
    Attachment,
    /// A captured web page.
    WebClip,
    /// A tag.
    Tag,
    /// A saved search.
    SavedSearch,
    /// A type this version does not know.
    Other(String),
}

impl ObjectType {
    /// Returns the value written in the `type` field.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Note => "note",
            Self::Task => "task",
            Self::Attachment => "attachment",
            Self::WebClip => "web_clip",
            Self::Tag => "tag",
            Self::SavedSearch => "saved_search",
            Self::Other(name) => name.as_str(),
        }
    }
}

impl From<String> for ObjectType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "note" => Self::Note,
            "task" => Self::Task,
            "attachment" => Self::Attachment,
            "web_clip" => Self::WebClip,
            "tag" => Self::Tag,
            "saved_search" => Self::SavedSearch,
            _ => Self::Other(value),
        }
    }
}

impl From<ObjectType> for String {
    fn from(value: ObjectType) -> Self {
        match value {
            ObjectType::Other(name) => name,
            known => known.as_str().to_owned(),
        }
    }
}

/// Where a piece of content came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProvenanceKind {
    /// Written by the user.
    Manual,
    /// Captured from the web.
    Web,
    /// Received by e-mail.
    Email,
    /// Imported from another application.
    Import,
    /// Produced by an AI feature.
    Ai,
    /// Added through the write-only capture key (clipper, share sheet, script).
    Capture,
}

/// Origin of an object: what it came from and when it was captured.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Kind of source.
    pub kind: ProvenanceKind,
    /// The source itself, for example a URL, an e-mail address or a file name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// When the content was captured.
    pub captured: DateTime<Utc>,
}

/// One object in the ALTAIS Document Model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// ADM version of this document, for example `"0.1"`.
    pub adm: String,
    /// Stable identity of the object.
    pub id: ObjectId,
    /// Type of the object.
    #[serde(rename = "type")]
    pub object_type: ObjectType,
    /// Title, if the object has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// When the object was created.
    pub created: DateTime<Utc>,
    /// When the object was last changed.
    pub updated: DateTime<Utc>,
    /// The notebook that holds this object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notebook: Option<ObjectId>,
    /// Tags, by name.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Main language of the content as a BCP 47 tag, for example `sk` or `zh-Hans`.
    ///
    /// Chooses search tokenization, OCR language and spell checking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Where the object came from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<Provenance>,
    /// Content blocks, in order.
    #[serde(default)]
    pub content: Vec<Block>,
}

impl Document {
    /// Creates an empty note with a new identity, created now.
    pub fn new_note(title: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            adm: ADM_VERSION.to_owned(),
            id: ObjectId::new(),
            object_type: ObjectType::Note,
            title: Some(title.into()),
            created: now,
            updated: now,
            notebook: None,
            tags: Vec::new(),
            lang: None,
            provenance: None,
            content: Vec::new(),
        }
    }

    /// Parses a document from ADM JSON. Call [`crate::validate`] afterwards.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Writes the document as pretty-printed ADM JSON.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_type_known_and_unknown_values() {
        assert_eq!(ObjectType::from("web_clip".to_owned()), ObjectType::WebClip);
        let other = ObjectType::from("journal_day".to_owned());
        assert_eq!(other, ObjectType::Other("journal_day".to_owned()));
        assert_eq!(String::from(other), "journal_day");
    }

    #[test]
    fn new_note_is_valid() {
        let note = Document::new_note("Nápad");
        assert_eq!(note.object_type, ObjectType::Note);
        assert_eq!(crate::validate(&note), Ok(()));
    }
}
