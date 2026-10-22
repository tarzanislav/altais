//! The real Windows Credential Manager (runs only on Windows).
#![cfg(windows)]

use altais_storage::{DbKey, KeyStore, OsKeyStore};

#[test]
fn windows_credential_manager_stores_loads_and_removes_the_key() {
    let store = OsKeyStore::system().expect("Windows Credential Manager");
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).unwrap();
    let archive_id: String = format!(
        "test-{}",
        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
    );
    let key = DbKey::generate();

    assert!(store.load(&archive_id).unwrap().is_none());
    store.store(&archive_id, &key).unwrap();
    assert_eq!(
        store.load(&archive_id).unwrap().unwrap().as_bytes(),
        key.as_bytes()
    );
    store.remove(&archive_id).unwrap();
    assert!(store.load(&archive_id).unwrap().is_none());
}
