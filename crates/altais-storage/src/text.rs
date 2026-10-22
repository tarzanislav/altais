//! Plain text of a note for the search index.

use altais_model::Document;
use serde_json::Value;

/// Fields whose string values are searchable text. Encrypted Evernote text
/// (`ciphertext`, `hint`) and technical fields (`bid`, `blob`, `mime`) are not.
const TEXT_FIELDS: &[&str] = &["text", "title", "name", "url", "caption"];

fn collect(value: &Value, out: &mut String) {
    match value {
        Value::Array(items) => items.iter().for_each(|v| collect(v, out)),
        Value::Object(map) => {
            for (key, v) in map {
                match v {
                    // Spans of one paragraph run together.
                    Value::String(s) if key == "s" => out.push_str(s),
                    Value::String(s) if key == "enml" => {
                        out.push_str(&strip_markup(s));
                        out.push('\n');
                    }
                    Value::String(s) if TEXT_FIELDS.contains(&key.as_str()) => {
                        out.push_str(s);
                        out.push('\n');
                    }
                    Value::Array(_) | Value::Object(_) => collect(v, out),
                    _ => {}
                }
            }
            // Each block ends its line.
            if map.contains_key("t") && !out.ends_with('\n') {
                out.push('\n');
            }
        }
        _ => {}
    }
}

/// Text of an ENML body kept unconverted (damaged XML): tags become spaces.
fn strip_markup(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut in_tag = false;
    for c in xml.chars() {
        match c {
            '<' => in_tag = true,
            '>' if in_tag => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}

/// All searchable text of the note's content (not the title), one piece per line.
pub fn body_text(doc: &Document) -> String {
    let mut out = String::new();
    for block in &doc.content {
        match serde_json::to_value(block) {
            Ok(value) => collect(&value, &mut out),
            Err(_) => continue,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_text_but_not_secrets_or_ids() {
        let doc =
            Document::from_json(include_str!("../../../spec/adm/examples/rich-note.json")).unwrap();
        let text = body_text(&doc);
        for expected in [
            "Výlet",
            "dôležité",
            "Kúpiť lístky",
            "raňajky",
            "celý deň voľno",
            "Hory volajú.",
            "\"km\": 12",
            "Video o trase",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text:?}");
        }
        let raw = r#"{"adm":"0.1","id":"0192b6e1-2d3c-7f40-8a51-6b7c8d9e0f12","type":"note","created":"2026-10-20T08:00:00Z","updated":"2026-10-20T08:00:00Z","content":[{"t":"evernote_enml","bid":"b0001","enml":"<en-note><div>poškodená <b>poznámka</div></en-note>"}]}"#;
        let damaged = body_text(&Document::from_json(raw).unwrap());
        assert!(
            damaged.contains("poškodená")
                && damaged.contains("poznámka")
                && !damaged.contains("en-note")
        );
        for hidden in ["RU5DMI1mnQ7fKjBk", "obvyklé", "b0001"] {
            assert!(!text.contains(hidden), "{hidden:?} must not be indexed");
        }
    }
}
