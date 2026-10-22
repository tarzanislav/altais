//! Evernote import into an archive (moves to altais-core with the desktop app).

use std::io::BufRead;
use std::path::Path;

use altais_interop::enex::{NoteStatus, read_enex};
use altais_storage::{Archive, StorageError};

/// What happened to one ENEX file.
#[derive(Debug, Default)]
pub struct Outcome {
    /// Notebook the notes went into.
    pub notebook: String,
    /// Notes stored.
    pub stored: usize,
    /// Of those, notes with content kept only in raw form.
    pub partial: Vec<(String, Vec<String>)>,
    /// Attachments stored.
    pub attachments: usize,
    /// Why reading stopped before the end, if it did.
    pub stopped_early: Option<String>,
}

impl Outcome {
    /// Prints a short report.
    pub fn print(&self, file: &Path) {
        println!(
            "{}: {} notes, {} attachments into notebook \"{}\"",
            file.display(),
            self.stored,
            self.attachments,
            self.notebook
        );
        for (title, problems) in &self.partial {
            println!("  partly imported: {title}");
            for p in problems {
                println!("    - {p}");
            }
        }
        if let Some(reason) = &self.stopped_early {
            println!("  stopped early: {reason}");
        }
    }
}

/// Imports every note of an ENEX file into a new notebook named `notebook`.
/// Notes are read one at a time, so large exports do not need much memory.
pub fn import_enex<R: BufRead>(
    archive: &mut Archive,
    input: R,
    notebook: &str,
) -> Result<Outcome, StorageError> {
    let nb = archive.create_notebook(notebook, None)?;
    let mut outcome = Outcome {
        notebook: notebook.to_owned(),
        ..Outcome::default()
    };
    for result in read_enex(input) {
        let mut imported = match result {
            Ok(note) => note,
            Err(e) => {
                outcome.stopped_early = Some(e.to_string());
                break;
            }
        };
        for blob in &imported.blobs {
            archive.put_blob(&blob.data)?;
            outcome.attachments += 1;
        }
        imported.document.notebook = Some(nb.id);
        archive.put_note(&imported.document)?;
        outcome.stored += 1;
        if imported.report.status == NoteStatus::Partial {
            outcome.partial.push((
                imported.report.title.clone(),
                imported
                    .report
                    .warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect(),
            ));
        }
    }
    Ok(outcome)
}
