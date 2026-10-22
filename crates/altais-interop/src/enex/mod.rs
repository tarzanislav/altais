//! Import of Evernote exports (ENEX) into the ALTAIS Document Model.
//!
//! The reader streams the file note by note, so exports of any size can be
//! imported with memory proportional to the largest single note.
//!
//! Guarantee: every note in the file is accounted for. It is either
//! converted, converted with parts kept in raw form (reported as
//! [`NoteStatus::Partial`]), or the import stops with an error that says after
//! how many notes. Attachments are kept byte for byte.

mod enml;
mod report;

use std::collections::HashMap;
use std::io::BufRead;

use altais_model::{
    ADM_VERSION, BlobRef, Block, Document, ObjectId, ObjectType, Provenance, ProvenanceKind,
    validate,
};
use base64::Engine as _;
use chrono::{DateTime, NaiveDateTime, Utc};
use md5::{Digest, Md5};
use quick_xml::Reader;
use quick_xml::events::Event;
use serde_json::{Map, Value, json};

use crate::xml::{self, Element, XmlError};
use enml::{Converter, ResourceInfo};
pub use report::{ImportSummary, NoteReport, NoteStatus, Severity, Warning};

/// Information from the `<en-export>` element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExportInfo {
    /// Application that wrote the file, for example `Evernote`.
    pub application: Option<String>,
    /// Version of that application.
    pub version: Option<String>,
    /// When the export was made, as written in the file.
    pub export_date: Option<String>,
}

/// An attachment of an imported note, with its original bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedBlob {
    /// Content address used by the ADM document.
    pub blob: BlobRef,
    /// MD5 hash used by Evernote, as lowercase hex.
    pub md5: String,
    /// MIME type.
    pub mime: String,
    /// File name.
    pub name: String,
    /// The original bytes.
    pub data: Vec<u8>,
}

/// One imported note.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportedNote {
    /// The note as an ADM document.
    pub document: Document,
    /// Its attachments.
    pub blobs: Vec<ImportedBlob>,
    /// What happened during conversion.
    pub report: NoteReport,
}

/// Error that stops reading an ENEX file.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EnexError {
    /// The file is not an Evernote export.
    #[error("not an Evernote export: expected <en-export>, found {0}")]
    NotEnex(String),
    /// The XML of the file is damaged.
    #[error("the export is damaged after {notes_read} notes: {source}")]
    Damaged {
        /// Notes read successfully before the damage.
        notes_read: usize,
        /// Description of the damage.
        source: XmlError,
    },
}

/// Streams the notes of an ENEX file.
pub struct EnexReader<R: BufRead> {
    reader: Reader<R>,
    export: Option<ExportInfo>,
    notes_read: usize,
    done: bool,
}

/// Starts reading an ENEX file.
pub fn read_enex<R: BufRead>(input: R) -> EnexReader<R> {
    let mut reader = Reader::from_reader(input);
    reader.config_mut().trim_text(false);
    EnexReader {
        reader,
        export: None,
        notes_read: 0,
        done: false,
    }
}

impl<R: BufRead> EnexReader<R> {
    /// Information from the `<en-export>` element, once it has been read.
    pub fn export_info(&self) -> Option<&ExportInfo> {
        self.export.as_ref()
    }

    fn damaged(&mut self, source: XmlError) -> Option<Result<ImportedNote, EnexError>> {
        self.done = true;
        Some(Err(EnexError::Damaged {
            notes_read: self.notes_read,
            source,
        }))
    }
}

