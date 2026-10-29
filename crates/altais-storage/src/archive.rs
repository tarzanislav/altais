//! Opening, creating and using an archive.

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use altais_model::{BlobRef, Block, Document, KnownBlock, ObjectId};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};

use crate::blobstore::BlobStore;
use crate::keys::{DbKey, KeyFile, KeyStore, PASSWORD_KDF, RecoveryKey};
use crate::{KdfParams, StorageError, fold, schema, text};

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

/// A search result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchHit {
    /// The note.
    pub note: NoteSummary,
}

/// An open archive.
pub struct Archive {
    conn: Connection,
    dir: PathBuf,
    keys: KeyFile,
    key: DbKey,
    keystore: Option<Arc<dyn KeyStore>>,
    kdf: KdfParams,
    blobs: BlobStore,
    /// Content stored in this session; never collected as unused, because
    /// the note it was added to may not be saved yet.
    fresh_blobs: RefCell<HashSet<String>>,
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
        "PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA secure_delete = ON;",
    )?;
    let previous = schema::migrate(&mut conn)?;
    if previous > 0 && previous < schema::SEARCH_INDEX_VERSION {
        reindex(&mut conn)?;
    }
    Ok(conn)
}

/// Writes a note into the search index (replacing what was there).
fn index_note(conn: &rusqlite::Transaction<'_>, doc: &Document) -> Result<(), StorageError> {
    let rowid: i64 = conn.query_row(
        "SELECT rowid FROM note WHERE id = ?1",
        [doc.id.to_string()],
        |r| r.get(0),
    )?;
    conn.execute("DELETE FROM note_fts WHERE rowid = ?1", [rowid])?;
    conn.execute(
        "INSERT INTO note_fts (rowid, title, body) VALUES (?1, ?2, ?3)",
        params![
            rowid,
            fold::fold(doc.title.as_deref().unwrap_or_default()),
            fold::fold(&text::body_text(doc))
        ],
    )?;
    Ok(())
}

