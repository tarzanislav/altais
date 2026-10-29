//! End-to-end tests of the encrypted archive.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use altais_model::{Document, ObjectId};
use altais_storage::{Archive, KdfParams, KeyStore, OsKeyStore, RecoveryKey, StorageError, Unlock};

/// A fresh folder in the system temp directory, removed afterwards.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let mut bytes = [0u8; 8];
        getrandom_bytes(&mut bytes);
        let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let path = std::env::temp_dir().join(format!("altais-test-{name}-{suffix}"));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn getrandom_bytes(buf: &mut [u8]) {
    getrandom::fill(buf).unwrap();
}

const FAST: KdfParams = KdfParams {
    memory_kib: 1024,
    iterations: 1,
    parallelism: 1,
};

fn store() -> Arc<dyn KeyStore> {
    Arc::new(OsKeyStore::mock())
}

fn create(
    dir: &Path,
    keystore: Option<Arc<dyn KeyStore>>,
    password: Option<&str>,
) -> (Archive, RecoveryKey) {
    Archive::create_with_kdf(dir, keystore, password, FAST).unwrap()
}

fn note(title: &str) -> Document {
    let mut doc = Document::new_note(title);
    doc.tags = vec!["výlet".into(), "Leto".into()];
    doc
}

#[test]
fn opens_without_password_through_the_credential_store() {
    let dir = TempDir::new("keystore");
    let keystore = store();
    let (mut archive, _recovery) = create(dir.path(), Some(keystore.clone()), None);
    let doc = note("Plán výletu");
    archive.put_note(&doc).unwrap();
    drop(archive);

    assert!(!Archive::needs_password(dir.path()).unwrap());
    let archive = Archive::open(dir.path(), Some(keystore), Unlock::KeyStore).unwrap();
    assert_eq!(archive.note(doc.id).unwrap().unwrap(), doc);
    assert_eq!(archive.tags_of(doc.id).unwrap(), vec!["Leto", "výlet"]);
}

#[test]
fn the_database_file_is_encrypted() {
    let dir = TempDir::new("encrypted");
    let (mut archive, _) = create(dir.path(), Some(store()), None);
    archive.put_note(&note("TAJNÝ NADPIS 4711")).unwrap();
    drop(archive);
    let bytes = fs::read(dir.path().join("archive.db")).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3"));
    let needle = "TAJNÝ NADPIS 4711".as_bytes();
    assert!(!bytes.windows(needle.len()).any(|w| w == needle));
    // The write-ahead log, if any, must not contain it either.
    if let Ok(wal) = fs::read(dir.path().join("archive.db-wal")) {
        assert!(!wal.windows(needle.len()).any(|w| w == needle));
    }
}

#[test]
fn copied_to_another_computer_it_opens_with_the_recovery_key() {
    let dir = TempDir::new("recovery");
    let (mut archive, recovery) = create(dir.path(), Some(store()), None);
    let doc = note("Recept");
    archive.put_note(&doc).unwrap();
    drop(archive);

    // Another computer: its credential store does not have the key.
    let other = store();
    assert!(matches!(
        Archive::open(dir.path(), Some(other.clone()), Unlock::KeyStore),
        Err(StorageError::KeyNotInKeyStore)
    ));
    assert!(matches!(
        Archive::open(
            dir.path(),
            Some(other.clone()),
            Unlock::Recovery(&RecoveryKey::generate())
        ),
        Err(StorageError::WrongRecoveryKey)
    ));
    let typed = RecoveryKey::parse(&recovery.display().to_lowercase()).unwrap();
    let archive = Archive::open(dir.path(), Some(other), Unlock::Recovery(&typed)).unwrap();
    assert_eq!(
        archive.note(doc.id).unwrap().unwrap().title.as_deref(),
        Some("Recept")
    );
}

#[test]
fn a_password_replaces_the_system_copy_and_can_be_removed() {
    let dir = TempDir::new("password");
    let keystore = store();
    let (mut archive, _) = create(dir.path(), Some(keystore.clone()), None);
    archive.set_password("Hory volajú!").unwrap();
    assert!(archive.has_password());
    drop(archive);

    assert!(Archive::needs_password(dir.path()).unwrap());
    assert!(matches!(
        Archive::open(dir.path(), Some(keystore.clone()), Unlock::KeyStore),
        Err(StorageError::KeyNotInKeyStore)
    ));
    assert!(matches!(
        Archive::open(
            dir.path(),
            Some(keystore.clone()),
            Unlock::Password("hory volajú!")
        ),
        Err(StorageError::WrongPassword)
    ));
    let mut archive = Archive::open(
        dir.path(),
        Some(keystore.clone()),
        Unlock::Password("Hory volajú!"),
    )
    .unwrap();
    archive.remove_password().unwrap();
    drop(archive);
    assert!(!Archive::needs_password(dir.path()).unwrap());
    Archive::open(dir.path(), Some(keystore), Unlock::KeyStore).unwrap();
}

