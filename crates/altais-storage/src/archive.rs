//! Opening, creating and using an archive.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use altais_model::{BlobRef, Block, Document, KnownBlock, ObjectId};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::keys::{DbKey, KeyFile, KeyStore, PASSWORD_KDF, RecoveryKey};
use crate::{KdfParams, StorageError, schema};

const DB_FILE: &str = "archive.db";
const KEY_FILE: &str = "keys.json";

/// How to unlock an archive.
#[derive(Clone, Copy, Debug)]
pub enum Unlock<'a> {
    /// With the key kept in the operating system's credential store.
    KeyStore,
    /// With the password.
    Password(&'a str),
    /// With the recovery key.
    Recovery(&'a RecoveryKey),
}

/// A notebook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notebook {
    /// Identifier.
    pub id: ObjectId,
    /// Name shown to the user.
    pub name: String,
    /// Parent notebook (stacks of notebooks).
    pub parent: Option<ObjectId>,
    /// Creation time.
    pub created: DateTime<Utc>,
    /// Last change.
    pub updated: DateTime<Utc>,
}

/// What a note list shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NoteSummary {
    /// Identifier.
    pub id: ObjectId,
    /// Title (empty if the note has none).
    pub title: String,
    /// Notebook.
    pub notebook: Option<ObjectId>,
    /// Last change.
    pub updated: DateTime<Utc>,
}

/// An open archive.
pub struct Archive {
    conn: Connection,
    dir: PathBuf,
    keys: KeyFile,
    key: DbKey,
    keystore: Option<Arc<dyn KeyStore>>,
    kdf: KdfParams,
}

impl std::fmt::Debug for Archive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Archive")
            .field("dir", &self.dir)
            .finish_non_exhaustive()
    }
}

fn write_atomically(path: &Path, contents: &str) -> Result<(), StorageError> {
    use std::io::Write;
    let tmp = path.with_extension("json.tmp");
    // Write and flush through one writable handle (Windows refuses to flush
    // a file opened only for reading), so the new key file is on disk before
    // it replaces the old one.
    let mut file = fs::File::create(&tmp)?;
    file.write_all(contents.as_bytes())?;
    file.sync_all()?;
    drop(file);
    fs::rename(&tmp, path)?;
    Ok(())
}

fn open_database(path: &Path, key: &DbKey) -> Result<Connection, StorageError> {
    let mut conn = Connection::open(path)?;
    // Must be the first statement: SQLCipher raw key (no password derivation,
    // the key is already random).
    conn.execute_batch(&format!(
        "PRAGMA key = \"{}\";",
        key.sqlcipher_pragma_value().as_str()
    ))?;
    // Reading the schema fails if the key does not decrypt the file.
    if conn
        .query_row("SELECT count(*) FROM sqlite_master", [], |r| {
            r.get::<_, i64>(0)
        })
        .is_err()
    {
        return Err(StorageError::CannotDecrypt);
    }
    conn.execute_batch(
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;",
    )?;
    schema::migrate(&mut conn)?;
    Ok(conn)
}

fn parse_time(s: String) -> Result<DateTime<Utc>, StorageError> {
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(|e| StorageError::Data(format!("time {s:?}: {e}")))
}

fn parse_id(s: String) -> Result<ObjectId, StorageError> {
    s.parse()
        .map_err(|e| StorageError::Data(format!("id {s:?}: {e}")))
}

fn attachments(blocks: &[Block], out: &mut Vec<(String, BlobRef, String, String)>) {
    for block in blocks {
        if let Block::Known(KnownBlock::Attachment {
            bid,
            blob,
            name,
            mime,
            ..
        }) = block
        {
            out.push((
                bid.as_str().to_owned(),
                blob.clone(),
                name.clone(),
                mime.clone(),
            ));
        }
        let children: Vec<Block> = block.children().into_iter().cloned().collect();
        attachments(&children, out);
    }
}

impl Archive {
    /// Whether `dir` contains an archive.
    pub fn exists(dir: &Path) -> bool {
        dir.join(KEY_FILE).is_file() && dir.join(DB_FILE).is_file()
    }

    /// Whether opening the archive in `dir` needs the password (or the
    /// recovery key), because no copy of the key is kept by the system.
    pub fn needs_password(dir: &Path) -> Result<bool, StorageError> {
        Ok(!Self::read_keys(dir)?.os_keystore)
    }

    fn read_keys(dir: &Path) -> Result<KeyFile, StorageError> {
        let path = dir.join(KEY_FILE);
        if !path.is_file() {
            return Err(StorageError::NotAnArchive);
        }
        KeyFile::from_json(&fs::read_to_string(path)?)
    }

    /// Creates an archive in `dir` and returns it with its recovery key,
    /// which the user must keep. The key is stored in `keystore` if given
    /// (the archive then opens without a password), and protected by
    /// `password` if given; at least one of the two is required.
    pub fn create(
        dir: &Path,
        keystore: Option<Arc<dyn KeyStore>>,
        password: Option<&str>,
    ) -> Result<(Self, RecoveryKey), StorageError> {
        Self::create_with_kdf(dir, keystore, password, PASSWORD_KDF)
    }

