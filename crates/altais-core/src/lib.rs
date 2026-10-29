//! The operations of the ALTAIS application, shared by the desktop app and
//! the `altais` command. Everything the user interface sends or receives is a
//! plain serializable value, so the same calls work through Tauri commands.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use altais_model::{BlobRef, Block, Document, ObjectId};
use altais_storage::{Archive, KeyStore, RecoveryKey, StorageError, Unlock};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

mod demo;
pub mod import;

pub use import::ImportOutcome;

/// An error the user interface can show: a stable `code` for translation and
/// an English `message`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, thiserror::Error)]
#[error("{message}")]
pub struct CoreError {
    /// Stable identifier, e.g. `wrong_password`.
    pub code: &'static str,
    /// English description.
    pub message: String,
}

impl CoreError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    /// An error from outside the core (for example the desktop shell).
    pub fn new_public(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(code, message)
    }

    fn locked() -> Self {
        Self::new("locked", "the archive is not open")
    }
}

impl From<StorageError> for CoreError {
    fn from(e: StorageError) -> Self {
        let code = match &e {
            StorageError::NotAnArchive => "not_an_archive",
            StorageError::AlreadyExists => "already_exists",
            StorageError::WrongPassword => "wrong_password",
            StorageError::WrongRecoveryKey => "wrong_recovery_key",
            StorageError::BadRecoveryKey => "bad_recovery_key",
            StorageError::NoPasswordSet => "no_password_set",
            StorageError::EmptyPassword => "empty_password",
            StorageError::KeyNotInKeyStore => "key_not_on_this_computer",
            StorageError::NoProtection => "no_protection",
            StorageError::CannotDecrypt => "cannot_decrypt",
            StorageError::InvalidNote(_) => "invalid_note",
            StorageError::MissingBlob { .. } => "missing_attachment",
            StorageError::DamagedBlob(_) => "damaged_attachment",
            StorageError::NewerSchema { .. } => "newer_version",
            _ => "storage",
        };
        Self::new(code, e.to_string())
    }
}

type Result<T> = std::result::Result<T, CoreError>;

/// What the start screen needs to know.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    /// Folder of the archive.
    pub dir: String,
    /// Whether an archive exists there.
    pub exists: bool,
    /// Whether opening it needs the password or recovery key.
    pub needs_password: bool,
    /// Whether it is open.
    pub unlocked: bool,
    /// Whether this system can keep the key (so no password is needed).
    pub has_keystore: bool,
    /// Whether a password is set (only known while open).
    pub has_password: bool,
    /// Whether the open archive is the demo archive.
    pub demo: bool,
    /// Whether a demo archive exists.
    pub demo_exists: bool,
}

/// How the user unlocks the archive.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "camelCase")]
pub enum Secret {
    /// The key kept by the system.
    KeyStore,
    /// The password.
    Password(String),
    /// The recovery key as typed.
    Recovery(String),
}

/// A notebook in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotebookInfo {
    /// Identifier.
    pub id: String,
    /// Name.
    pub name: String,
    /// Number of notes.
    pub count: usize,
}

/// A note in a list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteInfo {
    /// Identifier.
    pub id: String,
    /// Title.
    pub title: String,
    /// Notebook identifier.
    pub notebook: Option<String>,
    /// Last change (RFC 3339).
    pub updated: String,
}

/// A note opened in the editor.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteData {
    /// Identifier.
    pub id: String,
    /// Title (empty if none).
    pub title: String,
    /// Notebook identifier.
    pub notebook: Option<String>,
    /// Tags.
    pub tags: Vec<String>,
    /// Creation time (RFC 3339).
    pub created: String,
    /// Last change (RFC 3339).
    pub updated: String,
    /// Content as ADM blocks.
    pub content: Vec<Value>,
}

/// What the editor sends when a note changed.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteEdit {
    /// Identifier.
    pub id: String,
    /// New title.
    pub title: String,
    /// New content as ADM blocks.
    pub content: Vec<Value>,
}

/// A stored attachment, as the editor inserts it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentInfo {
    /// Content address.
    pub blob: String,
    /// File name.
    pub name: String,
    /// MIME type guessed from the file name.
    pub mime: String,
    /// Size in bytes.
    pub size: u64,
}

