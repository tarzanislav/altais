use std::io;

/// Everything that can go wrong with an archive.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// The folder does not contain an ALTAIS archive.
    #[error("this folder does not contain an ALTAIS archive")]
    NotAnArchive,
    /// An archive already exists in the folder.
    #[error("an ALTAIS archive already exists in this folder")]
    AlreadyExists,
    /// The password does not open the archive.
    #[error("wrong password")]
    WrongPassword,
    /// The recovery key does not belong to this archive.
    #[error("this recovery key does not belong to this archive")]
    WrongRecoveryKey,
    /// The text is not a recovery key (wrong length, characters or check digits).
    #[error("this is not a valid recovery key; check for typos")]
    BadRecoveryKey,
    /// Unlocking with a password was requested, but none is set.
    #[error("no password is set for this archive")]
    NoPasswordSet,
    /// An empty password was given.
    #[error("the password must not be empty")]
    EmptyPassword,
    /// The key is not in this computer's credential store (for example, the
    /// archive was copied from another computer).
    #[error("the archive key is not stored on this computer; use the password or the recovery key")]
    KeyNotInKeyStore,
    /// Neither a credential store nor a password was available to protect a new archive.
    #[error("a new archive needs the operating system's credential store or a password")]
    NoProtection,
    /// The credential store failed.
    #[error("credential store: {0}")]
    KeyStore(String),
    /// `keys.json` is damaged or of an unknown format.
    #[error("the key file is damaged: {0}")]
    DamagedKeyFile(String),
    /// The database key does not decrypt the database (damaged file or wrong key).
    #[error(
        "the archive cannot be decrypted: the database file is damaged or does not match the key file"
    )]
    CannotDecrypt,
    /// The note is not a valid ADM document.
    #[error("invalid note: {0}")]
    InvalidNote(String),
    /// An attachment refers to content that is not stored in the archive.
    #[error("attachment {name} refers to content that is not in the archive ({blob})")]
    MissingBlob {
        /// File name of the attachment.
        name: String,
        /// Content address.
        blob: String,
    },
    /// The file holding an attachment's content is missing, damaged or does
    /// not belong to this archive.
    #[error("the content of an attachment is missing or damaged ({0})")]
    DamagedBlob(String),
    /// The database was written by a newer version of ALTAIS.
    #[error(
        "the archive was created by a newer version of ALTAIS (schema {found}, this version knows {known})"
    )]
    NewerSchema {
        /// Schema version in the file.
        found: i64,
        /// Newest schema this version knows.
        known: i64,
    },
    /// SQLite or SQLCipher error.
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    /// File system error.
    #[error("file: {0}")]
    Io(#[from] io::Error),
    /// A stored value could not be read.
    #[error("stored data: {0}")]
    Data(String),
}
