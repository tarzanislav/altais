//! The operations the desktop app calls, end to end.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use altais_core::{Core, NoteEdit, Secret};
use altais_storage::{KeyStore, OsKeyStore};
use serde_json::json;

struct TempDir(PathBuf);
impl TempDir {
    fn new(name: &str) -> Self {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        Self(std::env::temp_dir().join(format!("altais-core-{name}-{suffix}")))
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../altais-interop/tests/fixtures"
    ))
    .join(name)
}

#[test]
fn first_run_import_edit_search_and_reopen() {
    let dir = TempDir::new("flow");
    let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
    let mut core = Core::new(dir.0.clone(), Some(keystore.clone()));

    let status = core.status();
    assert!(!status.exists && !status.unlocked && status.has_keystore);
    let recovery = core.create(None).unwrap();
    assert!(recovery.starts_with("ALTAIS-"));
    assert!(core.status().unlocked);

    // Import an Evernote export: one notebook named after the file.
    let outcomes = core.import_enex(&[fixture("evernote11.enex")]).unwrap();
    assert_eq!(outcomes[0].notebook, "evernote11");
    assert_eq!(outcomes[0].stored, 3);
    let notebooks = core.notebooks().unwrap();
    assert_eq!(notebooks.len(), 1);
    assert_eq!(notebooks[0].count, 3);

    // A new note, edited in the editor.
    let note = core.new_note(Some(&notebooks[0].id)).unwrap();
    assert_eq!(note.title, "");
    let saved = core
        .save_note(NoteEdit {
            id: note.id.clone(),
            title: "Nákup na víkend".into(),
            content: vec![json!({"t": "paragraph", "bid": "p0001", "text": [{"s": "chlieb, mlieko, bryndza"}]})],
        })
        .unwrap();
    assert_eq!(saved.title, "Nákup na víkend");
    assert_eq!(core.search("bryndza").unwrap()[0].id, note.id);
    assert_eq!(core.notebooks().unwrap()[0].count, 4);

    // Saving keeps what the editor does not show (e.g. Evernote data).
    let imported = core
        .notes(Some(&notebooks[0].id))
        .unwrap()
        .into_iter()
        .find(|n| n.title == "Zoznamy")
        .unwrap();
    let before = core.note(&imported.id).unwrap();
    core.save_note(NoteEdit {
        id: imported.id.clone(),
        title: "Zoznamy (upravené)".into(),
        content: before.content.clone(),
    })
    .unwrap();
    let after = core.note(&imported.id).unwrap();
    assert_eq!(after.tags, before.tags);
    assert_eq!(after.title, "Zoznamy (upravené)");

    core.trash_note(&note.id).unwrap();
    assert!(core.search("bryndza").unwrap().is_empty());

    // Close and open again with the key kept by the system.
    core.lock();
    assert!(matches!(core.notes(None), Err(e) if e.code == "locked"));
    let mut core = Core::new(dir.0.clone(), Some(keystore));
    assert!(!core.status().needs_password);
    core.unlock(Secret::KeyStore).unwrap();
    assert_eq!(core.notes(None).unwrap().len(), 3);
}

#[test]
fn password_and_recovery_key_errors_have_codes() {
    let dir = TempDir::new("errors");
    let mut core = Core::new(dir.0.clone(), None);
    assert_eq!(core.create(None).unwrap_err().code, "no_protection");
    let recovery = core.create(Some("Tatry 2026".into())).unwrap();
    core.lock();
    assert!(core.status().needs_password);
    assert_eq!(
        core.unlock(Secret::Password("tatry 2026".into()))
            .unwrap_err()
            .code,
        "wrong_password"
    );
    assert_eq!(
        core.unlock(Secret::Recovery("ALTAIS-123".into()))
            .unwrap_err()
            .code,
        "bad_recovery_key"
    );
    core.unlock(Secret::Recovery(recovery.to_lowercase()))
        .unwrap();
    assert!(core.status().has_password);
    let id = core.new_note(None).unwrap().id;
    let invalid = core.save_note(NoteEdit {
        id,
        title: String::new(),
        content: vec![json!({"t": "heading", "bid": "h001", "level": 9, "text": []})],
    });
    assert_eq!(invalid.unwrap_err().code, "invalid_note");
}

