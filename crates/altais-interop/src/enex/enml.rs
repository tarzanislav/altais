//! Conversion of ENML (the XHTML dialect of Evernote note bodies) into ADM blocks.
//!
//! Principle: no text is ever dropped. Elements that have no ADM equivalent
//! keep their text; content that cannot be represented at all is preserved
//! as an unknown block with the original data.

use std::collections::{BTreeSet, HashMap, HashSet};

use altais_model::{
    BlobRef, Block, BlockId, KnownBlock, ListItem, ListKind, Mark, SimpleMark, Span, TableCell,
    TableRow,
};
use serde_json::json;

use super::report::{Severity, Warning};
use crate::xml::{Element, Node};

/// An attachment that ENML can reference by its MD5 hash.
#[derive(Debug, Clone)]
pub(crate) struct ResourceInfo {
    pub blob: BlobRef,
    pub name: String,
    pub mime: String,
    pub size: u64,
}

/// Converts one note body. Block IDs are numbered `b0001`, `b0002`, ... so a
/// repeated import of the same note produces the same IDs.
pub(crate) struct Converter<'a> {
    resources: &'a HashMap<String, ResourceInfo>,
    pub used_resources: HashSet<String>,
    pub warnings: Vec<Warning>,
    next_id: u32,
    legacy_todo_lists: HashSet<String>,
    ignored_styles: BTreeSet<String>,
    unknown_elements: BTreeSet<String>,
}

impl<'a> Converter<'a> {
    pub fn new(resources: &'a HashMap<String, ResourceInfo>) -> Self {
        Self {
            resources,
            used_resources: HashSet::new(),
            warnings: Vec::new(),
            next_id: 0,
            legacy_todo_lists: HashSet::new(),
            ignored_styles: BTreeSet::new(),
            unknown_elements: BTreeSet::new(),
        }
    }

    /// Returns the next block ID.
    pub fn bid(&mut self) -> BlockId {
        self.next_id += 1;
        BlockId::try_from(format!("b{:04}", self.next_id)).expect("generated block IDs are valid")
    }

    /// Converts the children of `<en-note>`.
    pub fn convert(&mut self, en_note: &Element) -> Vec<Block> {
        let blocks = self.blocks(&en_note.children);
        self.finish_warnings();
        blocks
    }

    /// Builds an attachment block for a resource.
    pub fn attachment(&mut self, info: &ResourceInfo) -> Block {
        KnownBlock::Attachment {
            bid: self.bid(),
            blob: info.blob.clone(),
            name: info.name.clone(),
            mime: info.mime.clone(),
            size: info.size,
        }
        .into()
    }

    fn finish_warnings(&mut self) {
        if !self.ignored_styles.is_empty() {
            let list: Vec<_> = self.ignored_styles.iter().cloned().collect();
            self.warnings.push(Warning::new(
                Severity::Info,
                format!("presentational formatting ignored: {}", list.join(", ")),
            ));
        }
        if !self.unknown_elements.is_empty() {
            let list: Vec<_> = self.unknown_elements.iter().cloned().collect();
            self.warnings.push(Warning::new(
                Severity::Info,
                format!(
                    "elements without an ALTAIS equivalent, text kept: {}",
                    list.join(", ")
                ),
            ));
        }
    }

    /// Converts a sequence of nodes that may mix inline and block content.
    fn blocks(&mut self, nodes: &[Node]) -> Vec<Block> {
        let mut out = Vec::new();
        let mut buf = InlineBuf::default();
        let mut marks = Vec::new();
        for node in nodes {
            match node {
                Node::Text(text) => buf.push_text(text, &marks),
                Node::Element(element) if is_block(element) => {
                    self.flush(&mut buf, &mut out);
                    self.block(element, &mut out);
                }
                Node::Element(element) => self.inline(element, &mut marks, &mut buf),
            }
        }
        self.flush(&mut buf, &mut out);
        self.merge_legacy_todo_lists(out)
    }