/// MIME type from a file name's extension (common types; anything else is
/// `application/octet-stream`).
pub fn mime_for(name: &str) -> &'static str {
    let ext = name
        .rsplit_once('.')
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        "heic" => "image/heic",
        "pdf" => "application/pdf",
        "txt" => "text/plain",
        "md" => "text/markdown",
        "csv" => "text/csv",
        "html" | "htm" => "text/html",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "zip" => "application/zip",
        "mp3" => "audio/mpeg",
        "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        _ => "application/octet-stream",
    }
}

fn parse_id(id: &str) -> Result<ObjectId> {
    id.parse()
        .map_err(|_| CoreError::new("bad_id", format!("not a valid id: {id}")))
}

fn note_data(doc: &Document) -> Result<NoteData> {
    let content = doc
        .content
        .iter()
        .map(serde_json::to_value)
        .collect::<std::result::Result<_, _>>()
        .map_err(|e| CoreError::new("storage", e.to_string()))?;
    Ok(NoteData {
        id: doc.id.to_string(),
        title: doc.title.clone().unwrap_or_default(),
        notebook: doc.notebook.map(|n| n.to_string()),
        tags: doc.tags.clone(),
        created: doc.created.to_rfc3339(),
        updated: doc.updated.to_rfc3339(),
        content,
    })
}

fn note_info(n: altais_storage::NoteSummary) -> NoteInfo {
    NoteInfo {
        id: n.id.to_string(),
        title: n.title,
        notebook: n.notebook.map(|id| id.to_string()),
        updated: n.updated.to_rfc3339(),
    }
}

/// The application state: where the archive is and, once unlocked, the archive.
pub struct Core {
    dir: PathBuf,
    keystore: Option<Arc<dyn KeyStore>>,
    archive: Option<Archive>,
    /// Whether `archive` is the demo archive.
    demo: bool,
}

impl Core {
    /// An application using the archive in `dir`.
    pub fn new(dir: PathBuf, keystore: Option<Arc<dyn KeyStore>>) -> Self {
        Self {
            dir,
            keystore,
            archive: None,
            demo: false,
        }
    }

    /// Folder of the demo archive: next to the user's archive, never inside it.
    pub fn demo_dir(&self) -> PathBuf {
        self.dir.with_file_name("demo")
    }

    fn not_demo(&self) -> Result<()> {
        if self.demo {
            return Err(CoreError::new("demo_only", "not available in the demo"));
        }
        Ok(())
    }

    fn archive(&self) -> Result<&Archive> {
        self.archive.as_ref().ok_or_else(CoreError::locked)
    }

    fn archive_mut(&mut self) -> Result<&mut Archive> {
        self.archive.as_mut().ok_or_else(CoreError::locked)
    }

    /// Folder of the archive.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// State for the start screen.
    pub fn status(&self) -> Status {
        let exists = Archive::exists(&self.dir);
        Status {
            dir: self.dir.display().to_string(),
            exists,
            needs_password: exists && Archive::needs_password(&self.dir).unwrap_or(true),
            unlocked: self.archive.is_some(),
            has_keystore: self.keystore.is_some(),
            has_password: !self.demo && self.archive.as_ref().is_some_and(Archive::has_password),
            demo: self.archive.is_some() && self.demo,
            demo_exists: Archive::exists(&self.demo_dir()),
        }
    }

    /// Creates the demo archive from a package and opens it.
    pub fn load_demo(&mut self, package: &Path) -> Result<Status> {
        let dir = self.demo_dir();
        if Archive::exists(&dir) {
            return self.open_demo();
        }
        let result = (|| {
            let (mut archive, _) =
                Archive::create_with_kdf(&dir, None, Some(demo::DEMO_PASSWORD), demo::DEMO_KDF)?;
            demo::load_package(&mut archive, package)?;
            Ok::<_, CoreError>(archive)
        })();
        match result {
            Ok(archive) => {
                self.archive = Some(archive);
                self.demo = true;
                Ok(self.status())
            }
            Err(e) => {
                // A half-filled demo must not stay behind.
                let _ = std::fs::remove_dir_all(&dir);
                Err(e)
            }
        }
    }

