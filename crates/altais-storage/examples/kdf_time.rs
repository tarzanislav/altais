//! Prints how long the password key derivation takes on this machine.
fn main() {
    let key = altais_storage::DbKey::generate();
    let recovery = altais_storage::RecoveryKey::generate();
    let mut file = altais_storage::keys::KeyFile::new(&key, &recovery);
    let start = std::time::Instant::now();
    file.set_password(&key, "test", altais_storage::keys::PASSWORD_KDF).unwrap();
    println!("set password: {:?}", start.elapsed());
    let start = std::time::Instant::now();
    file.unlock_with_password("test").unwrap();
    println!("unlock: {:?}", start.elapsed());
}