#[test]
fn deserializes_what_the_interface_sends() {
    let secret: Secret = serde_json::from_value(json!({"kind": "password", "value": "x"})).unwrap();
    assert!(matches!(secret, Secret::Password(p) if p == "x"));
    let secret: Secret = serde_json::from_value(json!({"kind": "keyStore"})).unwrap();
    assert!(matches!(secret, Secret::KeyStore));
}

#[test]
fn attachments_can_be_added_read_and_saved() {
    let dir = TempDir::new("attachments");
    let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
    let mut core = Core::new(dir.0.join("archive"), Some(keystore));
    core.create(None).unwrap();

    let file = dir.0.join("Mapa Tatier.PNG");
    fs::write(&file, b"\x89PNG fake image").unwrap();
    let info = core.add_attachment(&file).unwrap();
    assert_eq!(info.name, "Mapa Tatier.PNG");
    assert_eq!(info.mime, "image/png");
    assert_eq!(info.size, 15);
    assert!(info.blob.starts_with("blake3:"));

    // The editor inserts it into a note, which saves with it.
    let note = core.new_note(None).unwrap();
    core.save_note(NoteEdit {
        id: note.id.clone(),
        title: "Výlet".into(),
        content: vec![json!({"t": "attachment", "bid": "a0001", "blob": info.blob, "name": info.name, "mime": info.mime, "size": info.size})],
    })
    .unwrap();
    assert_eq!(core.attachment(&info.blob).unwrap(), b"\x89PNG fake image");

    let copy = dir.0.join("kopia.png");
    core.save_attachment(&info.blob, &copy).unwrap();
    assert_eq!(fs::read(&copy).unwrap(), b"\x89PNG fake image");

    // Opening in the default program: a temporary copy, gone after locking.
    let copies = dir.0.join("open");
    core.set_open_copy_dir(copies.clone());
    let opened = core.open_copy(&info.blob, "..\\Mapa Tatier.PNG").unwrap();
    assert!(opened.starts_with(&copies));
    assert_eq!(opened.file_name().unwrap(), "Mapa Tatier.PNG");
    assert_eq!(fs::read(&opened).unwrap(), b"\x89PNG fake image");
    assert_eq!(
        core.open_copy(&info.blob, "faktura.pdf.exe")
            .unwrap_err()
            .code,
        "unsafe_to_open"
    );
    core.lock();
    assert!(!copies.exists());
    core.unlock(Secret::KeyStore).unwrap();

    let missing = "blake3:0000000000000000000000000000000000000000000000000000000000000000";
    assert_eq!(
        core.attachment(missing).unwrap_err().code,
        "missing_attachment"
    );
    assert_eq!(core.attachment("nonsense").unwrap_err().code, "bad_id");
    assert_eq!(
        altais_core::mime_for("správa.docx"),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
    );
    assert_eq!(
        altais_core::mime_for("bez-pripony"),
        "application/octet-stream"
    );
}

fn demo_package(lang: &str) -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../../resources/demo"))
        .join(format!("altais-demo-{lang}.json"))
}

fn files_under(dir: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                out.extend(files_under(&p));
            } else {
                out.push(p);
            }
        }
    }
    out
}