    /// Opens the existing demo archive.
    pub fn open_demo(&mut self) -> Result<Status> {
        let archive = Archive::open(
            &self.demo_dir(),
            None,
            Unlock::Password(demo::DEMO_PASSWORD),
        )?;
        self.archive = Some(archive);
        self.demo = true;
        Ok(self.status())
    }

    /// Removes the demo archive completely and checks that nothing is left.
    /// The user's own archive is never touched.
    pub fn remove_demo(&mut self) -> Result<Status> {
        if self.demo {
            self.lock();
        }
        let dir = self.demo_dir();
        if dir.exists() {
            std::fs::remove_dir_all(&dir).map_err(|e| {
                CoreError::new("cannot_remove_demo", format!("{}: {e}", dir.display()))
            })?;
        }
        if dir.exists() {
            return Err(CoreError::new(
                "cannot_remove_demo",
                "the demo folder is still there",
            ));
        }
        Ok(self.status())
    }

    /// Creates the archive; returns the recovery key to show once.
    pub fn create(&mut self, password: Option<String>) -> Result<String> {
        let password = password.filter(|p| !p.is_empty());
        let (archive, recovery) =
            Archive::create(&self.dir, self.keystore.clone(), password.as_deref())?;
        self.archive = Some(archive);
        self.demo = false;
        Ok(recovery.display().to_string())
    }

    /// Opens the archive.
    pub fn unlock(&mut self, secret: Secret) -> Result<Status> {
        let archive = match secret {
            Secret::KeyStore => Archive::open(&self.dir, self.keystore.clone(), Unlock::KeyStore)?,
            Secret::Password(p) => {
                Archive::open(&self.dir, self.keystore.clone(), Unlock::Password(&p))?
            }
            Secret::Recovery(text) => {
                let key = RecoveryKey::parse(&text)?;
                Archive::open(&self.dir, self.keystore.clone(), Unlock::Recovery(&key))?
            }
        };
        self.archive = Some(archive);
        self.demo = false;
        Ok(self.status())
    }

    /// Closes the archive.
    pub fn lock(&mut self) {
        self.archive = None;
        self.demo = false;
    }

    /// Sets or changes the password.
    pub fn set_password(&mut self, password: String) -> Result<Status> {
        self.not_demo()?;
        self.archive_mut()?.set_password(&password)?;
        Ok(self.status())
    }

    /// Removes the password (the system keeps the key again).
    pub fn remove_password(&mut self) -> Result<Status> {
        self.not_demo()?;
        self.archive_mut()?.remove_password()?;
        Ok(self.status())
    }

    /// Replaces the recovery key; returns the new one.
    pub fn new_recovery_key(&mut self) -> Result<String> {
        self.not_demo()?;
        Ok(self
            .archive_mut()?
            .new_recovery_key()?
            .display()
            .to_string())
    }

    /// Notebooks with their note counts, by name.
    pub fn notebooks(&self) -> Result<Vec<NotebookInfo>> {
        let archive = self.archive()?;
        let counts = archive.notebook_note_counts()?;
        Ok(archive
            .notebooks()?
            .into_iter()
            .map(|nb| NotebookInfo {
                count: counts.get(&nb.id).copied().unwrap_or(0),
                id: nb.id.to_string(),
                name: nb.name,
            })
            .collect())
    }