impl<R: BufRead> Iterator for EnexReader<R> {
    type Item = Result<ImportedNote, EnexError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        let mut buf = Vec::new();
        loop {
            buf.clear();
            let position = self.reader.buffer_position();
            let event = match xml::next_event(&mut self.reader, &mut buf) {
                Ok(event) => event,
                Err(error) => return self.damaged(error),
            };
            match event {
                Event::Start(start) => {
                    let element = match xml::element_from_start(&start, position) {
                        Ok(element) => element,
                        Err(error) => return self.damaged(error),
                    };
                    match (element.name.as_str(), self.export.is_some()) {
                        ("en-export", false) => {
                            self.export = Some(ExportInfo {
                                application: element.attr("application").map(str::to_owned),
                                version: element.attr("version").map(str::to_owned),
                                export_date: element.attr("export-date").map(str::to_owned),
                            });
                        }
                        (name, false) => {
                            self.done = true;
                            return Some(Err(EnexError::NotEnex(format!("<{name}>"))));
                        }
                        ("note", true) => {
                            let mut unknown_entities = Vec::new();
                            let note =
                                match xml::read_subtree(&mut self.reader, element, &mut |name| {
                                    unknown_entities.push(name.to_owned())
                                }) {
                                    Ok(note) => note,
                                    Err(error) => return self.damaged(error),
                                };
                            let index = self.notes_read;
                            self.notes_read += 1;
                            let export = self.export.clone().unwrap_or_default();
                            return Some(Ok(convert_note(
                                &note,
                                index,
                                &export,
                                &unknown_entities,
                            )));
                        }
                        _ => {}
                    }
                }
                Event::Eof => {
                    self.done = true;
                    if self.export.is_none() {
                        return Some(Err(EnexError::NotEnex("an empty file".to_owned())));
                    }
                    return None;
                }
                _ => {}
            }
        }
    }
}

/// Imports a whole ENEX file and returns the notes and a summary.
///
/// Reading stops at the first damage in the file; the summary then says after
/// how many notes and why, so nothing is lost silently.
pub fn import_all<R: BufRead>(input: R) -> (Vec<ImportedNote>, ImportSummary) {
    let mut notes = Vec::new();
    let mut summary = ImportSummary::default();
    for result in read_enex(input) {
        match result {
            Ok(note) => {
                summary.notes_read += 1;
                summary.attachments += note.blobs.len();
                match note.report.status {
                    NoteStatus::Imported => summary.imported += 1,
                    NoteStatus::Partial => summary.partial += 1,
                }
                notes.push(note);
            }
            Err(error) => summary.stopped_early = Some(error.to_string()),
        }
    }
    (notes, summary)
}