    /// [`Archive::create`] with explicit password cost (tests use a low one).
    pub fn create_with_kdf(
        dir: &Path,
        keystore: Option<Arc<dyn KeyStore>>,
        password: Option<&str>,
        kdf: KdfParams,
    ) -> Result<(Self, RecoveryKey), StorageError> {
        if dir.join(KEY_FILE).exists() || dir.join(DB_FILE).exists() {
            return Err(StorageError::AlreadyExists);
        }
        if keystore.is_none() && password.is_none() {
            return Err(StorageError::NoProtection);
        }
        fs::create_dir_all(dir)?;
        let key = DbKey::generate();
        let recovery = RecoveryKey::generate();
        let mut keys = KeyFile::new(&key, &recovery);
        if let Some(password) = password {
            keys.set_password(&key, password, kdf)?;
        } else if let Some(store) = &keystore {
            store.store(&keys.archive_id, &key)?;
            keys.os_keystore = true;
        }
        // The key file first: a database without it could never be opened.
        write_atomically(&dir.join(KEY_FILE), &keys.to_json())?;
        let conn = open_database(&dir.join(DB_FILE), &key)?;
        Ok((
            Self {
                conn,
                dir: dir.to_owned(),
                keys,
                key,
                keystore,
                kdf,
            },
            recovery,
        ))
    }

    /// Opens the archive in `dir`.
    pub fn open(
        dir: &Path,
        keystore: Option<Arc<dyn KeyStore>>,
        unlock: Unlock<'_>,
    ) -> Result<Self, StorageError> {
        let keys = Self::read_keys(dir)?;
        if !dir.join(DB_FILE).is_file() {
            return Err(StorageError::NotAnArchive);
        }
        let key = match unlock {
            Unlock::KeyStore => {
                let store = keystore.as_ref().ok_or(StorageError::KeyNotInKeyStore)?;
                if !keys.os_keystore {
                    return Err(StorageError::KeyNotInKeyStore);
                }
                store
                    .load(&keys.archive_id)?
                    .ok_or(StorageError::KeyNotInKeyStore)?
            }
            Unlock::Password(password) => keys.unlock_with_password(password)?,
            Unlock::Recovery(recovery) => keys.unlock_with_recovery(recovery)?,
        };
        let conn = open_database(&dir.join(DB_FILE), &key)?;
        Ok(Self {
            conn,
            dir: dir.to_owned(),
            keys,
            key,
            keystore,
            kdf: PASSWORD_KDF,
        })
    }

    /// Folder of the archive.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Whether a password is required to open the archive.
    pub fn has_password(&self) -> bool {
        self.keys.has_password()
    }

    fn save_keys(&self) -> Result<(), StorageError> {
        write_atomically(&self.dir.join(KEY_FILE), &self.keys.to_json())
    }

    /// Sets or changes the password. From then on the password is needed to
    /// open the archive: the copy in the system's credential store is removed.
    pub fn set_password(&mut self, password: &str) -> Result<(), StorageError> {
        self.keys.set_password(&self.key, password, self.kdf)?;
        let had_os_copy = self.keys.os_keystore;
        self.keys.os_keystore = false;
        self.save_keys()?;
        if had_os_copy && let Some(store) = &self.keystore {
            store.remove(&self.keys.archive_id)?;
        }
        Ok(())
    }

    /// Removes the password; the key is kept by the system's credential
    /// store again, so the archive opens without a password.
    pub fn remove_password(&mut self) -> Result<(), StorageError> {
        let store = self.keystore.as_ref().ok_or(StorageError::NoProtection)?;
        store.store(&self.keys.archive_id, &self.key)?;
        self.keys.os_keystore = true;
        self.keys.remove_password();
        self.save_keys()
    }

    /// Creates a new recovery key; the old one stops working.
    pub fn new_recovery_key(&mut self) -> Result<RecoveryKey, StorageError> {
        let recovery = RecoveryKey::generate();
        self.keys.set_recovery(&self.key, &recovery);
        self.save_keys()?;
        Ok(recovery)
    }

    // ---------------- Notebooks ----------------

    /// Creates a notebook.
    pub fn create_notebook(
        &self,
        name: &str,
        parent: Option<ObjectId>,
    ) -> Result<Notebook, StorageError> {
        let now = Utc::now();
        let notebook = Notebook {
            id: ObjectId::new(),
            name: name.to_owned(),
            parent,
            created: now,
            updated: now,
        };
        self.conn.execute(
            "INSERT INTO notebook (id, name, parent, created, updated) VALUES (?1, ?2, ?3, ?4, ?4)",
            params![
                notebook.id.to_string(),
                notebook.name,
                parent.map(|p| p.to_string()),
                now.to_rfc3339()
            ],
        )?;
        Ok(notebook)
    }