    /// Creates a notebook.
    pub fn create_notebook(&self, name: &str) -> Result<NotebookInfo> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::new("empty_name", "the notebook needs a name"));
        }
        let nb = self.archive()?.create_notebook(name, None)?;
        Ok(NotebookInfo {
            id: nb.id.to_string(),
            name: nb.name,
            count: 0,
        })
    }

    /// Renames a notebook.
    pub fn rename_notebook(&self, id: &str, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(CoreError::new("empty_name", "the notebook needs a name"));
        }
        Ok(self.archive()?.rename_notebook(parse_id(id)?, name)?)
    }

    /// Notes of a notebook (all notes with `None`), newest first.
    pub fn notes(&self, notebook: Option<&str>) -> Result<Vec<NoteInfo>> {
        let notebook = notebook.map(parse_id).transpose()?;
        Ok(self
            .archive()?
            .notes(notebook)?
            .into_iter()
            .map(note_info)
            .collect())
    }

    /// Notes matching what the user typed, best first.
    pub fn search(&self, query: &str) -> Result<Vec<NoteInfo>> {
        Ok(self
            .archive()?
            .search(query, 200)?
            .into_iter()
            .map(|hit| note_info(hit.note))
            .collect())
    }

    /// A note for the editor.
    pub fn note(&self, id: &str) -> Result<NoteData> {
        let doc = self
            .archive()?
            .note(parse_id(id)?)?
            .ok_or_else(|| CoreError::new("not_found", "the note does not exist"))?;
        note_data(&doc)
    }

    /// Creates an empty note in a notebook.
    pub fn new_note(&mut self, notebook: Option<&str>) -> Result<NoteData> {
        let mut doc = Document::new_note("");
        doc.title = None;
        doc.notebook = notebook.map(parse_id).transpose()?;
        self.archive_mut()?.put_note(&doc)?;
        note_data(&doc)
    }

    /// Saves the editor's title and content; everything else about the note
    /// (tags, source, Evernote data) is kept.
    pub fn save_note(&mut self, edit: NoteEdit) -> Result<NoteInfo> {
        let id = parse_id(&edit.id)?;
        let archive = self.archive_mut()?;
        let mut doc = archive
            .note(id)?
            .ok_or_else(|| CoreError::new("not_found", "the note does not exist"))?;
        let content: Vec<Block> = edit
            .content
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| CoreError::new("invalid_note", e.to_string()))?;
        let title = edit.title.trim().to_owned();
        doc.title = (!title.is_empty()).then_some(title);
        doc.content = content;
        doc.updated = Utc::now().max(doc.created);
        archive.put_note(&doc)?;
        Ok(NoteInfo {
            id: doc.id.to_string(),
            title: doc.title.clone().unwrap_or_default(),
            notebook: doc.notebook.map(|n| n.to_string()),
            updated: doc.updated.to_rfc3339(),
        })
    }

    /// Moves a note to the trash.
    pub fn trash_note(&self, id: &str) -> Result<()> {
        Ok(self.archive()?.trash_note(parse_id(id)?)?)
    }

    /// The content of an attachment.
    pub fn attachment(&self, blob: &str) -> Result<Vec<u8>> {
        let blob: BlobRef = blob
            .parse()
            .map_err(|_| CoreError::new("bad_id", format!("not a content address: {blob}")))?;
        self.archive()?.blob(&blob)?.ok_or_else(|| {
            CoreError::new("missing_attachment", "the attachment is not in the archive")
        })
    }

    /// Writes an attachment to a file the user chose.
    pub fn save_attachment(&self, blob: &str, path: &Path) -> Result<()> {
        let data = self.attachment(blob)?;
        std::fs::write(path, data)
            .map_err(|e| CoreError::new("cannot_write_file", format!("{}: {e}", path.display())))
    }

    /// Stores a file as an attachment; the editor then inserts the returned block data.
    pub fn add_attachment(&self, path: &Path) -> Result<AttachmentInfo> {
        let data = std::fs::read(path)
            .map_err(|e| CoreError::new("cannot_read_file", format!("{}: {e}", path.display())))?;
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".into());
        let blob = self.archive()?.put_blob(&data)?;
        Ok(AttachmentInfo {
            blob: blob.to_string(),
            mime: mime_for(&name).to_owned(),
            size: data.len() as u64,
            name,
        })
    }

    /// Imports Evernote exports, one notebook per file (named after the file).
    pub fn import_enex(&mut self, paths: &[PathBuf]) -> Result<Vec<ImportOutcome>> {
        // Own notes go into the own archive, never into the demo.
        self.not_demo()?;
        let archive = self.archive_mut()?;
        let mut outcomes = Vec::new();
        for path in paths {
            let name = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Evernote".into());
            let file = std::fs::File::open(path).map_err(|e| {
                CoreError::new("cannot_read_file", format!("{}: {e}", path.display()))
            })?;
            let mut outcome = import::import_enex(archive, std::io::BufReader::new(file), &name)?;
            outcome.file = path.display().to_string();
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }
}
