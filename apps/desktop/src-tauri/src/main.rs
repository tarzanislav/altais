//! ALTAIS desktop application: a Tauri window around the web interface
//! (`apps/desktop/src`), with every operation handled by `altais-core`.

// No console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};

use altais_core::{
    Core, CoreError, ImportOutcome, NoteData, NoteEdit, NoteInfo, NotebookInfo, Secret, Status,
};
use altais_storage::{KeyStore, OsKeyStore};
use tauri::{Manager, State};

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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // %APPDATA%\io.github.tarzanislav.altais\archive on Windows.
            let dir = app.path().app_data_dir()?.join("archive");
            let keystore = OsKeyStore::system()
                .ok()
                .map(|s| Arc::new(s) as Arc<dyn KeyStore>);
            app.manage(AppState(Mutex::new(Core::new(dir, keystore))));
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
            import_enex
        ])
        .run(tauri::generate_context!())
        .expect("ALTAIS could not start");
}
