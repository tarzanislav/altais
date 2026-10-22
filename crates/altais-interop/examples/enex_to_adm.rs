//! Converts an Evernote export into ALTAIS Document Model files.
//!
//! Usage: `cargo run -p altais-interop --example enex_to_adm -- <file.enex> <output-dir>`
//!
//! Writes one ADM JSON file per note, every attachment under its BLAKE3 hash,
//! and `report.json` with the result for each note.

use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::process::ExitCode;

use altais_interop::enex::import_all;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        eprintln!("usage: enex_to_adm <file.enex> <output-dir>");
        return ExitCode::from(2);
    };
    let file = match File::open(input) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("cannot open {input}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let (notes, summary) = import_all(BufReader::new(file));

    let output = PathBuf::from(output);
    let blobs_dir = output.join("blobs");
    let notes_dir = output.join("notes");
    if let Err(error) = fs::create_dir_all(&blobs_dir).and(fs::create_dir_all(&notes_dir)) {
        eprintln!("cannot create {}: {error}", output.display());
        return ExitCode::FAILURE;
    }
    let mut reports = Vec::new();
    for note in &notes {
        let path = notes_dir.join(format!(
            "{:05}-{}.json",
            note.report.index, note.document.id
        ));
        let json = note.document.to_json_pretty().expect("documents serialize");
        if let Err(error) = fs::write(&path, json) {
            eprintln!("cannot write {}: {error}", path.display());
            return ExitCode::FAILURE;
        }
        for blob in &note.blobs {
            let path = blobs_dir.join(blob.blob.hex());
            if let Err(error) = fs::write(&path, &blob.data) {
                eprintln!("cannot write {}: {error}", path.display());
                return ExitCode::FAILURE;
            }
        }
        reports.push(&note.report);
    }
    let report = serde_json::json!({ "summary": summary, "notes": reports });
    let report_path = output.join("report.json");
    if let Err(error) = fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).expect("report serializes"),
    ) {
        eprintln!("cannot write {}: {error}", report_path.display());
        return ExitCode::FAILURE;
    }

    println!(
        "notes: {} read, {} imported, {} partial; attachments: {}",
        summary.notes_read, summary.imported, summary.partial, summary.attachments
    );
    if let Some(reason) = &summary.stopped_early {
        println!("stopped early: {reason}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
