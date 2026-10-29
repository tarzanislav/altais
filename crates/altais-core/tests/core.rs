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
