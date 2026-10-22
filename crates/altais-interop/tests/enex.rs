//! Tests of the ENEX importer against synthetic fixtures (see `fixtures/generate.py`).

use std::io::Cursor;

use altais_interop::enex::{EnexError, ImportedNote, NoteStatus, Severity, import_all, read_enex};
use altais_model::{Block, KnownBlock, ListKind, Mark, SimpleMark, Span, validate};

const EVERNOTE11: &str = include_str!("fixtures/evernote11.enex");
const LEGACY: &str = include_str!("fixtures/legacy.enex");
const DAMAGED: &str = include_str!("fixtures/damaged.enex");

fn import(enex: &str) -> Vec<ImportedNote> {
    let (notes, summary) = import_all(Cursor::new(enex));
    assert_eq!(
        summary.stopped_early, None,
        "the fixture must be readable to the end"
    );
    notes
}

/// All blocks, including nested ones, in document order.
fn all_blocks(blocks: &[Block]) -> Vec<&Block> {
    let mut out = Vec::new();
    for block in blocks {
        out.push(block);
        out.extend(all_blocks_owned(block.children()));
    }
    out
}

fn all_blocks_owned(blocks: Vec<&Block>) -> Vec<&Block> {
    let mut out = Vec::new();
    for block in blocks {
        out.push(block);
        out.extend(all_blocks_owned(block.children()));
    }
    out
}

fn spans(block: &Block) -> &[Span] {
    match block {
        Block::Known(KnownBlock::Paragraph { text, .. } | KnownBlock::Heading { text, .. }) => text,
        _ => &[],
    }
}

fn plain_text(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in all_blocks(blocks) {
        for span in spans(block) {
            out.push_str(&span.s);
        }
        match block {
            Block::Known(KnownBlock::Code { text, .. }) => out.push_str(text),
            Block::Known(KnownBlock::Bookmark {
                title: Some(title), ..
            }) => out.push_str(title),
            _ => {}
        }
    }
    out
}

/// Text of an ENEX note body: everything between tags, with entities resolved.
fn enml_text(enex: &str, note_index: usize) -> String {
    let body = enex
        .split("<en-note>")
        .nth(note_index + 1)
        .and_then(|rest| rest.split("</en-note>").next())
        .expect("note body");
    let mut out = String::new();
    let mut in_tag = false;
    for c in body.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.replace("&nbsp;", "\u{a0}").replace("&quot;", "\"")
}