/// Rebuilds the whole search index (after a schema upgrade).
fn reindex(conn: &mut Connection) -> Result<(), StorageError> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM note_fts", [])?;
    let docs: Vec<String> = {
        let mut stmt = tx.prepare("SELECT adm FROM note")?;
        stmt.query_map([], |r| r.get(0))?
            .collect::<Result<_, _>>()?
    };
    for json in docs {
        let doc = Document::from_json(&json).map_err(|e| StorageError::Data(e.to_string()))?;
        index_note(&tx, &doc)?;
    }
    tx.commit()?;
    Ok(())
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
        let blobs = BlobStore::new(dir, &key);
        Ok((
            Self {
                conn,
                dir: dir.to_owned(),
                keys,
                key,
                keystore,
                kdf,
                blobs,
                fresh_blobs: RefCell::default(),
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
        let blobs = BlobStore::new(dir, &key);
        let archive = Self {
            conn,
            dir: dir.to_owned(),
            keys,
            key,
            keystore,
            kdf: PASSWORD_KDF,
            blobs,
            fresh_blobs: RefCell::default(),
        };
        archive.move_blobs_to_files()?;
        Ok(archive)
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

    /// Number of notes in each notebook (notebooks without notes are missing).
    pub fn notebook_note_counts(
        &self,
    ) -> Result<std::collections::HashMap<ObjectId, usize>, StorageError> {
        let mut stmt = self.conn.prepare(
            "SELECT notebook, count(*) FROM note WHERE deleted IS NULL AND notebook IS NOT NULL GROUP BY notebook",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
        rows.map(|row| {
            let (id, n) = row?;
            Ok((parse_id(id)?, n as usize))
        })
        .collect()
    }

    /// Renames a notebook.
    pub fn rename_notebook(&self, id: ObjectId, name: &str) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE notebook SET name = ?2, updated = ?3 WHERE id = ?1",
            params![id.to_string(), name, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    // ---------------- Notes ----------------

    /// Moves a note to the trash: it disappears from lists and search but is
    /// kept, so it can be restored.
    pub fn trash_note(&self, id: ObjectId) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE note SET deleted = ?2 WHERE id = ?1 AND deleted IS NULL",
            params![id.to_string(), Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    /// Takes a note back out of the trash.
    pub fn restore_note(&self, id: ObjectId) -> Result<(), StorageError> {
        self.conn.execute(
            "UPDATE note SET deleted = NULL WHERE id = ?1",
            [id.to_string()],
        )?;
        Ok(())
    }

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
        index_note(&tx, doc)?;
        tx.commit()?;
        Ok(())
    }

    /// Finds notes. Every word must occur (as the start of a word, ignoring
    /// case and diacritics); text in quotes and Chinese words must occur as
    /// written. Best matches first; a match in the title counts more.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchHit>, StorageError> {
        let Some(fts) = fold::fts_query(query) else {
            return Ok(Vec::new());
        };
        let mut stmt = self.conn.prepare(
            "SELECT note.id, note.title, note.notebook, note.updated
             FROM note_fts JOIN note ON note.rowid = note_fts.rowid
             WHERE note_fts MATCH ?1 AND note.deleted IS NULL
             ORDER BY bm25(note_fts, 5.0, 1.0)
             LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts, limit as i64], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?;
        rows.map(|row| {
            let (id, title, notebook, updated) = row?;
            Ok(SearchHit {
                note: NoteSummary {
                    id: parse_id(id)?,
                    title,
                    notebook: notebook.map(parse_id).transpose()?,
                    updated: parse_time(updated)?,
                },
            })
        })
        .collect()
    }

    /// Number of notes (not counting deleted ones).
    pub fn note_count(&self) -> Result<usize, StorageError> {
        let n: i64 =
            self.conn
                .query_row("SELECT count(*) FROM note WHERE deleted IS NULL", [], |r| {
                    r.get(0)
                })?;
        Ok(n as usize)
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
        self.summaries(
            "SELECT id, title, notebook, updated FROM note
             WHERE deleted IS NULL AND (?1 IS NULL OR notebook = ?1)
             ORDER BY updated DESC",
            notebook.map(|n| n.to_string()),
        )
    }

    /// Notes in the trash, most recently trashed first.
    pub fn trashed_notes(&self) -> Result<Vec<NoteSummary>, StorageError> {
        self.summaries(
            "SELECT id, title, notebook, updated FROM note
             WHERE deleted IS NOT NULL AND ?1 IS NULL
             ORDER BY deleted DESC",
            None,
        )
    }

    fn summaries(&self, sql: &str, arg: Option<String>) -> Result<Vec<NoteSummary>, StorageError> {
        let mut stmt = self.conn.prepare(sql)?;
        let rows = stmt.query_map([arg], |r| {
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

    /// Deletes a note in the trash for good, with its search entry, and then
    /// the content no note uses any more. A note not in the trash is left
    /// alone (returns false).
    pub fn delete_note(&mut self, id: ObjectId) -> Result<bool, StorageError> {
        let deleted = self.delete_where("id = ?1 AND deleted IS NOT NULL", Some(id.to_string()))?;
        if deleted > 0 {
            self.collect_garbage()?;
        }
        Ok(deleted > 0)
    }

    /// Deletes every note in the trash for good; returns how many.
    pub fn empty_trash(&mut self) -> Result<usize, StorageError> {
        let deleted = self.delete_where("deleted IS NOT NULL AND ?1 IS NULL", None)?;
        self.collect_garbage()?;
        Ok(deleted)
    }

    fn delete_where(
        &mut self,
        condition: &str,
        arg: Option<String>,
    ) -> Result<usize, StorageError> {
        let tx = self.conn.transaction()?;
        tx.execute(
            &format!(
                "DELETE FROM note_fts WHERE rowid IN (SELECT rowid FROM note WHERE {condition})"
            ),
            [&arg],
        )?;
        // Tags and attachment rows go with the note (ON DELETE CASCADE).
        let deleted = tx.execute(&format!("DELETE FROM note WHERE {condition}"), [&arg])?;
        tx.execute(
            "DELETE FROM tag WHERE id NOT IN (SELECT tag FROM note_tag)",
            [],
        )?;
        tx.commit()?;
        Ok(deleted)
    }

    /// Deletes stored content that no note (not even one in the trash) uses,
    /// except content stored in this session. Returns how many.
    pub fn collect_garbage(&self) -> Result<usize, StorageError> {
        let unused: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT hash FROM blob WHERE hash NOT IN (SELECT blob FROM attachment)")?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        let fresh = self.fresh_blobs.borrow();
        let mut removed = 0;
        for hex in unused.iter().filter(|h| !fresh.contains(*h)) {
            // The record first: a crash in between leaves an unused file,
            // never a record without its file.
            self.conn
                .execute("DELETE FROM blob WHERE hash = ?1", [hex])?;
            self.blobs.remove(&hash_bytes(hex)?)?;
            removed += 1;
        }
        // Move the deleted pages out of the write-ahead log into the
        // database, where secure_delete has overwritten them.
        self.conn
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))?;
        Ok(removed)
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
    /// twice keeps one copy. The content goes into an encrypted file
    /// (ADR-0009); the database only records that it exists.
    pub fn put_blob(&self, data: &[u8]) -> Result<BlobRef, StorageError> {
        let hash = blake3::hash(data);
        self.blobs.put(hash.as_bytes(), data)?;
        let hex = hash.to_hex();
        self.fresh_blobs.borrow_mut().insert(hex.to_string());
        self.conn.execute(
            "INSERT INTO blob (hash, size, data) VALUES (?1, ?2, X'') ON CONFLICT(hash) DO NOTHING",
            params![hex.as_str(), data.len() as i64],
        )?;
        BlobRef::try_from(format!("blake3:{hex}")).map_err(|e| StorageError::Data(e.to_string()))
    }

    /// Stored content by its address; an error if its file is missing or
    /// damaged.
    pub fn blob(&self, blob: &BlobRef) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(size) = self
            .conn
            .query_row("SELECT size FROM blob WHERE hash = ?1", [blob.hex()], |r| {
                r.get::<_, i64>(0)
            })
            .optional()?
        else {
            return Ok(None);
        };
        let hash = hash_bytes(blob.hex())?;
        match self.blobs.get(&hash)? {
            Some(data) if data.len() as i64 == size => Ok(Some(data)),
            _ => Err(StorageError::DamagedBlob(blob.to_string())),
        }
    }

    /// Moves content still kept in the database (archives from before
    /// ADR-0009) into encrypted files, one at a time, so an interrupted move
    /// simply continues on the next opening.
    fn move_blobs_to_files(&self) -> Result<(), StorageError> {
        let pending: Vec<String> = {
            let mut stmt = self
                .conn
                .prepare("SELECT hash FROM blob WHERE length(data) > 0")?;
            stmt.query_map([], |r| r.get(0))?
                .collect::<Result<_, _>>()?
        };
        if pending.is_empty() {
            return Ok(());
        }
        for hex in &pending {
            let data: Vec<u8> =
                self.conn
                    .query_row("SELECT data FROM blob WHERE hash = ?1", [hex], |r| r.get(0))?;
            let hash = hash_bytes(hex)?;
            if blake3::hash(&data).as_bytes() != &hash {
                return Err(StorageError::DamagedBlob(format!("blake3:{hex}")));
            }
            self.blobs.put(&hash, &data)?;
            // Only after the file is safely on disk.
            self.conn
                .execute("UPDATE blob SET data = X'' WHERE hash = ?1", [hex])?;
        }
        // Give the freed space back and leave no old pages behind.
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }
}

fn hash_bytes(hex: &str) -> Result<[u8; 32], StorageError> {
    let bad = || StorageError::Data(format!("bad content address {hex}"));
    if hex.len() != 64 {
        return Err(bad());
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).map_err(|_| bad())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OsKeyStore;

    #[test]
    fn an_archive_from_before_the_search_index_gets_indexed_on_open() {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let dir = std::env::temp_dir().join(format!("altais-migrate-{suffix}"));
        let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
        let fast = KdfParams {
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
        };
        let (mut archive, _) =
            Archive::create_with_kdf(&dir, Some(keystore.clone()), None, fast).unwrap();
        archive
            .put_note(&Document::new_note("Starý archív"))
            .unwrap();
        // Turn it into a schema-1 archive.
        archive
            .conn
            .execute_batch("DROP TABLE note_fts; PRAGMA user_version = 1;")
            .unwrap();
        drop(archive);

        let archive = Archive::open(&dir, Some(keystore), Unlock::KeyStore).unwrap();
        let hits = archive.search("stary", 10).unwrap();
        assert_eq!(hits.len(), 1);
        drop(archive);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn attachments_kept_in_the_database_move_into_files_on_open() {
        let mut bytes = [0u8; 8];
        getrandom::fill(&mut bytes).unwrap();
        let suffix: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        let dir = std::env::temp_dir().join(format!("altais-move-{suffix}"));
        let keystore: Arc<dyn KeyStore> = Arc::new(OsKeyStore::mock());
        let fast = KdfParams {
            memory_kib: 1024,
            iterations: 1,
            parallelism: 1,
        };
        let (archive, _) =
            Archive::create_with_kdf(&dir, Some(keystore.clone()), None, fast).unwrap();
        // An archive from before ADR-0009: content in the database.
        let data = b"stara priloha".to_vec();
        let hex = blake3::hash(&data).to_hex();
        archive
            .conn
            .execute(
                "INSERT INTO blob (hash, size, data) VALUES (?1, ?2, ?3)",
                params![hex.as_str(), data.len() as i64, data],
            )
            .unwrap();
        archive
            .conn
            .execute_batch("PRAGMA user_version = 2;")
            .unwrap();
        drop(archive);

        let archive = Archive::open(&dir, Some(keystore), Unlock::KeyStore).unwrap();
        let left: i64 = archive
            .conn
            .query_row("SELECT sum(length(data)) FROM blob", [], |r| r.get(0))
            .unwrap();
        assert_eq!(left, 0);
        assert!(archive.blobs.contains(blake3::hash(&data).as_bytes()));
        let blob: BlobRef = format!("blake3:{hex}").parse().unwrap();
        assert_eq!(archive.blob(&blob).unwrap().unwrap(), data);

        // A deleted file is reported, not silently empty.
        fs::remove_file(archive.blobs.path_of(blake3::hash(&data).as_bytes())).unwrap();
        assert!(matches!(
            archive.blob(&blob),
            Err(StorageError::DamagedBlob(_))
        ));
        drop(archive);
        let _ = fs::remove_dir_all(dir);
    }
}
