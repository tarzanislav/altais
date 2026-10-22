//! The two CRDT engines behind one small interface (the same shape the
//! `DocEngine` trait in `altais-core` will have).

use crate::trace::Op;

/// A CRDT engine holding one note's text.
pub trait Engine {
    /// Name used in the report.
    const NAME: &'static str;
    /// One replica of a note.
    type Doc;
    /// What a device has seen (version vector / state vector).
    type Version;

    /// An empty note on device `peer`.
    fn new(peer: u64) -> Self::Doc;
    /// Applies edits as one local transaction.
    fn apply(doc: &mut Self::Doc, ops: &[Op]);
    /// Applies edits as one transaction and returns them as the sync message
    /// that would be sent to other devices right away.
    fn apply_and_encode(doc: &mut Self::Doc, ops: &[Op]) -> Vec<u8>;
    /// The plain text.
    fn text(doc: &Self::Doc) -> String;
    /// Full state including history, as stored on disk and sent to a new device.
    fn snapshot(doc: &Self::Doc) -> Vec<u8>;
    /// Smallest form of the current state the engine offers
    /// (Loro: shallow snapshot without history; Yrs: v2 encoding).
    fn compact_snapshot(doc: &Self::Doc) -> Vec<u8>;
    /// Opens a note from a snapshot on device `peer`.
    fn load(bytes: &[u8], peer: u64) -> Self::Doc;
    /// What this device has seen.
    fn version(doc: &Self::Doc) -> Self::Version;
    /// Changes that a device at `version` has not seen.
    fn updates_since(doc: &Self::Doc, version: &Self::Version) -> Vec<u8>;
    /// Merges changes from another device.
    fn import(doc: &mut Self::Doc, bytes: &[u8]);
    /// Number of bold runs (to check that formatting arrived).
    fn bold_runs(doc: &Self::Doc) -> usize;
}

// ---------------- Loro ----------------

/// Loro 1.x rich text.
pub struct LoroEngine;

/// A Loro replica.
pub struct LoroNote {
    doc: loro::LoroDoc,
    text: loro::LoroText,
}

impl LoroNote {
    fn wrap(doc: loro::LoroDoc) -> Self {
        let text = doc.get_text("text");
        Self { doc, text }
    }
}

fn loro_doc(peer: u64) -> loro::LoroDoc {
    let doc = loro::LoroDoc::new();
    doc.set_peer_id(peer).expect("valid peer id");
    doc
}

impl Engine for LoroEngine {
    const NAME: &'static str = "Loro 1.17.1";
    type Doc = LoroNote;
    type Version = loro::VersionVector;

    fn new(peer: u64) -> LoroNote {
        LoroNote::wrap(loro_doc(peer))
    }

    fn apply(note: &mut LoroNote, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Insert { pos, text } => note.text.insert(*pos, text),
                Op::Delete { pos, len } => note.text.delete(*pos, *len),
                Op::Bold { start, end } => note.text.mark(*start..*end, "bold", true),
            }
            .expect("edit within the text");
        }
        note.doc.commit();
    }

    fn apply_and_encode(note: &mut LoroNote, ops: &[Op]) -> Vec<u8> {
        let before = note.doc.oplog_vv();
        Self::apply(note, ops);
        Self::updates_since(note, &before)
    }

    fn text(note: &LoroNote) -> String {
        note.text.to_string()
    }

    fn snapshot(note: &LoroNote) -> Vec<u8> {
        note.doc.export(loro::ExportMode::Snapshot).expect("export")
    }

    fn compact_snapshot(note: &LoroNote) -> Vec<u8> {
        let frontiers = note.doc.oplog_frontiers();
        note.doc
            .export(loro::ExportMode::shallow_snapshot(&frontiers))
            .expect("export")
    }

    fn load(bytes: &[u8], peer: u64) -> LoroNote {
        let doc = loro_doc(peer);
        doc.import(bytes).expect("valid snapshot");
        LoroNote::wrap(doc)
    }

    fn version(note: &LoroNote) -> loro::VersionVector {
        note.doc.oplog_vv()
    }

    fn updates_since(note: &LoroNote, version: &loro::VersionVector) -> Vec<u8> {
        note.doc
            .export(loro::ExportMode::updates(version))
            .expect("export")
    }

    fn import(note: &mut LoroNote, bytes: &[u8]) {
        note.doc.import(bytes).expect("valid update");
    }

    fn bold_runs(note: &LoroNote) -> usize {
        note.text
            .to_delta()
            .iter()
            .filter(|d| match d {
                loro::TextDelta::Insert { attributes, .. } => {
                    attributes.as_ref().is_some_and(|a| a.contains_key("bold"))
                }
                _ => false,
            })
            .count()
    }
}

