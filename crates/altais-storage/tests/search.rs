//! Search across the interface languages.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use altais_model::{Block, Document};
use altais_storage::{Archive, KdfParams, OsKeyStore};

struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let path = std::env::temp_dir().join(format!("altais-search-{name}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const FAST: KdfParams = KdfParams {
    memory_kib: 1024,
    iterations: 1,
    parallelism: 1,
};

fn paragraph(bid: &str, text: &str) -> Block {
    serde_json::from_value(serde_json::json!({"bid": bid, "t": "paragraph", "text": [{"s": text}]}))
        .unwrap()
}

fn note(title: &str, body: &str) -> Document {
    let mut doc = Document::new_note(title);
    doc.content = vec![paragraph("b0001", body)];
    doc
}

fn archive(dir: &TempDir) -> Archive {
    Archive::create_with_kdf(&dir.0, Some(Arc::new(OsKeyStore::mock())), None, FAST)
        .unwrap()
        .0
}

fn titles(archive: &Archive, query: &str) -> Vec<String> {
    archive
        .search(query, 20)
        .unwrap()
        .into_iter()
        .map(|hit| hit.note.title)
        .collect()
}

#[test]
fn finds_words_without_diacritics_in_every_language() {
    let dir = TempDir::new("languages");
    let mut a = archive(&dir);
    for (title, body) in [
        ("Výlet na Štrbské Pleso", "Kúpiť lístky a zbaliť batoh."),
        ("Česky", "Příliš žluťoučký kůň úpěl ďábelské ódy."),
        ("Magyar", "Árvíztűrő tükörfúrógép"),
        ("Polski", "Zażółć gęślą jaźń w Łodzi"),
        ("Русский", "Новогодняя ёлка и подарки"),
        ("中文", "加密笔记很重要，同步也很方便。"),
    ] {
        a.put_note(&note(title, body)).unwrap();
    }
    assert_eq!(titles(&a, "vylet"), ["Výlet na Štrbské Pleso"]);
    assert_eq!(titles(&a, "VÝLET"), ["Výlet na Štrbské Pleso"]);
    assert_eq!(
        titles(&a, "strb pleso"),
        ["Výlet na Štrbské Pleso"],
        "word prefixes"
    );
    assert_eq!(
        titles(&a, "listky"),
        ["Výlet na Štrbské Pleso"],
        "body text"
    );
    assert_eq!(titles(&a, "zlutoucky kun"), ["Česky"]);
    assert_eq!(titles(&a, "tukorfurogep"), ["Magyar"]);
    assert_eq!(titles(&a, "lodzi"), ["Polski"], "ł is folded to l");
    assert_eq!(titles(&a, "елка"), ["Русский"], "ё is folded to е");
    assert_eq!(
        titles(&a, "笔记"),
        ["中文"],
        "a two-character word inside a sentence"
    );
    assert_eq!(titles(&a, "同步"), ["中文"]);
    assert!(titles(&a, "笔同").is_empty(), "characters must be adjacent");
    assert!(
        titles(&a, "vylet magyar").is_empty(),
        "every word must match"
    );
    assert!(titles(&a, "").is_empty());
}

#[test]
fn quoted_phrases_title_weight_and_updates() {
    let dir = TempDir::new("phrases");
    let mut a = archive(&dir);
    a.put_note(&note("Nákupný zoznam", "mlieko, chlieb, Tatry čaj"))
        .unwrap();
    let mut tatry = note("Tatry", "plán na leto");
    a.put_note(&tatry).unwrap();
    assert_eq!(
        titles(&a, "tatry"),
        ["Tatry", "Nákupný zoznam"],
        "a title match ranks first"
    );
    assert_eq!(
        titles(&a, "\"mlieko tatry\""),
        Vec::<String>::new(),
        "phrase words must be adjacent"
    );
    assert_eq!(titles(&a, "\"tatry caj\""), ["Nákupný zoznam"]);

    tatry.content = vec![paragraph("b0001", "zimná dovolenka")];
    a.put_note(&tatry).unwrap();
    assert!(
        titles(&a, "leto").is_empty(),
        "old text is gone from the index"
    );
    assert_eq!(titles(&a, "dovolenka"), ["Tatry"]);
}

#[test]
fn an_imported_evernote_export_is_searchable() {
    let dir = TempDir::new("enex");
    let mut a = archive(&dir);
    let enex = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../altais-interop/tests/fixtures/evernote11.enex"
    ))
    .unwrap();
    let (notes, _) = altais_interop::enex::import_all(&enex[..]);
    for imported in &notes {
        for blob in &imported.blobs {
            a.put_blob(&blob.data).unwrap();
        }
        a.put_note(&imported.document).unwrap();
    }
    assert_eq!(a.note_count().unwrap(), notes.len());
    // Every note can be found by a word of its title.
    for imported in &notes {
        let title = imported.document.title.clone().unwrap_or_default();
        let Some(word) = title.split_whitespace().find(|w| w.chars().count() > 3) else {
            continue;
        };
        let hits = a.search(word, 50).unwrap();
        assert!(
            hits.iter().any(|h| h.note.id == imported.document.id),
            "{word:?} should find {title:?}"
        );
    }
}