#[test]
fn without_a_credential_store_a_password_is_required() {
    let dir = TempDir::new("no-store");
    assert!(matches!(
        Archive::create_with_kdf(dir.path(), None, None, FAST),
        Err(StorageError::NoProtection)
    ));
    let (archive, _) = create(dir.path(), None, Some("heslo"));
    drop(archive);
    assert!(Archive::needs_password(dir.path()).unwrap());
    Archive::open(dir.path(), None, Unlock::Password("heslo")).unwrap();
    assert!(matches!(
        Archive::create_with_kdf(dir.path(), None, Some("x"), FAST),
        Err(StorageError::AlreadyExists)
    ));
}

#[test]
fn a_new_recovery_key_replaces_the_old_one() {
    let dir = TempDir::new("new-recovery");
    let (mut archive, old) = create(dir.path(), Some(store()), None);
    let new = archive.new_recovery_key().unwrap();
    drop(archive);
    assert!(matches!(
        Archive::open(dir.path(), None, Unlock::Recovery(&old)),
        Err(StorageError::WrongRecoveryKey)
    ));
    Archive::open(dir.path(), None, Unlock::Recovery(&new)).unwrap();
}

#[test]
fn a_database_from_another_archive_is_rejected() {
    let a = TempDir::new("mix-a");
    let b = TempDir::new("mix-b");
    let (archive_a, _) = create(a.path(), None, Some("a"));
    let (archive_b, _) = create(b.path(), None, Some("a"));
    drop((archive_a, archive_b));
    // Put B's key file next to A's database.
    fs::copy(b.path().join("keys.json"), a.path().join("keys.json")).unwrap();
    assert!(matches!(
        Archive::open(a.path(), None, Unlock::Password("a")),
        Err(StorageError::CannotDecrypt)
    ));
}

#[test]
fn notes_attachments_and_notebooks() {
    let dir = TempDir::new("notes");
    let (mut archive, _) = create(dir.path(), Some(store()), None);
    let travel = archive.create_notebook("Cestovanie", None).unwrap();
    let recipes = archive.create_notebook("Recepty", None).unwrap();
    assert_eq!(
        archive
            .notebooks()
            .unwrap()
            .iter()
            .map(|n| n.name.as_str())
            .collect::<Vec<_>>(),
        ["Cestovanie", "Recepty"]
    );

    let mut doc = note("Tatry");
    doc.notebook = Some(travel.id);
    let json = serde_json::json!({
        "bid": "a001", "t": "attachment", "blob": "blake3:0000000000000000000000000000000000000000000000000000000000000000",
        "name": "mapa.pdf", "mime": "application/pdf", "size": 3
    });
    doc.content.push(serde_json::from_value(json).unwrap());
    assert!(matches!(
        archive.put_note(&doc),
        Err(StorageError::MissingBlob { .. })
    ));

    let blob = archive.put_blob(b"PDF").unwrap();
    assert_eq!(archive.put_blob(b"PDF").unwrap(), blob); // stored once
    let json = serde_json::json!({
        "bid": "a001", "t": "attachment", "blob": blob.to_string(),
        "name": "mapa.pdf", "mime": "application/pdf", "size": 3
    });
    doc.content.pop();
    doc.content.push(serde_json::from_value(json).unwrap());
    archive.put_note(&doc).unwrap();
    assert_eq!(archive.blob(&blob).unwrap().unwrap(), b"PDF");

    let mut other = note("Bryndzové halušky");
    other.notebook = Some(recipes.id);
    archive.put_note(&other).unwrap();
    assert_eq!(archive.notes(Some(travel.id)).unwrap().len(), 1);
    assert_eq!(archive.notes(None).unwrap().len(), 2);
    assert!(archive.note(ObjectId::new()).unwrap().is_none());
}

#[test]
fn an_evernote_export_is_stored_completely() {
    let dir = TempDir::new("enex");
    let (mut archive, _) = create(dir.path(), Some(store()), None);
    let enex = fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../altais-interop/tests/fixtures/evernote11.enex"
    ))
    .unwrap();
    let (notes, summary) = altais_interop::enex::import_all(&enex[..]);
    assert!(summary.notes_read > 0);
    for imported in &notes {
        for blob in &imported.blobs {
            assert_eq!(archive.put_blob(&blob.data).unwrap(), blob.blob);
        }
        archive.put_note(&imported.document).unwrap();
    }
    assert_eq!(archive.notes(None).unwrap().len(), notes.len());
    for imported in &notes {
        assert_eq!(
            archive.note(imported.document.id).unwrap().as_ref(),
            Some(&imported.document)
        );
    }
}