fn without_whitespace(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn evernote11_notes_are_imported_completely() {
    let notes = import(EVERNOTE11);
    assert_eq!(notes.len(), 3);
    for (index, note) in notes.iter().enumerate() {
        assert_eq!(
            note.report.status,
            NoteStatus::Imported,
            "{:?}",
            note.report
        );
        assert!(
            note.report
                .warnings
                .iter()
                .all(|w| w.severity == Severity::Info),
            "{:?}",
            note.report.warnings
        );
        assert_eq!(validate(&note.document), Ok(()));
        // No text is lost: every character of the body is in the ADM document.
        assert_eq!(
            without_whitespace(&plain_text(&note.document.content)),
            without_whitespace(&enml_text(EVERNOTE11, index)),
            "text of note {index}"
        );
    }
}

#[test]
fn formatting_marks_bookmarks_and_code_blocks_are_converted() {
    let notes = import(EVERNOTE11);
    let doc = &notes[0].document;
    let blocks = all_blocks(&doc.content);
    let marks: Vec<&Mark> = blocks
        .iter()
        .flat_map(|b| spans(b))
        .flat_map(|s| &s.marks)
        .collect();
    for expected in [
        Mark::Simple(SimpleMark::Italic),
        Mark::Simple(SimpleMark::Underline),
        Mark::Simple(SimpleMark::Strike),
        Mark::Simple(SimpleMark::Bold),
        Mark::Simple(SimpleMark::Subscript),
        Mark::Simple(SimpleMark::Superscript),
        Mark::Color {
            color: "#e59e25".to_owned(),
        },
        Mark::HighlightColor {
            highlight: "purple".to_owned(),
        },
        Mark::Link {
            link: "https://example.com/trasa".to_owned(),
        },
    ] {
        assert!(marks.contains(&&expected), "missing {expected:?}");
    }

    let heading = blocks
        .iter()
        .find_map(|b| match b {
            Block::Known(KnownBlock::Heading { level: 1, text, .. }) => Some(text),
            _ => None,
        })
        .expect("heading");
    assert_eq!(
        heading[0].s, "Výlet\u{a0}do hôr",
        "non-breaking space is kept"
    );

    assert!(blocks.iter().any(|b| matches!(
        b,
        Block::Known(KnownBlock::Bookmark { url, title: Some(title), .. })
            if url == "https://example.com/video" && title == "Video o trase"
    )));
    assert!(blocks.iter().any(|b| matches!(
        b,
        Block::Known(KnownBlock::Code { text, .. }) if text == "{\n  \"km\": 12\n}"
    )));
    // Editor settings hidden in the note are not content.
    assert!(
        notes[0]
            .report
            .warnings
            .iter()
            .any(|w| w.message.contains("editor style settings"))
    );
}

#[test]
fn checklists_keep_their_state_and_nesting() {
    let notes = import(EVERNOTE11);
    let doc = &notes[1].document;
    let Block::Known(KnownBlock::List {
        kind: ListKind::Check,
        items,
        ..
    }) = &doc.content[0]
    else {
        panic!("first block is a checklist: {:?}", doc.content[0]);
    };
    assert_eq!(
        items.iter().map(|i| i.checked).collect::<Vec<_>>(),
        [Some(true), Some(false)]
    );

    // An indented checklist (a list directly inside a list) keeps its nesting.
    let Block::Known(KnownBlock::List { items: outer, .. }) = &doc.content[1] else {
        panic!("second block is a list");
    };
    let Block::Known(KnownBlock::List { items: inner, .. }) = &outer[0].content[0] else {
        panic!("nested list");
    };
    assert_eq!(inner[0].checked, Some(false));

    let Block::Known(KnownBlock::List {
        kind: ListKind::Ordered,
        items,
        ..
    }) = &doc.content[2]
    else {
        panic!("third block is a numbered list");
    };
    assert_eq!(items.len(), 2);
    assert!(items[1].content.iter().any(|b| matches!(
        b,
        Block::Known(KnownBlock::List {
            kind: ListKind::Bullet,
            ..
        })
    )));
}

#[test]
fn tables_and_attachments_are_kept_exactly() {
    let notes = import(EVERNOTE11);
    let note = &notes[2];
    let Block::Known(KnownBlock::Table { rows, .. }) = &note.document.content[0] else {
        panic!("first block is a table");
    };
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].cells.len(), 2);
    assert_eq!(rows[1].cells[0].colspan, 2);

    assert_eq!(note.blobs.len(), 2);
    let attachments: Vec<_> = all_blocks(&note.document.content)
        .into_iter()
        .filter_map(|b| match b {
            Block::Known(KnownBlock::Attachment {
                blob, name, size, ..
            }) => Some((blob, name, *size)),
            _ => None,
        })
        .collect();
    assert_eq!(
        attachments.len(),
        2,
        "both attachments are shown where Evernote showed them"
    );
    for blob in &note.blobs {
        assert_eq!(blob.blob.hex(), blake3::hash(&blob.data).to_hex().as_str());
        assert!(attachments.iter().any(|(b, name, size)| **b == blob.blob
            && **name == blob.name
            && *size == blob.data.len() as u64));
    }
    // An attachment inside a sentence splits the paragraph without losing text.
    let text = plain_text(&note.document.content);
    assert!(text.contains("Leták:") && text.contains("koniec."));

    // Evernote metadata such as the image caption is preserved.
    let resources = &note.document.extra["x-evernote"]["resources"];
    assert_eq!(resources[0]["application-data"], "Priehľadný bod");
    assert_eq!(resources[0]["source-url"], "https://example.com/bod.png");
}

