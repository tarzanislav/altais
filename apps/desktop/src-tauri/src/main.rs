//! ALTAIS desktop application: a Tauri window around the web interface
//! (`apps/desktop/src`), with every operation handled by `altais-core`.

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use altais_core::{
    AttachmentInfo, Core, CoreError, ImportOutcome, NoteData, NoteEdit, NoteInfo, NotebookInfo,
    Secret, Status,
};
use altais_storage::{KeyStore, OsKeyStore};
use tauri::{Manager, RunEvent, State};
use tauri_plugin_opener::OpenerExt;

struct AppState(Mutex<Core>);

impl AppState {
    fn core(&self) -> MutexGuard<'_, Core> {
        // A panic in one command must not lock the user out of the archive.
        self.0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

type Reply<T> = Result<T, CoreError>;

// Commands run on a worker thread (`async`), so a long import does not
// freeze the window.

#[tauri::command(async)]
fn status(state: State<'_, AppState>) -> Reply<Status> {
    Ok(state.core().status())
}

#[tauri::command(async)]
fn create_archive(state: State<'_, AppState>, password: Option<String>) -> Reply<String> {
    state.core().create(password)
}

#[tauri::command(async)]
fn unlock(state: State<'_, AppState>, secret: Secret) -> Reply<Status> {
    state.core().unlock(secret)
}

#[tauri::command(async)]
fn lock(state: State<'_, AppState>) -> Reply<()> {
    state.core().lock();
    Ok(())
}

#[tauri::command(async)]
fn notebooks(state: State<'_, AppState>) -> Reply<Vec<NotebookInfo>> {
    state.core().notebooks()
}

#[tauri::command(async)]
fn create_notebook(state: State<'_, AppState>, name: String) -> Reply<NotebookInfo> {
    state.core().create_notebook(&name)
}

#[tauri::command(async)]
fn notes(state: State<'_, AppState>, notebook: Option<String>) -> Reply<Vec<NoteInfo>> {
    state.core().notes(notebook.as_deref())
}

#[tauri::command(async)]
fn search(state: State<'_, AppState>, query: String) -> Reply<Vec<NoteInfo>> {
    state.core().search(&query)
}

#[tauri::command(async)]
fn note(state: State<'_, AppState>, id: String) -> Reply<NoteData> {
    state.core().note(&id)
}

#[tauri::command(async)]
fn new_note(state: State<'_, AppState>, notebook: Option<String>) -> Reply<NoteData> {
    state.core().new_note(notebook.as_deref())
}

#[tauri::command(async)]
fn save_note(state: State<'_, AppState>, edit: NoteEdit) -> Reply<NoteInfo> {
    state.core().save_note(edit)
}

#[tauri::command(async)]
fn trash_note(state: State<'_, AppState>, id: String) -> Reply<()> {
    state.core().trash_note(&id)
}

#[tauri::command(async)]
fn import_enex(state: State<'_, AppState>, paths: Vec<PathBuf>) -> Reply<Vec<ImportOutcome>> {
    state.core().import_enex(&paths)
}

/// Loads the demo package for the interface language ("sk" or "en") into
/// the separate demo archive.
#[tauri::command(async)]
fn load_demo(app: tauri::AppHandle, state: State<'_, AppState>, lang: String) -> Reply<Status> {
    let lang = if lang == "sk" { "sk" } else { "en" };
    let package = app
        .path()
        .resource_dir()
        .map_err(|e| CoreError::new_public("missing_demo_package", e.to_string()))?
        .join("demo")
        .join(format!("altais-demo-{lang}.json"));
    state.core().load_demo(&package)
}

#[tauri::command(async)]
fn open_demo(state: State<'_, AppState>) -> Reply<Status> {
    state.core().open_demo()
}

#[tauri::command(async)]
fn remove_demo(state: State<'_, AppState>) -> Reply<Status> {
    state.core().remove_demo()
}

#[tauri::command(async)]
fn set_password(state: State<'_, AppState>, password: String) -> Reply<Status> {
    state.core().set_password(password)
}

#[tauri::command(async)]
fn remove_password(state: State<'_, AppState>) -> Reply<Status> {
    state.core().remove_password()
}

#[tauri::command(async)]
fn new_recovery_key(state: State<'_, AppState>) -> Reply<String> {
    state.core().new_recovery_key()
}

/// Attachment content as raw bytes (an ArrayBuffer in the interface, not JSON).
#[tauri::command(async)]
fn attachment(state: State<'_, AppState>, blob: String) -> Reply<tauri::ipc::Response> {
    Ok(tauri::ipc::Response::new(state.core().attachment(&blob)?))
}

#[tauri::command(async)]
fn save_attachment(state: State<'_, AppState>, blob: String, path: PathBuf) -> Reply<()> {
    state.core().save_attachment(&blob, &path)
}

/// Opens an attachment in its default program, from a temporary decrypted
/// copy that is deleted when the archive locks or ALTAIS closes. Only the
/// application can open files this way; the window has no permission to.
#[tauri::command(async)]
fn open_attachment(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    blob: String,
    name: String,
) -> Reply<()> {
    let path = state.core().open_copy(&blob, &name)?;
    app.opener()
        .open_path(path.to_string_lossy(), None::<&str>)
        .map_err(|e| CoreError::new_public("cannot_open_file", e.to_string()))
}

#[tauri::command(async)]
fn add_attachment(state: State<'_, AppState>, path: PathBuf) -> Reply<AttachmentInfo> {
    state.core().add_attachment(&path)
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // %APPDATA%\io.github.tarzanislav.altais\archive on Windows.
            let dir = app.path().app_data_dir()?.join("archive");
            let keystore = OsKeyStore::system()
                .ok()
                .map(|s| Arc::new(s) as Arc<dyn KeyStore>);
            let mut core = Core::new(dir, keystore);
            core.set_open_copy_dir(std::env::temp_dir().join("ALTAIS-open"));
            app.manage(AppState(Mutex::new(core)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            status,
            create_archive,
            unlock,
            lock,
            notebooks,
            create_notebook,
            notes,
            search,
            note,
            new_note,
            save_note,
            trash_note,
            import_enex,
            attachment,
            save_attachment,
            open_attachment,
            add_attachment,
            set_password,
            remove_password,
            new_recovery_key,
            load_demo,
            open_demo,
            remove_demo
        ])
        .build(tauri::generate_context!())
        .expect("ALTAIS could not start")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<AppState>().core().clear_open_copies();
            }
        });
}