    /// All notebooks, by name.
    pub fn notebooks(&self) -> Result<Vec<Notebook>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, parent, created, updated FROM notebook ORDER BY name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?;
        rows.map(|row| {
            let (id, name, parent, created, updated) = row?;
            Ok(Notebook {
                id: parse_id(id)?,
                name,
                parent: parent.map(parse_id).transpose()?,
                created: parse_time(created)?,
                updated: parse_time(updated)?,
            })
        })
        .collect()
    }

    // ---------------- Notes ----------------

    /// Stores a note (new or changed). Its tags and attachments are indexed;
    /// the content of every attachment must already be stored with
    /// [`Archive::put_blob`].
    pub fn put_note(&mut self, doc: &Document) -> Result<(), StorageError> {
        altais_model::validate(doc).map_err(|errors| {
            StorageError::InvalidNote(
                errors
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;
        let json = serde_json::to_string(doc).map_err(|e| StorageError::Data(e.to_string()))?;
        let mut found = Vec::new();
        attachments(&doc.content, &mut found);

        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO note (id, notebook, title, created, updated, adm) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET notebook = ?2, title = ?3, updated = ?5, adm = ?6",
            params![
                doc.id.to_string(),
                doc.notebook.map(|n| n.to_string()),
                doc.title.clone().unwrap_or_default(),
                doc.created.to_rfc3339(),
                doc.updated.to_rfc3339(),
                json
            ],
        )?;
        let id = doc.id.to_string();
        tx.execute("DELETE FROM note_tag WHERE note = ?1", [&id])?;
        for tag in &doc.tags {
            tx.execute(
                "INSERT INTO tag (name) VALUES (?1) ON CONFLICT(name) DO NOTHING",
                [tag],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO note_tag (note, tag) SELECT ?1, id FROM tag WHERE name = ?2",
                params![id, tag],
            )?;
        }
        tx.execute("DELETE FROM attachment WHERE note = ?1", [&id])?;
        for (bid, blob, name, mime) in found {
            let present: bool = tx.query_row(
                "SELECT EXISTS(SELECT 1 FROM blob WHERE hash = ?1)",
                [blob.hex()],
                |r| r.get(0),
            )?;
            if !present {
                return Err(StorageError::MissingBlob {
                    name,
                    blob: blob.to_string(),
                });
            }
            tx.execute(
                "INSERT INTO attachment (note, bid, blob, name, mime) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![id, bid, blob.hex(), name, mime],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// A note by its identifier.
    pub fn note(&self, id: ObjectId) -> Result<Option<Document>, StorageError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT adm FROM note WHERE id = ?1",
                [id.to_string()],
                |r| r.get(0),
            )
            .optional()?;
        json.map(|j| Document::from_json(&j).map_err(|e| StorageError::Data(e.to_string())))
            .transpose()
    }

    /// Notes of a notebook (or all notes with `None`), newest first.
    pub fn notes(&self, notebook: Option<ObjectId>) -> Result<Vec<NoteSummary>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, notebook, updated FROM note
             WHERE deleted IS NULL AND (?1 IS NULL OR notebook = ?1)
             ORDER BY updated DESC",
        )?;
        let rows = stmt.query_map([notebook.map(|n| n.to_string())], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (id, title, notebook, updated) = row?;
            Ok(NoteSummary {
                id: parse_id(id)?,
                title,
                notebook: notebook.map(parse_id).transpose()?,
                updated: parse_time(updated)?,
            })
        })
        .collect()
    }

    /// Names of the tags of a note, sorted.
    pub fn tags_of(&self, id: ObjectId) -> Result<Vec<String>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT tag.name FROM note_tag JOIN tag ON tag.id = note_tag.tag
             WHERE note_tag.note = ?1 ORDER BY tag.name COLLATE NOCASE",
        )?;
        let rows = stmt.query_map([id.to_string()], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    // ---------------- Attachment content ----------------

    /// Stores content and returns its address; storing the same content
    /// twice keeps one copy.
    pub fn put_blob(&self, data: &[u8]) -> Result<BlobRef, StorageError> {
        let hex = blake3::hash(data).to_hex();
        self.conn.execute(
            "INSERT INTO blob (hash, size, data) VALUES (?1, ?2, ?3) ON CONFLICT(hash) DO NOTHING",
            params![hex.as_str(), data.len() as i64, data],
        )?;
        BlobRef::try_from(format!("blake3:{hex}")).map_err(|e| StorageError::Data(e.to_string()))
    }

    /// Stored content by its address.
    pub fn blob(&self, blob: &BlobRef) -> Result<Option<Vec<u8>>, StorageError> {
        Ok(self
            .conn
            .query_row("SELECT data FROM blob WHERE hash = ?1", [blob.hex()], |r| {
                r.get(0)
            })
            .optional()?)
    }
}