/// Converts one `<note>` element.
fn convert_note(
    note: &Element,
    index: usize,
    export: &ExportInfo,
    unknown_entities: &[String],
) -> ImportedNote {
    let mut warnings = Vec::new();
    let title = note.child_text("title").unwrap_or_default();

    let created = parse_time(note.child_text("created").as_deref());
    let updated = parse_time(note.child_text("updated").as_deref());
    let created = created.or(updated).unwrap_or_else(|| {
        warnings.push(Warning::new(
            Severity::Notice,
            "the note has no creation time; the import time was used",
        ));
        Utc::now()
    });
    let mut updated = updated.unwrap_or(created);
    if updated < created {
        warnings.push(Warning::new(
            Severity::Info,
            "the change time was earlier than the creation time and was corrected",
        ));
        updated = created;
    }

    let mut tags = Vec::new();
    for tag in note.elements_named("tag") {
        let tag = tag.text().trim().to_owned();
        if !tag.is_empty() && !tags.contains(&tag) {
            tags.push(tag);
        }
    }

    // Attachments.
    let mut blobs = Vec::new();
    let mut resources = HashMap::new();
    let mut resource_meta = Vec::new();
    for (position, resource) in note.elements_named("resource").enumerate() {
        let mime = resource
            .child_text("mime")
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| "application/octet-stream".to_owned());
        let encoded: String = resource
            .child("data")
            .map(Element::text)
            .unwrap_or_default()
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        let attributes = resource.child("resource-attributes");
        let file_name = attributes
            .and_then(|a| a.child_text("file-name"))
            .filter(|n| !n.is_empty());
        let mut meta = element_fields(attributes);
        match base64::engine::general_purpose::STANDARD.decode(encoded.as_bytes()) {
            Ok(data) => {
                let md5 = hex(&Md5::digest(&data));
                let blob = BlobRef::try_from(format!("blake3:{}", blake3::hash(&data).to_hex()))
                    .expect("BLAKE3 hex is a valid blob reference");
                let name = file_name.unwrap_or_else(|| default_file_name(position, &mime));
                meta.insert("md5".to_owned(), json!(md5));
                meta.insert("blob".to_owned(), json!(blob.to_string()));
                for field in ["width", "height", "duration"] {
                    if let Some(value) = resource.child_text(field) {
                        meta.insert(field.to_owned(), json!(value));
                    }
                }
                resources.insert(
                    md5.clone(),
                    ResourceInfo {
                        blob: blob.clone(),
                        name: name.clone(),
                        mime: mime.clone(),
                        size: data.len() as u64,
                    },
                );
                blobs.push(ImportedBlob {
                    blob,
                    md5,
                    mime,
                    name,
                    data,
                });
            }
            Err(error) => {
                warnings.push(Warning::new(
                    Severity::Problem,
                    format!(
                        "attachment {} could not be decoded ({error}); its raw data is kept",
                        file_name.as_deref().unwrap_or("without a name")
                    ),
                ));
                meta.insert("undecodable-data".to_owned(), json!(encoded));
                meta.insert("mime".to_owned(), json!(mime));
            }
        }
        resource_meta.push(Value::Object(meta));
    }

    // Body.
    let enml = note.child("content").map(Element::text).unwrap_or_default();
    let mut converter = Converter::new(&resources);
    let mut entity_problems = Vec::new();
    let mut content = match xml::parse_document(&enml, &mut |name| {
        entity_problems.push(name.to_owned());
    }) {
        Ok(root) if root.name == "en-note" => converter.convert(&root),
        Ok(root) => vec![raw_enml(
            &mut converter,
            &enml,
            &format!(
                "the note body starts with <{}> instead of <en-note>",
                root.name
            ),
            &mut warnings,
        )],
        Err(error) => {
            if enml.trim().is_empty() {
                Vec::new()
            } else {
                vec![raw_enml(
                    &mut converter,
                    &enml,
                    &format!("the note body is not valid ENML ({error})"),
                    &mut warnings,
                )]
            }
        }
    };
    warnings.append(&mut converter.warnings);

    let unused: Vec<_> = blobs
        .iter()
        .filter(|blob| !converter.used_resources.contains(&blob.md5))
        .map(|blob| blob.md5.clone())
        .collect();
    if !unused.is_empty() {
        warnings.push(Warning::new(
            Severity::Notice,
            format!(
                "{} attachment(s) not shown in the note text were added at its end",
                unused.len()
            ),
        ));
        for md5 in unused {
            if let Some(info) = resources.get(&md5) {
                content.push(converter.attachment(info));
            }
        }
    }

    let mut all_entities: Vec<String> = unknown_entities.to_vec();
    all_entities.extend(entity_problems);
    all_entities.sort();
    all_entities.dedup();
    if !all_entities.is_empty() {
        warnings.push(Warning::new(
            Severity::Info,
            format!(
                "unknown character names kept as written: {}",
                all_entities
                    .iter()
                    .map(|e| format!("&{e};"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ));
    }

    // Everything Evernote knows that ALTAIS does not map yet is preserved.
    let mut evernote = Map::new();
    let attributes = element_fields(note.child("note-attributes"));
    if !attributes.is_empty() {
        evernote.insert("note-attributes".to_owned(), Value::Object(attributes));
    }
    if !resource_meta.is_empty() {
        evernote.insert("resources".to_owned(), Value::Array(resource_meta));
    }
    let tasks: Vec<Value> = note
        .elements_named("task")
        .map(|task| Value::Object(element_fields(Some(task))))
        .collect();
    if !tasks.is_empty() {
        warnings.push(Warning::new(
            Severity::Notice,
            format!(
                "{} Evernote task(s) were kept as data; they become ALTAIS tasks in a later version",
                tasks.len()
            ),
        ));
        evernote.insert("tasks".to_owned(), Value::Array(tasks));
    }
    let mut other = Map::new();
    for child in note.elements() {
        if !KNOWN_NOTE_FIELDS.contains(&child.name.as_str()) {
            other.insert(child.name.clone(), json!(child.text().trim()));
        }
    }
    if !other.is_empty() {
        evernote.insert("other".to_owned(), Value::Object(other));
    }
    evernote.insert(
        "export".to_owned(),
        json!({
            "application": export.application,
            "version": export.version,
            "export-date": export.export_date,
        }),
    );

    let mut extra = Map::new();
    extra.insert("x-evernote".to_owned(), Value::Object(evernote));

    let document = Document {
        adm: ADM_VERSION.to_owned(),
        id: ObjectId::new(),
        object_type: ObjectType::Note,
        title: (!title.is_empty()).then(|| title.clone()),
        created,
        updated,
        notebook: None,
        tags,
        lang: None,
        provenance: Some(Provenance {
            kind: ProvenanceKind::Import,
            source: Some("Evernote".to_owned()),
            captured: created,
        }),
        content,
        extra,
    };

    if let Err(errors) = validate(&document) {
        for error in errors {
            warnings.push(Warning::new(
                Severity::Problem,
                format!("the converted note is not valid: {error}"),
            ));
        }
    }

    warnings.sort_by_key(|w| std::cmp::Reverse(w.severity));
    let status = if warnings.iter().any(|w| w.severity == Severity::Problem) {
        NoteStatus::Partial
    } else {
        NoteStatus::Imported
    };
    let report = NoteReport {
        index,
        title,
        status,
        attachments: blobs.len(),
        warnings,
    };
    ImportedNote {
        document,
        blobs,
        report,
    }
}

const KNOWN_NOTE_FIELDS: &[&str] = &[
    "title",
    "created",
    "updated",
    "tag",
    "note-attributes",
    "content",
    "resource",
    "task",
];

/// Keeps an unparseable note body in raw form.
fn raw_enml(
    converter: &mut Converter<'_>,
    enml: &str,
    reason: &str,
    warnings: &mut Vec<Warning>,
) -> Block {
    warnings.push(Warning::new(
        Severity::Problem,
        format!("{reason}; the original body is kept unchanged"),
    ));
    Block::Unknown(json!({
        "t": "evernote_enml",
        "bid": converter.bid().to_string(),
        "enml": enml,
    }))
}

/// Child elements of `element` as a JSON object of their text values.
/// Repeated elements become arrays.
fn element_fields(element: Option<&Element>) -> Map<String, Value> {
    let mut map = Map::new();
    let Some(element) = element else {
        return map;
    };
    for child in element.elements() {
        let value = if child.elements().next().is_some() {
            Value::Object(element_fields(Some(child)))
        } else {
            json!(child.text().trim())
        };
        match map.get_mut(&child.name) {
            Some(Value::Array(values)) => values.push(value),
            Some(existing) => {
                let first = existing.take();
                *existing = Value::Array(vec![first, value]);
            }
            None => {
                map.insert(child.name.clone(), value);
            }
        }
    }
    map
}

/// Parses Evernote's timestamp format `20261022T201802Z`.
fn parse_time(value: Option<&str>) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(value?.trim(), "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|naive| naive.and_utc())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn default_file_name(position: usize, mime: &str) -> String {
    let extension = match mime {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/heic" => "heic",
        "image/svg+xml" => "svg",
        "application/pdf" => "pdf",
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/mp4" | "audio/x-m4a" => "m4a",
        "video/mp4" => "mp4",
        "text/plain" => "txt",
        "text/html" => "html",
        _ => "bin",
    };
    format!("attachment-{}.{extension}", position + 1)
}
