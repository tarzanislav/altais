//! Import report: what happened to every note.

use serde::Serialize;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Nothing was lost; for example presentational formatting was ignored.
    Info,
    /// Content was kept in a form the user should know about, for example
    /// encrypted text that stays locked or Evernote tasks kept as data.
    Notice,
    /// Part of the note could not be converted; it is kept in raw form so
    /// nothing is lost, but it needs attention.
    Problem,
}

/// One finding about a note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Warning {
    /// How serious the finding is.
    pub severity: Severity,
    /// Description in English; the application translates it for display.
    pub message: String,
}

impl Warning {
    pub(crate) fn new(severity: Severity, message: impl Into<String>) -> Self {
        Self {
            severity,
            message: message.into(),
        }
    }
}

/// Result for one note.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NoteStatus {
    /// Converted completely.
    Imported,
    /// Converted, but some content is kept only in raw form (see the warnings).
    Partial,
}

/// Report for one note.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NoteReport {
    /// Position of the note in the ENEX file, starting at 0.
    pub index: usize,
    /// Title of the note.
    pub title: String,
    /// Overall result.
    pub status: NoteStatus,
    /// Number of attachments imported with the note.
    pub attachments: usize,
    /// Findings, most important first.
    pub warnings: Vec<Warning>,
}

/// Summary of a whole import.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportSummary {
    /// Notes read from the file.
    pub notes_read: usize,
    /// Notes converted completely.
    pub imported: usize,
    /// Notes converted with content kept in raw form.
    pub partial: usize,
    /// Attachments imported.
    pub attachments: usize,
    /// If the file could not be read to the end: why, and after how many notes.
    pub stopped_early: Option<String>,
}
