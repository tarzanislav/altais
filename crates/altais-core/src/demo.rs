//! Demo data (ADR-0005): a package of example notes loaded into a separate
//! demo archive, which can be removed completely.
//!
//! The demo archive lives in its own folder next to the user's archive and
//! never shares a file with it. Its content is public (CC0), so it is
//! protected only by a fixed password and no key is stored in the system;
//! removing the demo therefore means deleting its folder, nothing else.

use std::path::Path;

use altais_model::{Block, Document};
use altais_storage::{Archive, KdfParams};
use base64::Engine as _;
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::CoreError;

/// Password of every demo archive (the content is public).
pub(crate) const DEMO_PASSWORD: &str = "altais-demo";

/// Cheap key derivation for the demo archive: there is nothing secret in it.
pub(crate) const DEMO_KDF: KdfParams = KdfParams {
    memory_kib: 1024,
    iterations: 1,
    parallelism: 1,
};

#[derive(Deserialize)]
struct Package {
    format: String,
    version: u32,
    notebooks: Vec<PackageNotebook>,
    notes: Vec<PackageNote>,
    blobs: std::collections::BTreeMap<String, String>,
}

#[derive(Deserialize)]
struct PackageNotebook {
    key: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageNote {
    notebook: String,
    title: String,
    tags: Vec<String>,
    age_days: i64,
    content: Vec<Block>,
}

fn damaged(message: impl Into<String>) -> CoreError {
    CoreError::new("damaged_demo_package", message)
}

/// Fills an empty archive from a demo package; returns the number of notes.
pub(crate) fn load_package(archive: &mut Archive, path: &Path) -> Result<usize, CoreError> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| CoreError::new("cannot_read_file", format!("{}: {e}", path.display())))?;
    let package: Package = serde_json::from_str(&text).map_err(|e| damaged(e.to_string()))?;
    if package.format != "altais-demo" || package.version != 1 {
        return Err(damaged(format!(
            "unknown package {} {}",
            package.format, package.version
        )));
    }
    // Attachments are checked against their content address while stored.
    for (address, data) in &package.blobs {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| damaged(format!("attachment {address}: {e}")))?;
        let stored = archive.put_blob(&bytes)?;
        if stored.as_str() != address {
            return Err(damaged(format!(
                "attachment {address} does not match its content"
            )));
        }
    }
    let mut notebooks = std::collections::HashMap::new();
    for nb in &package.notebooks {
        notebooks.insert(nb.key.clone(), archive.create_notebook(&nb.name, None)?.id);
    }
    let now = Utc::now();
    for note in &package.notes {
        let mut doc = Document::new_note(note.title.clone());
        let when = now - Duration::days(note.age_days.max(0));
        doc.created = when;
        doc.updated = when;
        doc.tags = note.tags.clone();
        doc.notebook = Some(
            *notebooks
                .get(&note.notebook)
                .ok_or_else(|| damaged(format!("unknown notebook {}", note.notebook)))?,
        );
        doc.content = note.content.clone();
        archive.put_note(&doc)?;
    }
    Ok(package.notes.len())
}
