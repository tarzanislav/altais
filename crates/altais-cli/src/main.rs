//! `altais`: a small command-line tool for ALTAIS archives, until the desktop
//! application exists. It creates an archive, imports Evernote exports into
//! it, and searches and shows notes.

use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use altais_model::ObjectId;
use altais_storage::{Archive, KeyStore, OsKeyStore, RecoveryKey, StorageError, Unlock};

const HELP: &str = "\
altais — ALTAIS archive tool

Usage:
  altais init <folder>                        create an encrypted archive
  altais import <folder> <file.enex>… [--notebook <name>]
                                              import Evernote exports (one notebook per file)
  altais search <folder> <words…>             find notes (ignores case and diacritics)
  altais list <folder>                        notebooks and their notes
  altais show <folder> <note-id>              print a note as text

Unlocking: the key kept by Windows is used when present; otherwise the
password is read from ALTAIS_PASSWORD or asked for, or use
--recovery <recovery key>.
";

fn keystore() -> Option<Arc<dyn KeyStore>> {
    OsKeyStore::system()
        .ok()
        .map(|s| Arc::new(s) as Arc<dyn KeyStore>)
}

fn ask(prompt: &str) -> io::Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\r', '\n']).to_owned())
}

fn password() -> io::Result<String> {
    match std::env::var("ALTAIS_PASSWORD") {
        Ok(p) => Ok(p),
        Err(_) => ask("Password (shown as you type): "),
    }
}

fn open(folder: &Path, recovery: Option<&str>) -> Result<Archive, Box<dyn std::error::Error>> {
    if let Some(text) = recovery {
        let key = RecoveryKey::parse(text)?;
        return Ok(Archive::open(folder, keystore(), Unlock::Recovery(&key))?);
    }
    if !Archive::needs_password(folder)? {
        match Archive::open(folder, keystore(), Unlock::KeyStore) {
            Ok(archive) => return Ok(archive),
            Err(StorageError::KeyNotInKeyStore) => {
                eprintln!("The archive key is not stored on this computer.");
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(Archive::open(
        folder,
        keystore(),
        Unlock::Password(&password()?),
    )?)
}

fn init(folder: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let store = keystore();
    let password = if store.is_none() {
        eprintln!(
            "This system has no supported credential store, so the archive needs a password."
        );
        Some(password()?)
    } else {
        None
    };
    let (archive, recovery) = Archive::create(folder, store, password.as_deref())?;
    println!(
        "Created an encrypted archive in {}",
        archive.dir().display()
    );
    println!();
    println!("Your recovery key (write it down or print it and keep it safe):");
    println!();
    println!("    {}", recovery.display().as_str());
    println!();
    println!("It opens the archive on another computer or if you forget the password.");
    println!(
        "Without it, a forgotten password or a lost system key store means the notes are lost."
    );
    Ok(())
}

fn run(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let mut positional = Vec::new();
    let mut notebook = None;
    let mut recovery = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--notebook" => notebook = Some(it.next().ok_or("--notebook needs a name")?.clone()),
            "--recovery" => recovery = Some(it.next().ok_or("--recovery needs the key")?.clone()),
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(());
            }
            _ => positional.push(arg.clone()),
        }
    }
    let (command, rest) = positional.split_first().ok_or(HELP)?;
    let folder = PathBuf::from(rest.first().ok_or("missing <folder>")?);
    match command.as_str() {
        "init" => init(&folder),
        "import" => {
            let files = &rest[1..];
            if files.is_empty() {
                return Err("missing <file.enex>".into());
            }
            let mut archive = open(&folder, recovery.as_deref())?;
            for file in files {
                let path = Path::new(file);
                let name = notebook.clone().unwrap_or_else(|| {
                    path.file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "Evernote".into())
                });
                let reader = BufReader::new(std::fs::File::open(path)?);
                let mut outcome = altais_core::import::import_enex(&mut archive, reader, &name)?;
                outcome.file = path.display().to_string();
                print!("{}", outcome.report());
            }
            Ok(())
        }
        "search" => {
            let archive = open(&folder, recovery.as_deref())?;
            let query = rest[1..].join(" ");
            let hits = archive.search(&query, 50)?;
            if hits.is_empty() {
                println!("No notes found.");
            }
            for hit in hits {
                println!(
                    "{}  {}  {}",
                    hit.note.id,
                    hit.note.updated.format("%Y-%m-%d"),
                    hit.note.title
                );
            }
            Ok(())
        }
        "list" => {
            let archive = open(&folder, recovery.as_deref())?;
            for nb in archive.notebooks()? {
                println!("{}", nb.name);
                for note in archive.notes(Some(nb.id))? {
                    println!(
                        "  {}  {}  {}",
                        note.id,
                        note.updated.format("%Y-%m-%d"),
                        note.title
                    );
                }
            }
            println!("{} notes in total.", archive.note_count()?);
            Ok(())
        }
        "show" => {
            let archive = open(&folder, recovery.as_deref())?;
            let id: ObjectId = rest.get(1).ok_or("missing <note-id>")?.parse()?;
            let doc = archive.note(id)?.ok_or("no note with this id")?;
            println!("{}", doc.title.as_deref().unwrap_or("(untitled)"));
            let tags = archive.tags_of(id)?;
            if !tags.is_empty() {
                println!("Tags: {}", tags.join(", "));
            }
            println!();
            print!("{}", altais_storage::note_text(&doc));
            Ok(())
        }
        other => Err(format!("unknown command {other:?}\n\n{HELP}").into()),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("altais: {e}");
            ExitCode::FAILURE
        }
    }
}
