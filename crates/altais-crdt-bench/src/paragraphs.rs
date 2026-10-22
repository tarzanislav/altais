//! The structure the editor actually stores: a note as a list of paragraphs,
//! each its own text (as loro-prosemirror and y-prosemirror do). Long notes
//! then never become one long text, which matters for engines whose cost
//! grows with the length of a single text.

/// A note stored as a list of paragraph texts.
pub trait ParagraphEngine {
    /// Name used in the report.
    const NAME: &'static str;
    /// One replica.
    type Doc;

    /// An empty note on device `peer`.
    fn new(peer: u64) -> Self::Doc;
    /// Appends an empty paragraph (one transaction).
    fn push_paragraph(doc: &mut Self::Doc);
    /// Inserts text into paragraph `para` (one transaction).
    fn insert(doc: &mut Self::Doc, para: usize, pos: usize, text: &str);
    /// The plain text, paragraphs joined by line breaks.
    fn text(doc: &Self::Doc) -> String;
    /// Full state including history.
    fn snapshot(doc: &Self::Doc) -> Vec<u8>;
    /// Opens a note from a snapshot.
    fn load(bytes: &[u8], peer: u64) -> Self::Doc;
}

/// Loro: a list of `LoroText` containers.
pub struct LoroParagraphs;

/// Loro replica with handles to its paragraph texts.
pub struct LoroParaDoc {
    doc: loro::LoroDoc,
    list: loro::LoroList,
    paras: Vec<loro::LoroText>,
}

impl ParagraphEngine for LoroParagraphs {
    const NAME: &'static str = "Loro 1.17.1";
    type Doc = LoroParaDoc;

    fn new(peer: u64) -> LoroParaDoc {
        let doc = loro::LoroDoc::new();
        doc.set_peer_id(peer).expect("valid peer id");
        let list = doc.get_list("paragraphs");
        LoroParaDoc {
            doc,
            list,
            paras: Vec::new(),
        }
    }

    fn push_paragraph(d: &mut LoroParaDoc) {
        let text = d
            .list
            .insert_container(d.paras.len(), loro::LoroText::new())
            .expect("insert paragraph");
        d.paras.push(text);
        d.doc.commit();
    }

    fn insert(d: &mut LoroParaDoc, para: usize, pos: usize, text: &str) {
        d.paras[para].insert(pos, text).expect("insert text");
        d.doc.commit();
    }

    fn text(d: &LoroParaDoc) -> String {
        d.paras
            .iter()
            .map(|p| p.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn snapshot(d: &LoroParaDoc) -> Vec<u8> {
        d.doc.export(loro::ExportMode::Snapshot).expect("export")
    }

    fn load(bytes: &[u8], peer: u64) -> LoroParaDoc {
        let mut d = Self::new(peer);
        d.doc.import(bytes).expect("valid snapshot");
        d.paras = (0..d.list.len())
            .map(|i| match d.list.get(i) {
                Some(loro::ValueOrContainer::Container(loro::Container::Text(t))) => t,
                _ => panic!("paragraph {i} is not a text"),
            })
            .collect();
        d
    }
}

/// Yrs: an array of `TextRef`s.
pub struct YrsParagraphs;

/// Yrs replica with handles to its paragraph texts.
pub struct YrsParaDoc {
    doc: yrs::Doc,
    array: yrs::ArrayRef,
    paras: Vec<yrs::TextRef>,
}

impl ParagraphEngine for YrsParagraphs {
    const NAME: &'static str = "Yrs 0.28.0";
    type Doc = YrsParaDoc;

    fn new(peer: u64) -> YrsParaDoc {
        let doc = yrs::Doc::with_options(yrs::Options {
            client_id: yrs::ClientID::new(peer),
            offset_kind: yrs::OffsetKind::Utf16,
            ..Default::default()
        });
        let array = doc.get_or_insert_array("paragraphs");
        YrsParaDoc {
            doc,
            array,
            paras: Vec::new(),
        }
    }

    fn push_paragraph(d: &mut YrsParaDoc) {
        use yrs::{Array, Transact};
        let mut txn = d.doc.transact_mut();
        let text = d
            .array
            .insert(&mut txn, d.paras.len() as u32, yrs::TextPrelim::new(""));
        drop(txn);
        d.paras.push(text);
    }

    fn insert(d: &mut YrsParaDoc, para: usize, pos: usize, text: &str) {
        use yrs::{Text, Transact};
        let mut txn = d.doc.transact_mut();
        d.paras[para].insert(&mut txn, pos as u32, text);
    }

    fn text(d: &YrsParaDoc) -> String {
        use yrs::{GetString, Transact};
        let txn = d.doc.transact();
        d.paras
            .iter()
            .map(|p| p.get_string(&txn))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn snapshot(d: &YrsParaDoc) -> Vec<u8> {
        use yrs::{ReadTxn, Transact};
        d.doc
            .transact()
            .encode_state_as_update_v1(&yrs::StateVector::default())
    }

    fn load(bytes: &[u8], peer: u64) -> YrsParaDoc {
        use yrs::updates::decoder::Decode;
        use yrs::{Array, Transact};
        let mut d = Self::new(peer);
        d.doc
            .transact_mut()
            .apply_update(yrs::Update::decode_v1(bytes).expect("valid snapshot"))
            .expect("apply snapshot");
        let txn = d.doc.transact();
        let paras = (0..d.array.len(&txn))
            .map(|i| match d.array.get(&txn, i) {
                Some(yrs::Out::YText(t)) => t,
                _ => panic!("paragraph {i} is not a text"),
            })
            .collect();
        drop(txn);
        d.paras = paras;
        d
    }
}

/// Types `words` words keystroke by keystroke, a new paragraph every 40 words.
pub fn type_note<E: ParagraphEngine>(doc: &mut E::Doc, words: usize, seed: u64) {
    let mut rng = crate::Rng::new(seed);
    let mut para = 0;
    let mut pos = 0;
    E::push_paragraph(doc);
    for i in 0..words {
        if i > 0 && i % 40 == 0 {
            E::push_paragraph(doc);
            para += 1;
            pos = 0;
        } else if i > 0 {
            E::insert(doc, para, pos, " ");
            pos += 1;
        }
        for ch in crate::trace::word(&mut rng).chars() {
            let mut buf = [0u8; 4];
            E::insert(doc, para, pos, ch.encode_utf8(&mut buf));
            pos += 1;
        }
    }
}
