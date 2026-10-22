//! Tests against the example documents in `spec/adm/examples`.

use altais_model::{
    Block, Document, KnownBlock, ObjectType, ProvenanceKind, ValidationError, validate,
};
use serde_json::{Value, json};

const NOTE: &str = include_str!("../../../spec/adm/examples/note.json");

fn note_with_content(content: Value) -> String {
    let mut doc: Value = serde_json::from_str(NOTE).unwrap();
    doc["content"] = content;
    doc.to_string()
}

#[test]
fn example_note_parses_and_validates() {
    let doc = Document::from_json(NOTE).unwrap();
    assert_eq!(doc.object_type, ObjectType::Note);
    assert_eq!(doc.title.as_deref(), Some("Ponuka na servis kotla"));
    assert_eq!(doc.tags, ["domacnost", "ponuky"]);
    assert_eq!(
        doc.provenance.as_ref().map(|p| p.kind),
        Some(ProvenanceKind::Email)
    );
    assert_eq!(doc.content.len(), 4);
    assert!(
        doc.content
            .iter()
            .all(|block| matches!(block, Block::Known(_)))
    );
    assert_eq!(validate(&doc), Ok(()));
}

#[test]
fn round_trip_keeps_the_document_unchanged() {
    let doc = Document::from_json(NOTE).unwrap();
    let again = Document::from_json(&doc.to_json_pretty().unwrap()).unwrap();
    assert_eq!(doc, again);

    let original: Value = serde_json::from_str(NOTE).unwrap();
    let written: Value = serde_json::to_value(&doc).unwrap();
    assert_eq!(
        original, written,
        "ADM JSON must survive a round trip byte-for-byte in meaning"
    );
}

#[test]
fn unknown_block_types_are_preserved_verbatim() {
    let diagram =
        json!({ "bid": "d1x9Qa", "t": "diagram", "lang": "mermaid", "src": "graph TD; A-->B" });
    let doc = Document::from_json(&note_with_content(json!([diagram.clone()]))).unwrap();

    assert!(matches!(&doc.content[0], Block::Unknown(_)));
    assert_eq!(doc.content[0].type_name(), Some("diagram"));
    assert_eq!(validate(&doc), Ok(()));

    let written = serde_json::to_value(&doc).unwrap();
    assert_eq!(written["content"][0], diagram);
}

#[test]
fn unknown_object_types_are_preserved() {
    let mut value: Value = serde_json::from_str(NOTE).unwrap();
    value["type"] = json!("journal_day");
    let doc: Document = serde_json::from_value(value).unwrap();
    assert_eq!(doc.object_type, ObjectType::Other("journal_day".to_owned()));
    assert_eq!(
        serde_json::to_value(&doc).unwrap()["type"],
        json!("journal_day")
    );
}

#[test]
fn duplicate_block_ids_are_rejected() {
    let json = note_with_content(json!([
        { "bid": "same01", "t": "paragraph", "text": [{ "s": "a" }] },
        { "bid": "same01", "t": "paragraph", "text": [{ "s": "b" }] }
    ]));
    let doc = Document::from_json(&json).unwrap();
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::DuplicateBlockId {
            bid: "same01".to_owned()
        }])
    );
}

#[test]
fn malformed_known_blocks_are_reported_not_hidden() {
    // A heading without `level` must not pass silently as an unknown block.
    let json = note_with_content(json!([{ "bid": "h1abcd", "t": "heading", "text": [] }]));
    let doc = Document::from_json(&json).unwrap();
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::MalformedKnownBlock {
            index: 0,
            t: "heading".to_owned()
        }])
    );
}

#[test]
fn heading_levels_are_limited() {
    let json =
        note_with_content(json!([{ "bid": "h5abcd", "t": "heading", "level": 5, "text": [] }]));
    let doc = Document::from_json(&json).unwrap();
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::InvalidHeadingLevel {
            bid: "h5abcd".to_owned(),
            level: 5
        }])
    );
}

#[test]
fn blocks_without_id_are_reported() {
    let json = note_with_content(json!([{ "t": "future_block" }]));
    let doc = Document::from_json(&json).unwrap();
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::MissingBlockId { index: 0 }])
    );
}

#[test]
fn invalid_blob_reference_is_not_a_valid_attachment() {
    let json = note_with_content(json!([{
        "bid": "att001", "t": "attachment", "blob": "md5:abc",
        "name": "a.pdf", "mime": "application/pdf", "size": 1
    }]));
    let doc = Document::from_json(&json).unwrap();
    assert!(!matches!(
        doc.content[0],
        Block::Known(KnownBlock::Attachment { .. })
    ));
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::MalformedKnownBlock {
            index: 0,
            t: "attachment".to_owned()
        }])
    );
}

#[test]
fn updated_before_created_is_rejected() {
    let mut value: Value = serde_json::from_str(NOTE).unwrap();
    value["updated"] = json!("2020-01-01T00:00:00Z");
    let doc: Document = serde_json::from_value(value).unwrap();
    assert_eq!(
        validate(&doc),
        Err(vec![ValidationError::UpdatedBeforeCreated])
    );
}
