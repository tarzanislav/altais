//! A small, iterative XML tree builder on top of `quick-xml`.
//!
//! It is used for single ENEX notes and for ENML note bodies. Building the
//! tree iteratively (not recursively) keeps hostile, deeply nested input from
//! exhausting the stack.

use std::io::BufRead;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

/// Maximum nesting depth accepted in a single tree.
pub(crate) const MAX_DEPTH: usize = 512;

/// An XML element with its attributes and children.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

/// A node in the tree.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Element(Element),
    Text(String),
}

impl Element {
    /// Returns the value of an attribute.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Returns the first child element with the given name.
    pub fn child(&self, name: &str) -> Option<&Element> {
        self.elements().find(|element| element.name == name)
    }

    /// Returns all child elements.
    pub fn elements(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|node| match node {
            Node::Element(element) => Some(element),
            Node::Text(_) => None,
        })
    }

    /// Returns all child elements with the given name.
    pub fn elements_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.elements().filter(move |element| element.name == name)
    }

    /// Returns the concatenated text of all descendants.
    pub fn text(&self) -> String {
        let mut out = String::new();
        collect_text(&self.children, &mut out);
        out
    }

    /// Returns the trimmed text of the first child element with the given name.
    pub fn child_text(&self, name: &str) -> Option<String> {
        self.child(name)
            .map(|element| element.text().trim().to_owned())
    }
}

fn collect_text(nodes: &[Node], out: &mut String) {
    for node in nodes {
        match node {
            Node::Text(text) => out.push_str(text),
            Node::Element(element) => collect_text(&element.children, out),
        }
    }
}

/// Error while building a tree.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XmlError {
    /// The XML is not well formed.
    #[error("XML error at byte {position}: {message}")]
    Syntax {
        /// Byte offset where the problem was found.
        position: u64,
        /// Description from the XML parser.
        message: String,
    },
    /// Elements are nested deeper than [`MAX_DEPTH`].
    #[error("elements are nested more than {MAX_DEPTH} levels deep")]
    TooDeep,
    /// The input ended before all elements were closed.
    #[error("the input ended before element <{0}> was closed")]
    Truncated(String),
}

/// Resolves a named entity: the XML ones and the HTML ones that ENML allows.
pub(crate) fn entity(name: &str) -> Option<&'static str> {
    Some(match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        "ensp" => "\u{2002}",
        "emsp" => "\u{2003}",
        "thinsp" => "\u{2009}",
        "zwnj" => "\u{200c}",
        "zwj" => "\u{200d}",
        "shy" => "\u{ad}",
        "ndash" => "\u{2013}",
        "mdash" => "\u{2014}",
        "lsquo" => "\u{2018}",
        "rsquo" => "\u{2019}",
        "sbquo" => "\u{201a}",
        "ldquo" => "\u{201c}",
        "rdquo" => "\u{201d}",
        "bdquo" => "\u{201e}",
        "laquo" => "\u{ab}",
        "raquo" => "\u{bb}",
        "hellip" => "\u{2026}",
        "bull" => "\u{2022}",
        "middot" => "\u{b7}",
        "copy" => "\u{a9}",
        "reg" => "\u{ae}",
        "trade" => "\u{2122}",
        "deg" => "\u{b0}",
        "plusmn" => "\u{b1}",
        "times" => "\u{d7}",
        "divide" => "\u{f7}",
        "euro" => "\u{20ac}",
        "pound" => "\u{a3}",
        "yen" => "\u{a5}",
        "cent" => "\u{a2}",
        "sect" => "\u{a7}",
        "para" => "\u{b6}",
        "larr" => "\u{2190}",
        "rarr" => "\u{2192}",
        "uarr" => "\u{2191}",
        "darr" => "\u{2193}",
        "check" => "\u{2713}",
        _ => return None,
    })
}

/// Reads the next event, mapping parser errors to [`XmlError`].
pub(crate) fn next_event<'b, R: BufRead>(
    reader: &mut Reader<R>,
    buf: &'b mut Vec<u8>,
) -> Result<Event<'b>, XmlError> {
    let position = reader.buffer_position();
    reader
        .read_event_into(buf)
        .map_err(|error| XmlError::Syntax {
            position,
            message: error.to_string(),
        })
}