    /// Turns buffered inline content into paragraphs.
    fn flush(&mut self, buf: &mut InlineBuf, out: &mut Vec<Block>) {
        let todo = buf.todo.take();
        let mut first = true;
        for segment in buf.segments.drain(..) {
            match segment {
                Segment::Spans { spans, had_break } => {
                    let Some(text) = finalize_spans(spans, had_break) else {
                        continue;
                    };
                    let paragraph: Block = KnownBlock::Paragraph {
                        bid: self.bid(),
                        text,
                    }
                    .into();
                    match todo {
                        Some(checked) if first => {
                            let bid = self.bid();
                            self.legacy_todo_lists.insert(bid.to_string());
                            let item_bid = self.bid();
                            out.push(
                                KnownBlock::List {
                                    bid,
                                    kind: ListKind::Check,
                                    items: vec![ListItem {
                                        bid: item_bid,
                                        checked: Some(checked),
                                        content: vec![paragraph],
                                    }],
                                }
                                .into(),
                            );
                        }
                        _ => out.push(paragraph),
                    }
                    first = false;
                }
                Segment::Blocks(blocks) => out.extend(blocks),
            }
        }
    }

    /// Merges consecutive one-item checklists created from legacy `<en-todo>`.
    fn merge_legacy_todo_lists(&self, blocks: Vec<Block>) -> Vec<Block> {
        let mut out: Vec<Block> = Vec::with_capacity(blocks.len());
        for block in blocks {
            let is_legacy = |b: &Block| {
                b.bid()
                    .is_some_and(|bid| self.legacy_todo_lists.contains(bid))
            };
            if is_legacy(&block) && out.last().is_some_and(is_legacy) {
                if let (
                    Some(Block::Known(KnownBlock::List { items, .. })),
                    Block::Known(KnownBlock::List { items: more, .. }),
                ) = (out.last_mut(), block)
                {
                    items.extend(more);
                }
                continue;
            }
            out.push(block);
        }
        out
    }