#[test]
fn demo_loads_works_and_is_removed_without_a_trace() {
    let dir = TempDir::new("demo");
    let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
    let mut core = Core::new(dir.0.join("archive"), Some(keystore.clone()));

    for lang in ["sk", "en"] {
        let status = core.load_demo(&demo_package(lang)).unwrap();
        assert!(status.demo && status.demo_exists && status.unlocked && !status.exists);
        let notebooks = core.notebooks().unwrap();
        assert_eq!(notebooks.len(), 5);
        assert_eq!(core.notes(None).unwrap().len(), 20);
        // Every note is valid and opens; the picture is stored and found by its address.
        let photo = core
            .notes(None)
            .unwrap()
            .into_iter()
            .find(|n| n.title.contains("Štrbské") || n.title.contains("lake"))
            .unwrap();
        let note = core.note(&photo.id).unwrap();
        let blob = note
            .content
            .iter()
            .find(|b| b["t"] == "attachment")
            .unwrap()["blob"]
            .as_str()
            .unwrap()
            .to_owned();
        assert!(core.attachment(&blob).unwrap().starts_with(b"\x89PNG"));
        // Search works in the demo, including Chinese.
        assert_eq!(core.search("笔记").unwrap().len(), 1);
        // Using it: edit a note.
        core.save_note(NoteEdit {
            id: photo.id.clone(),
            title: "Upravené".into(),
            content: note.content.clone(),
        })
        .unwrap();
        // Own data never goes into the demo; key settings are not offered.
        assert_eq!(
            core.import_enex(&[fixture("evernote11.enex")])
                .unwrap_err()
                .code,
            "demo_only"
        );
        assert_eq!(core.set_password("x".into()).unwrap_err().code, "demo_only");

        let status = core.remove_demo().unwrap();
        assert!(!status.demo && !status.demo_exists && !status.unlocked);
        assert!(!core.demo_dir().exists(), "no demo file may remain");
    }

    // The user's own archive is untouched by loading and removing the demo.
    core.create(None).unwrap();
    let own = core.new_note(None).unwrap();
    core.lock();
    core.load_demo(&demo_package("sk")).unwrap();
    core.remove_demo().unwrap();
    core.unlock(Secret::KeyStore).unwrap();
    assert_eq!(core.note(&own.id).unwrap().id, own.id);
    assert!(
        files_under(&dir.0)
            .iter()
            .all(|p| !p.starts_with(dir.0.join("demo")))
    );
}

#[test]
fn a_damaged_demo_package_leaves_nothing_behind() {
    let dir = TempDir::new("demo-damaged");
    fs::create_dir_all(&dir.0).unwrap();
    let bad = dir.0.join("bad.json");
    let text = fs::read_to_string(demo_package("en")).unwrap();
    // Corrupt one attachment: its content no longer matches its address.
    let corrupted = text.replacen("\"iVBOR", "\"AAAA", 1);
    assert_ne!(text, corrupted);
    fs::write(&bad, corrupted).unwrap();
    let mut core = Core::new(dir.0.join("archive"), None);
    assert_eq!(
        core.load_demo(&bad).unwrap_err().code,
        "damaged_demo_package"
    );
    assert!(!core.demo_dir().exists());
}

#[test]
fn trash_restore_and_empty() {
    let dir = TempDir::new("trash");
    let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
    let mut core = Core::new(dir.0.clone(), Some(keystore));
    core.create(None).unwrap();
    let a = core.new_note(None).unwrap();
    let b = core.new_note(None).unwrap();
    core.trash_note(&a.id).unwrap();
    core.trash_note(&b.id).unwrap();
    assert_eq!(core.trash().unwrap().len(), 2);
    assert!(core.notes(None).unwrap().is_empty());
    // A note in the trash can still be read.
    assert_eq!(core.note(&a.id).unwrap().id, a.id);

    core.restore_note(&a.id).unwrap();
    assert_eq!(core.notes(None).unwrap().len(), 1);
    assert_eq!(core.delete_note(&a.id).unwrap_err().code, "not_in_trash");

    core.delete_note(&b.id).unwrap();
    assert_eq!(core.note(&b.id).unwrap_err().code, "not_found");
    core.trash_note(&a.id).unwrap();
    assert_eq!(core.empty_trash().unwrap(), 1);
    assert!(core.trash().unwrap().is_empty());
}
