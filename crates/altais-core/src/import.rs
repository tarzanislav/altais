//! Evernote import into an archive.

use std::io::BufRead;

use altais_interop::enex::{NoteStatus, read_enex};
use altais_storage::{Archive, StorageError};
use serde::Serialize;

/// A note that was imported only partly, with the reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialNote {
    /// Title of the note.
    pub title: String,
    /// What could not be converted (English; the interface translates codes later).
    pub problems: Vec<String>,
}

/// What happened to one ENEX file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    /// The file.
    pub file: String,
    /// Notebook the notes went into.
    pub notebook: String,
    /// Notes stored.
    pub stored: usize,
    /// Notes with content kept only in raw form.
    pub partial: Vec<PartialNote>,
    /// Attachments stored.
    pub attachments: usize,
    /// Why reading stopped before the end, if it did.
    pub stopped_early: Option<String>,
}

impl ImportOutcome {
    /// A short report for the command line.
    pub fn report(&self) -> String {
        let mut out = format!(
            "{}: {} notes, {} attachments into notebook \"{}\"\n",
            self.file, self.stored, self.attachments, self.notebook
        );
        for note in &self.partial {
            out.push_str(&format!("  partly imported: {}\n", note.title));
            for p in &note.problems {
                out.push_str(&format!("    - {p}\n"));
            }
        }
        if let Some(reason) = &self.stopped_early {
            out.push_str(&format!("  stopped early: {reason}\n"));
        }
        out
    }
}

/// Imports every note of an ENEX file into a new notebook named `notebook`.
/// Notes are read one at a time, so large exports do not need much memory.
pub fn import_enex<R: BufRead>(
    archive: &mut Archive,
    input: R,
    notebook: &str,
) -> Result<ImportOutcome, StorageError> {
    let nb = archive.create_notebook(notebook, None)?;
    let mut outcome = ImportOutcome {
        notebook: notebook.to_owned(),
        ..ImportOutcome::default()
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
            outcome.partial.push(PartialNote {
                title: imported.report.title.clone(),
                problems: imported
                    .report
                    .warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect(),
            });
        }
    }
    Ok(outcome)
}