    /// Converts one block-level element.
    fn block(&mut self, element: &Element, out: &mut Vec<Block>) {
        let style = Style::parse(element.attr("style"));
        match element.name.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                let level = element.name[1..].parse::<u8>().unwrap_or(4).min(4);
                let bid = self.bid();
                let (text, rest) = self.inline_content(element);
                out.push(
                    KnownBlock::Heading {
                        bid,
                        level,
                        text: finalize_spans(text, false).unwrap_or_default(),
                    }
                    .into(),
                );
                out.extend(rest);
            }
            "ul" | "ol" => out.push(self.list(element, &style)),
            "table" => out.push(self.table(element)),
            "hr" => out.push(KnownBlock::Divider { bid: self.bid() }.into()),
            "blockquote" => {
                let bid = self.bid();
                let content = self.blocks(&element.children);
                out.push(KnownBlock::Quote { bid, content }.into());
            }
            "pre" => out.push(
                KnownBlock::Code {
                    bid: self.bid(),
                    lang: None,
                    text: code_text(element),
                }
                .into(),
            ),
            "en-media" => out.push(self.media(element)),
            "en-crypt" => out.push(self.encrypted(element)),
            _ if style.is("display", "none") => {
                if element.text().trim().is_empty() {
                    // Evernote keeps editor settings (for example heading styles) in
                    // hidden elements without text; they are not content.
                    if style.has("--en-chs") {
                        self.ignored_styles
                            .insert("editor style settings".to_owned());
                    }
                } else {
                    self.warnings.push(Warning::new(
                        Severity::Notice,
                        "hidden text in the note was imported as visible text",
                    ));
                    out.extend(self.blocks(&element.children));
                }
            }
            _ if style.is("--en-codeblock", "true") => out.push(
                KnownBlock::Code {
                    bid: self.bid(),
                    lang: None,
                    text: code_text(element),
                }
                .into(),
            ),
            _ if style.is("--en-richlink", "true") => {
                let url = style
                    .get("--en-href")
                    .map(str::to_owned)
                    .or_else(|| first_link(element))
                    .unwrap_or_default();
                let title = element.text().trim().to_owned();
                out.push(
                    KnownBlock::Bookmark {
                        bid: self.bid(),
                        url,
                        title: (!title.is_empty()).then_some(title),
                    }
                    .into(),
                );
            }
            name => {
                if !CONTAINER_BLOCKS.contains(&name) {
                    self.unknown_elements.insert(format!("<{name}>"));
                }
                out.extend(self.blocks(&element.children));
            }
        }
    }

    /// Collects the inline content of an element; atomic blocks found inside
    /// (attachments, tables, ...) are returned separately.
    fn inline_content(&mut self, element: &Element) -> (Vec<Span>, Vec<Block>) {
        let mut buf = InlineBuf::default();
        let mut marks = self.marks_of(element);
        for node in &element.children {
            match node {
                Node::Text(text) => buf.push_text(text, &marks),
                Node::Element(child) => self.inline(child, &mut marks, &mut buf),
            }
        }
        let mut spans = Vec::new();
        let mut blocks = Vec::new();
        for segment in buf.segments {
            match segment {
                Segment::Spans { spans: more, .. } => spans.extend(more),
                Segment::Blocks(more) => blocks.extend(more),
            }
        }
        (spans, blocks)
    }

    /// Converts an inline element (and its children) into the buffer.
    fn inline(&mut self, element: &Element, marks: &mut Vec<Mark>, buf: &mut InlineBuf) {
        match element.name.as_str() {
            "br" => return buf.push_break(),
            "en-todo" => {
                buf.todo = Some(element.attr("checked") == Some("true"));
                return;
            }
            name if ATOMIC_BLOCKS.contains(&name) => {
                let mut blocks = Vec::new();
                self.block(element, &mut blocks);
                buf.push_blocks(blocks);
                return;
            }
            name if !INLINE_ELEMENTS.contains(&name) && !CONTAINER_BLOCKS.contains(&name) => {
                self.unknown_elements.insert(format!("<{name}>"));
            }
            _ => {}
        }
        let added = self.marks_of(element);
        let before = marks.len();
        for mark in added {
            if !marks.contains(&mark) {
                marks.push(mark);
            }
        }
        for node in &element.children {
            match node {
                Node::Text(text) => buf.push_text(text, marks),
                Node::Element(child) => self.inline(child, marks, buf),
            }
        }
        marks.truncate(before);
    }

    /// Marks implied by an element's name and inline style.
    fn marks_of(&mut self, element: &Element) -> Vec<Mark> {
        let mut marks = Vec::new();
        let simple = match element.name.as_str() {
            "b" | "strong" => Some(SimpleMark::Bold),
            "i" | "em" | "cite" => Some(SimpleMark::Italic),
            "u" | "ins" => Some(SimpleMark::Underline),
            "s" | "strike" | "del" => Some(SimpleMark::Strike),
            "sub" => Some(SimpleMark::Subscript),
            "sup" => Some(SimpleMark::Superscript),
            "code" | "tt" | "kbd" | "samp" => Some(SimpleMark::Code),
            "mark" => Some(SimpleMark::Highlight),
            _ => None,
        };
        marks.extend(simple.map(Mark::Simple));
        if element.name == "a"
            && let Some(href) = element.attr("href").filter(|h| !h.is_empty())
        {
            marks.push(Mark::Link {
                link: href.to_owned(),
            });
        }
        if element.name == "font"
            && let Some(color) = element.attr("color")
        {
            marks.push(Mark::Color {
                color: normalize_color(color),
            });
        }
        let style = Style::parse(element.attr("style"));
        for (key, value) in &style.props {
            match key.as_str() {
                "color" => marks.push(Mark::Color {
                    color: normalize_color(value),
                }),
                "background-color" | "--en-highlight" => {}
                "font-weight" if is_bold_weight(value) => {
                    marks.push(Mark::Simple(SimpleMark::Bold))
                }
                "font-style" if value.contains("italic") => {
                    marks.push(Mark::Simple(SimpleMark::Italic));
                }
                "text-decoration" | "text-decoration-line" => {
                    if value.contains("underline") {
                        marks.push(Mark::Simple(SimpleMark::Underline));
                    }
                    if value.contains("line-through") {
                        marks.push(Mark::Simple(SimpleMark::Strike));
                    }
                }
                key if key.starts_with("--en-") || key.starts_with("--inversion") => {}
                "font-weight" | "font-style" => {}
                key => {
                    self.ignored_styles.insert(key.to_owned());
                }
            }
        }
        if let Some(name) = style.get("--en-highlight") {
            marks.push(Mark::HighlightColor {
                highlight: name.to_owned(),
            });
        } else if let Some(color) = style.get("background-color") {
            marks.push(Mark::HighlightColor {
                highlight: normalize_color(color),
            });
        }
        marks
    }

    fn list(&mut self, element: &Element, style: &Style) -> Block {
        let kind = if style.is("--en-todo", "true") {
            ListKind::Check
        } else if element.name == "ol" {
            ListKind::Ordered
        } else {
            ListKind::Bullet
        };
        let bid = self.bid();
        let mut items = Vec::new();
        for node in &element.children {
            match node {
                Node::Element(li) if li.name == "li" => {
                    let checked = match Style::parse(li.attr("style")).get("--en-checked") {
                        Some("true") => Some(true),
                        Some("false") => Some(false),
                        _ => None,
                    };
                    let item_bid = self.bid();
                    let content = self.blocks(&li.children);
                    items.push(ListItem {
                        bid: item_bid,
                        checked,
                        content,
                    });
                }
                Node::Element(nested) if nested.name == "ul" || nested.name == "ol" => {
                    // Evernote writes indentation as a list directly inside a list.
                    let item_bid = self.bid();
                    let nested_style = Style::parse(nested.attr("style"));
                    let content = vec![self.list(nested, &nested_style)];
                    items.push(ListItem {
                        bid: item_bid,
                        checked: None,
                        content,
                    });
                }
                Node::Text(text) if text.trim().is_empty() => {}
                other => {
                    let item_bid = self.bid();
                    let content = self.blocks(std::slice::from_ref(other));
                    items.push(ListItem {
                        bid: item_bid,
                        checked: None,
                        content,
                    });
                }
            }
        }
        KnownBlock::List { bid, kind, items }.into()
    }

    fn table(&mut self, element: &Element) -> Block {
        let bid = self.bid();
        let mut rows = Vec::new();
        self.collect_rows(element, &mut rows);
        KnownBlock::Table { bid, rows }.into()
    }

    fn collect_rows(&mut self, element: &Element, rows: &mut Vec<TableRow>) {
        for child in element.elements() {
            match child.name.as_str() {
                "tr" => {
                    let mut cells = Vec::new();
                    for cell in child.elements() {
                        if cell.name != "td" && cell.name != "th" {
                            continue;
                        }
                        let content = self.blocks(&cell.children);
                        cells.push(TableCell {
                            content,
                            colspan: span_attr(cell.attr("colspan")),
                            rowspan: span_attr(cell.attr("rowspan")),
                            header: cell.name == "th",
                        });
                    }
                    rows.push(TableRow { cells });
                }
                "thead" | "tbody" | "tfoot" => self.collect_rows(child, rows),
                "caption" => {
                    let text = child.text();
                    if !text.trim().is_empty() {
                        let content = vec![
                            KnownBlock::Paragraph {
                                bid: self.bid(),
                                text: vec![Span::plain(text.trim())],
                            }
                            .into(),
                        ];
                        rows.insert(
                            0,
                            TableRow {
                                cells: vec![TableCell {
                                    content,
                                    colspan: 1,
                                    rowspan: 1,
                                    header: true,
                                }],
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    }

    fn media(&mut self, element: &Element) -> Block {
        let hash = element
            .attr("hash")
            .unwrap_or_default()
            .to_ascii_lowercase();
        if let Some(info) = self.resources.get(&hash).cloned() {
            self.used_resources.insert(hash);
            return self.attachment(&info);
        }
        self.warnings.push(Warning::new(
            Severity::Problem,
            format!(
                "the note refers to an attachment that is missing from the export (MD5 {hash})"
            ),
        ));
        Block::Unknown(json!({
            "t": "missing_attachment",
            "bid": self.bid().to_string(),
            "md5": hash,
            "mime": element.attr("type"),
        }))
    }

    fn encrypted(&mut self, element: &Element) -> Block {
        self.warnings.push(Warning::new(
            Severity::Notice,
            "encrypted text was kept locked as it is in Evernote",
        ));
        Block::Unknown(json!({
            "t": "evernote_encrypted",
            "bid": self.bid().to_string(),
            "cipher": element.attr("cipher").unwrap_or("AES"),
            "length": element.attr("length").and_then(|l| l.parse::<u32>().ok()),
            "hint": element.attr("hint"),
            "ciphertext": element.text().trim(),
        }))
    }
}

/// Elements whose content is laid out as blocks.
const CONTAINER_BLOCKS: &[&str] = &[
    "div",
    "p",
    "center",
    "section",
    "article",
    "header",
    "footer",
    "aside",
    "nav",
    "main",
    "address",
    "figure",
    "figcaption",
    "li",
    "dl",
    "dt",
    "dd",
    "tbody",
    "thead",
    "tfoot",
    "tr",
    "td",
    "th",
    "en-note",
];

/// Block elements that are converted as a whole, even inside inline content.
const ATOMIC_BLOCKS: &[&str] = &[
    "en-media",
    "en-crypt",
    "table",
    "ul",
    "ol",
    "hr",
    "pre",
    "blockquote",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
];

/// Inline elements that ALTAIS understands.
const INLINE_ELEMENTS: &[&str] = &[
    "span", "font", "a", "b", "strong", "i", "em", "cite", "u", "ins", "s", "strike", "del", "sub",
    "sup", "code", "tt", "kbd", "samp", "mark", "small", "big", "abbr", "acronym", "q", "dfn",
    "var", "bdo", "label", "br", "en-todo",
];

fn is_block(element: &Element) -> bool {
    let name = element.name.as_str();
    ATOMIC_BLOCKS.contains(&name) || (CONTAINER_BLOCKS.contains(&name) && name != "en-note")
}

/// Buffered inline content between block elements.
#[derive(Default)]
struct InlineBuf {
    segments: Vec<Segment>,
    todo: Option<bool>,
}

enum Segment {
    Spans { spans: Vec<Span>, had_break: bool },
    Blocks(Vec<Block>),
}

impl InlineBuf {
    fn current(&mut self) -> (&mut Vec<Span>, &mut bool) {
        if !matches!(self.segments.last(), Some(Segment::Spans { .. })) {
            self.segments.push(Segment::Spans {
                spans: Vec::new(),
                had_break: false,
            });
        }
        match self.segments.last_mut() {
            Some(Segment::Spans { spans, had_break }) => (spans, had_break),
            _ => unreachable!("a spans segment was just pushed"),
        }
    }

    fn push_text(&mut self, text: &str, marks: &[Mark]) {
        let collapsed = collapse_whitespace(text);
        if collapsed.is_empty() {
            return;
        }
        let (spans, _) = self.current();
        match spans.last_mut() {
            Some(last) if last.marks == marks => last.s.push_str(&collapsed),
            _ => spans.push(Span {
                s: collapsed,
                marks: marks.to_vec(),
            }),
        }
    }

    fn push_break(&mut self) {
        let (spans, had_break) = self.current();
        *had_break = true;
        match spans.last_mut() {
            Some(last) => last.s.push('\n'),
            None => spans.push(Span::plain("\n")),
        }
    }

    fn push_blocks(&mut self, blocks: Vec<Block>) {
        if !blocks.is_empty() {
            self.segments.push(Segment::Blocks(blocks));
        }
    }
}

/// Cleans the spans of one paragraph. Returns `None` when there is nothing to
/// keep: no text and no explicit line break.
fn finalize_spans(mut spans: Vec<Span>, had_break: bool) -> Option<Vec<Span>> {
    if let Some(first) = spans.first_mut() {
        first.s = first.s.trim_start_matches(' ').to_owned();
    }
    if let Some(last) = spans.last_mut() {
        let trimmed = last.s.trim_end_matches(' ');
        // A line break at the very end of a block is not visible in Evernote.
        let trimmed = trimmed.strip_suffix('\n').unwrap_or(trimmed);
        last.s = trimmed.trim_end_matches(' ').to_owned();
    }
    for span in &mut spans {
        span.s = span.s.replace(" \n", "\n").replace("\n ", "\n");
    }
    spans.retain(|span| !span.s.is_empty());
    if spans.is_empty() && !had_break {
        None
    } else {
        Some(spans)
    }
}

/// Collapses runs of XML whitespace into one space, as HTML rendering does.
/// Non-breaking spaces are kept.
fn collapse_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    out
}

/// Text of a code block: line breaks and nested lines become `\n`.
fn code_text(element: &Element) -> String {
    fn walk(nodes: &[Node], out: &mut String) {
        for node in nodes {
            match node {
                Node::Text(text) => out.push_str(text),
                Node::Element(child) if child.name == "br" => out.push('\n'),
                Node::Element(child) if child.name == "div" || child.name == "p" => {
                    if !out.is_empty() && !out.ends_with('\n') {
                        out.push('\n');
                    }
                    walk(&child.children, out);
                }
                Node::Element(child) => walk(&child.children, out),
            }
        }
    }
    let mut out = String::new();
    walk(&element.children, &mut out);
    out.trim_end_matches('\n').to_owned()
}

fn first_link(element: &Element) -> Option<String> {
    if element.name == "a" {
        return element.attr("href").map(str::to_owned);
    }
    element.elements().find_map(first_link)
}

fn span_attr(value: Option<&str>) -> u32 {
    value
        .and_then(|v| v.trim().parse::<u32>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(1)
}

fn is_bold_weight(value: &str) -> bool {
    matches!(value.trim(), "bold" | "bolder")
        || value
            .trim()
            .parse::<u32>()
            .is_ok_and(|weight| weight >= 600)
}

/// Normalizes CSS colours: `rgb(229, 158, 37)` and `#E59E25` become `#e59e25`.
fn normalize_color(value: &str) -> String {
    let value = value.trim().to_ascii_lowercase();
    if let Some(inner) = value
        .strip_prefix("rgb(")
        .or_else(|| value.strip_prefix("rgba("))
        .and_then(|v| v.strip_suffix(')'))
    {
        let parts: Vec<u8> = inner
            .split(',')
            .take(3)
            .filter_map(|p| p.trim().parse::<u8>().ok())
            .collect();
        if let [r, g, b] = parts[..] {
            return format!("#{r:02x}{g:02x}{b:02x}");
        }
    }
    if let Some(hex) = value.strip_prefix('#')
        && hex.len() == 3
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return hex.chars().fold(String::from("#"), |mut acc, c| {
            acc.push(c);
            acc.push(c);
            acc
        });
    }
    value
}

/// A parsed `style` attribute.
#[derive(Debug, Default)]
struct Style {
    props: Vec<(String, String)>,
}

impl Style {
    fn parse(style: Option<&str>) -> Self {
        let props = style
            .unwrap_or_default()
            .split(';')
            .filter_map(|decl| {
                let (key, value) = decl.split_once(':')?;
                let key = key.trim().to_ascii_lowercase();
                (!key.is_empty()).then(|| (key, value.trim().to_owned()))
            })
            .collect();
        Self { props }
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }

    fn is(&self, key: &str, value: &str) -> bool {
        self.get(key).is_some_and(|v| v.eq_ignore_ascii_case(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_are_normalized() {
        assert_eq!(normalize_color("rgb(229, 158, 37)"), "#e59e25");
        assert_eq!(normalize_color("#ABC"), "#aabbcc");
        assert_eq!(normalize_color("Red"), "red");
    }

    #[test]
    fn whitespace_is_collapsed_but_nbsp_kept() {
        assert_eq!(
            collapse_whitespace("a \n\t b\u{a0}\u{a0}c"),
            "a b\u{a0}\u{a0}c"
        );
    }

    #[test]
    fn styles_are_parsed() {
        let style = Style::parse(Some("--en-todo:true; color: red ;bad"));
        assert!(style.is("--en-todo", "true"));
        assert_eq!(style.get("color"), Some("red"));
        assert!(!style.has("bad"));
    }
}