// ---------------- Yrs ----------------

/// Yrs 0.28 (the Rust port of Yjs) rich text.
pub struct YrsEngine;

/// A Yrs replica.
pub struct YrsNote {
    doc: yrs::Doc,
    text: yrs::TextRef,
}

fn yrs_doc(peer: u64) -> YrsNote {
    let doc = yrs::Doc::with_options(yrs::Options {
        client_id: yrs::ClientID::new(peer),
        offset_kind: yrs::OffsetKind::Utf16,
        ..Default::default()
    });
    let text = doc.get_or_insert_text("text");
    YrsNote { doc, text }
}

fn yrs_edit(text: &yrs::TextRef, txn: &mut yrs::TransactionMut, ops: &[Op]) {
    use yrs::Text;
    for op in ops {
        match op {
            Op::Insert { pos, text: t } => text.insert(txn, *pos as u32, t),
            Op::Delete { pos, len } => text.remove_range(txn, *pos as u32, *len as u32),
            Op::Bold { start, end } => {
                text.format(txn, *start as u32, (*end - *start) as u32, bold_attrs())
            }
        }
    }
}

fn bold_attrs() -> yrs::types::Attrs {
    let mut attrs = yrs::types::Attrs::new();
    attrs.insert("bold".into(), yrs::Any::Bool(true));
    attrs
}

impl Engine for YrsEngine {
    const NAME: &'static str = "Yrs 0.28.0";
    type Doc = YrsNote;
    type Version = yrs::StateVector;

    fn new(peer: u64) -> YrsNote {
        yrs_doc(peer)
    }

    fn apply(note: &mut YrsNote, ops: &[Op]) {
        use yrs::Transact;
        let mut txn = note.doc.transact_mut();
        yrs_edit(&note.text, &mut txn, ops);
        // Dropping the transaction commits it.
    }

    fn apply_and_encode(note: &mut YrsNote, ops: &[Op]) -> Vec<u8> {
        use yrs::Transact;
        let mut txn = note.doc.transact_mut();
        yrs_edit(&note.text, &mut txn, ops);
        // What Yjs sends per transaction (the update event).
        txn.encode_update_v1()
    }

    fn text(note: &YrsNote) -> String {
        use yrs::{GetString, Transact};
        note.text.get_string(&note.doc.transact())
    }

    fn snapshot(note: &YrsNote) -> Vec<u8> {
        use yrs::{ReadTxn, Transact};
        note.doc
            .transact()
            .encode_state_as_update_v1(&yrs::StateVector::default())
    }

    fn compact_snapshot(note: &YrsNote) -> Vec<u8> {
        use yrs::{ReadTxn, Transact};
        note.doc
            .transact()
            .encode_state_as_update_v2(&yrs::StateVector::default())
    }

    fn load(bytes: &[u8], peer: u64) -> YrsNote {
        use yrs::Transact;
        use yrs::updates::decoder::Decode;
        let note = yrs_doc(peer);
        note.doc
            .transact_mut()
            .apply_update(yrs::Update::decode_v1(bytes).expect("valid snapshot"))
            .expect("apply snapshot");
        note
    }

    fn version(note: &YrsNote) -> yrs::StateVector {
        use yrs::{ReadTxn, Transact};
        note.doc.transact().state_vector()
    }

    fn updates_since(note: &YrsNote, version: &yrs::StateVector) -> Vec<u8> {
        use yrs::{ReadTxn, Transact};
        note.doc.transact().encode_state_as_update_v1(version)
    }

    fn import(note: &mut YrsNote, bytes: &[u8]) {
        use yrs::Transact;
        use yrs::updates::decoder::Decode;
        note.doc
            .transact_mut()
            .apply_update(yrs::Update::decode_v1(bytes).expect("valid update"))
            .expect("apply update");
    }

    fn bold_runs(note: &YrsNote) -> usize {
        use yrs::Transact;
        use yrs::types::text::YChange;
        let txn = note.doc.transact();
        yrs::Text::diff(&note.text, &txn, YChange::identity)
            .iter()
            .filter(|d| {
                d.attributes
                    .as_ref()
                    .is_some_and(|a| a.contains_key("bold"))
            })
            .count()
    }
}