#[test]
fn legacy_elements_are_converted_or_preserved() {
    let notes = import(LEGACY);
    assert_eq!(notes.len(), 1);
    let note = &notes[0];
    let doc = &note.document;

    // Two legacy to-dos become one checklist.
    let Block::Known(KnownBlock::List {
        kind: ListKind::Check,
        items,
        ..
    }) = &doc.content[0]
    else {
        panic!("first block is a checklist: {:?}", doc.content[0]);
    };
    assert_eq!(
        items.iter().map(|i| i.checked).collect::<Vec<_>>(),
        [Some(true), Some(false)]
    );

    let types: Vec<_> = doc.content.iter().filter_map(Block::type_name).collect();
    for expected in [
        "quote",
        "divider",
        "code",
        "evernote_encrypted",
        "missing_attachment",
        "attachment",
    ] {
        assert!(types.contains(&expected), "missing {expected} in {types:?}");
    }
    let encrypted = doc
        .content
        .iter()
        .find(|b| b.type_name() == Some("evernote_encrypted"))
        .expect("encrypted block");
    let Block::Unknown(value) = encrypted else {
        panic!("kept as raw data")
    };
    assert_eq!(value["ciphertext"], "RU5DMI1mnQ7fKjBkvymT8g==");
    assert_eq!(value["hint"], "obvyklé");

    let text = plain_text(&doc.content);
    assert!(text.contains("let x = 1;\nlet y = 2;"));
    assert!(
        text.contains("&foo;"),
        "unknown entities are kept as written"
    );
    assert!(
        text.contains("neznámy prvok"),
        "text of unknown elements is kept"
    );
    assert!(
        all_blocks(&doc.content)
            .iter()
            .flat_map(|b| spans(b))
            .any(|s| s.marks.contains(&Mark::Color {
                color: "#ff0000".to_owned()
            }))
    );

    assert_eq!(doc.tags, ["práca", "projekt"]);
    let evernote = &doc.extra["x-evernote"];
    assert_eq!(evernote["note-attributes"]["author"], "Ján Vzorový");
    assert_eq!(evernote["tasks"][0]["title"], "Zavolať do servisu");

    // The missing attachment makes the note partial, with an explanation.
    assert_eq!(note.report.status, NoteStatus::Partial);
    let problems: Vec<_> = note
        .report
        .warnings
        .iter()
        .filter(|w| w.severity == Severity::Problem)
        .collect();
    assert_eq!(problems.len(), 1);
    assert!(problems[0].message.contains("missing"));
    for topic in [
        "encrypted",
        "task",
        "not shown in the note text",
        "&foo;",
        "<blink>",
    ] {
        assert!(
            note.report
                .warnings
                .iter()
                .any(|w| w.message.contains(topic)),
            "no warning about {topic}: {:?}",
            note.report.warnings
        );
    }
    assert_eq!(validate(doc), Ok(()));
}

#[test]
fn damaged_files_are_reported_and_nothing_is_lost_silently() {
    let (notes, summary) = import_all(Cursor::new(DAMAGED));
    assert_eq!(summary.notes_read, 2);
    assert_eq!(summary.imported, 1);
    assert_eq!(summary.partial, 1);
    let reason = summary.stopped_early.expect("the cut-off note is reported");
    assert!(reason.contains("after 2 notes"), "{reason}");

    // The damaged body is kept unchanged as raw ENML.
    let raw = notes[1]
        .document
        .content
        .iter()
        .find_map(|b| match b {
            Block::Unknown(value) if value["t"] == "evernote_enml" => value["enml"].as_str(),
            _ => None,
        })
        .expect("raw body");
    assert!(raw.contains("<div>Neuzavretý <b>prvok</div>"));
}

#[test]
fn files_that_are_not_evernote_exports_are_rejected() {
    let mut reader = read_enex(Cursor::new("<?xml version=\"1.0\"?><html><body/></html>"));
    assert_eq!(
        reader.next(),
        Some(Err(EnexError::NotEnex("<html>".to_owned())))
    );
    assert_eq!(reader.next(), None);

    let mut empty = read_enex(Cursor::new(""));
    assert!(matches!(empty.next(), Some(Err(EnexError::NotEnex(_)))));
}

#[test]
fn deeply_nested_input_is_rejected_instead_of_crashing() {
    let deep = format!(
        "<en-export>\n<note><title>t</title>{}x{}</note></en-export>",
        "<a>".repeat(2_000),
        "</a>".repeat(2_000)
    );
    let (_, summary) = import_all(Cursor::new(deep));
    assert!(summary.stopped_early.is_some_and(|r| r.contains("nested")));
}