#[test]
fn counts_rename_and_trash() {
    let dir = TempDir::new("trash");
    let (mut archive, _) = create(dir.path(), Some(store()), None);
    let nb = archive.create_notebook("Pracovné", None).unwrap();
    let mut doc = note("Porada");
    doc.notebook = Some(nb.id);
    archive.put_note(&doc).unwrap();
    assert_eq!(
        archive.notebook_note_counts().unwrap().get(&nb.id),
        Some(&1)
    );
    archive.rename_notebook(nb.id, "Práca").unwrap();
    assert_eq!(archive.notebooks().unwrap()[0].name, "Práca");

    archive.trash_note(doc.id).unwrap();
    assert!(archive.notes(None).unwrap().is_empty());
    assert!(archive.search("porada", 10).unwrap().is_empty());
    assert_eq!(archive.notebook_note_counts().unwrap().get(&nb.id), None);
    assert!(archive.note(doc.id).unwrap().is_some(), "kept in the trash");
    archive.restore_note(doc.id).unwrap();
    assert_eq!(archive.search("porada", 10).unwrap().len(), 1);
}

fn with_attachment(
    archive: &Archive,
    title: &str,
    data: &[u8],
) -> (Document, altais_model::BlobRef) {
    let blob = archive.put_blob(data).unwrap();
    let mut doc = note(title);
    let json = serde_json::json!({
        "bid": "a001", "t": "attachment", "blob": blob.to_string(),
        "name": "subor.pdf", "mime": "application/pdf", "size": data.len()
    });
    doc.content.push(serde_json::from_value(json).unwrap());
    (doc, blob)
}

fn files_in(dir: &Path) -> usize {
    let mut n = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            if e.path().is_dir() {
                n += files_in(&e.path());
            } else {
                n += 1;
            }
        }
    }
    n
}

#[test]
fn trash_restore_and_delete_for_good() {
    let dir = TempDir::new("trash");
    let keystore = store();
    let (mut archive, _) = create(dir.path(), Some(keystore.clone()), None);
    let (a, blob_a) = with_attachment(&archive, "Zmluva Bratislava", b"PDF A");
    let (b, blob_shared) = with_attachment(&archive, "Faktúra", b"PDF SHARED");
    let (c, _) = with_attachment(&archive, "Kópia", b"PDF SHARED");
    for doc in [&a, &b, &c] {
        archive.put_note(doc).unwrap();
    }
    drop(archive); // nothing "fresh" any more after reopening
    let mut archive = Archive::open(dir.path(), Some(keystore), Unlock::KeyStore).unwrap();

    archive.trash_note(a.id).unwrap();
    archive.trash_note(b.id).unwrap();
    assert_eq!(archive.trashed_notes().unwrap().len(), 2);
    assert_eq!(archive.notes(None).unwrap().len(), 1);
    assert!(archive.search("bratislava", 10).unwrap().is_empty());

    archive.restore_note(b.id).unwrap();
    assert_eq!(archive.trashed_notes().unwrap().len(), 1);
    assert_eq!(archive.notes(None).unwrap().len(), 2);

    // A note outside the trash cannot be deleted for good.
    assert!(!archive.delete_note(b.id).unwrap());
    assert!(archive.note(b.id).unwrap().is_some());

    // Trash still keeps its attachment.
    assert_eq!(archive.collect_garbage().unwrap(), 0);
    assert!(archive.blob(&blob_a).unwrap().is_some());

    assert!(archive.delete_note(a.id).unwrap());
    assert!(archive.note(a.id).unwrap().is_none());
    assert!(archive.blob(&blob_a).unwrap().is_none());
    assert_eq!(files_in(&dir.path().join("blobs")), 1); // the shared one only

    // Content used by another note stays.
    archive.trash_note(b.id).unwrap();
    assert_eq!(archive.empty_trash().unwrap(), 1);
    assert!(archive.blob(&blob_shared).unwrap().is_some());
    archive.trash_note(c.id).unwrap();
    assert_eq!(archive.empty_trash().unwrap(), 1);
    assert!(archive.blob(&blob_shared).unwrap().is_none());
    assert_eq!(files_in(&dir.path().join("blobs")), 0);
    assert_eq!(archive.note_count().unwrap(), 0);
    assert!(archive.search("faktura", 10).unwrap().is_empty());
}

#[test]
fn content_stored_in_this_session_is_not_collected() {
    let dir = TempDir::new("fresh");
    let (archive, _) = create(dir.path(), Some(store()), None);
    // Added to a note the editor has not saved yet.
    let blob = archive.put_blob(b"new picture").unwrap();
    assert_eq!(archive.collect_garbage().unwrap(), 0);
    assert!(archive.blob(&blob).unwrap().is_some());
}