/// Converts a start tag into an element without children.
pub(crate) fn element_from_start(
    start: &BytesStart<'_>,
    position: u64,
) -> Result<Element, XmlError> {
    let mut element = Element {
        name: start.name().as_ref().to_owned(),
        ..Element::default()
    };
    for attribute in start.attributes() {
        let attribute = attribute.map_err(|error| XmlError::Syntax {
            position,
            message: error.to_string(),
        })?;
        let value = attribute
            .normalized_value_with(quick_xml::XmlVersion::Implicit1_0, 128, entity)
            .map(|value| value.into_owned())
            // An unknown entity in an attribute keeps the raw value instead of failing.
            .unwrap_or_else(|_| attribute.value.clone().into_owned());
        element
            .attrs
            .push((attribute.key.as_ref().to_owned(), value));
    }
    Ok(element)
}

/// Builds the subtree of `root`, whose start tag has just been read.
///
/// Unknown entities are kept as their literal text (for example `&foo;`) and
/// reported through `on_unknown_entity`, so no character is lost.
pub(crate) fn read_subtree<R: BufRead>(
    reader: &mut Reader<R>,
    root: Element,
    on_unknown_entity: &mut dyn FnMut(&str),
) -> Result<Element, XmlError> {
    let mut stack = vec![root];
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let position = reader.buffer_position();
        let event = next_event(reader, &mut buf)?;
        match event {
            Event::Start(start) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(XmlError::TooDeep);
                }
                stack.push(element_from_start(&start, position)?);
            }
            Event::Empty(start) => {
                let element = element_from_start(&start, position)?;
                push_node(&mut stack, Node::Element(element));
            }
            Event::End(_) => {
                let finished = stack.pop().unwrap_or_default();
                match stack.last_mut() {
                    Some(parent) => parent.children.push(Node::Element(finished)),
                    None => return Ok(finished),
                }
            }
            Event::Text(text) => push_text(&mut stack, &text.xml10_content()),
            Event::CData(data) => push_text(&mut stack, &data.into_inner()),
            Event::GeneralRef(reference) => {
                let resolved = if reference.is_char_ref() {
                    reference
                        .resolve_char_ref()
                        .ok()
                        .flatten()
                        .map(|c| c.to_string())
                } else {
                    entity(&reference.xml10_content()).map(str::to_owned)
                };
                match resolved {
                    Some(text) => push_text(&mut stack, &text),
                    None => {
                        let name = reference.xml10_content().into_owned();
                        on_unknown_entity(&name);
                        push_text(&mut stack, &format!("&{name};"));
                    }
                }
            }
            Event::Eof => {
                let open = stack.last().map(|e| e.name.clone()).unwrap_or_default();
                return Err(XmlError::Truncated(open));
            }
            Event::Comment(_) | Event::Decl(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
}

fn push_node(stack: &mut [Element], node: Node) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    }
}

fn push_text(stack: &mut [Element], text: &str) {
    if text.is_empty() {
        return;
    }
    if let Some(parent) = stack.last_mut() {
        if let Some(Node::Text(previous)) = parent.children.last_mut() {
            previous.push_str(text);
        } else {
            parent.children.push(Node::Text(text.to_owned()));
        }
    }
}

/// Parses a complete XML document held in memory and returns its root element.
pub(crate) fn parse_document(
    xml: &str,
    on_unknown_entity: &mut dyn FnMut(&str),
) -> Result<Element, XmlError> {
    let mut reader = Reader::from_reader(xml.as_bytes());
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let position = reader.buffer_position();
        match next_event(&mut reader, &mut buf)? {
            Event::Start(start) => {
                let root = element_from_start(&start, position)?;
                return read_subtree(&mut reader, root, on_unknown_entity);
            }
            Event::Empty(start) => return element_from_start(&start, position),
            Event::Eof => return Err(XmlError::Truncated(String::new())),
            _ => {}
        }
    }
}
