//! Hand-written recursive-descent XML parser.
//!
//! Operates on a validated UTF-8 string input and constructs DOM nodes
//! directly into the `Document`'s node arena.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::arena::NodeId;
use crate::document::Document;
use crate::error::{ParseErrorKind, Result, XmlError};
use crate::node::{Attribute, ElementData, NodeData, NodeKind, TextData};
use crate::{ParseOptions, Whitespace};

/// The internal XML parser state.
pub(crate) struct Parser<'a> {
    input: &'a str,
    pos: usize,
    line: u32,
    options: ParseOptions,
}

impl<'a> Parser<'a> {
    /// Creates a new Parser instance.
    pub(crate) fn new(input: &'a str, options: ParseOptions) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            options,
        }
    }

    /// Returns the remaining portion of the input string.
    #[inline]
    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    /// Returns whether the end of the input string has been reached.
    #[inline]
    fn is_eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Peeks at the next character in the input without consuming it.
    #[inline]
    fn peek(&self) -> Option<char> {
        self.remaining().chars().next()
    }

    /// Advances the parser position by a single character.
    fn advance_char(&mut self) -> Option<char> {
        let ch = self.peek()?;
        if ch == '\n' {
            self.line += 1;
        }
        self.pos += ch.len_utf8();
        Some(ch)
    }

    /// Advances the parser position if the remaining input starts with the prefix.
    fn advance_str(&mut self, prefix: &str) -> bool {
        if self.remaining().starts_with(prefix) {
            for ch in prefix.chars() {
                if ch == '\n' {
                    self.line += 1;
                }
            }
            self.pos += prefix.len();
            true
        } else {
            false
        }
    }

    /// Skips leading XML whitespace.
    fn skip_whitespace(&mut self) {
        let (rest, newlines) = crate::util::skip_whitespace(self.remaining());
        self.line += newlines;
        self.pos = self.input.len() - rest.len();
    }

    /// Parses the entire input into the given document.
    pub(crate) fn parse_document(&mut self, doc: &mut Document) -> Result<()> {
        let root_id = doc.root();
        let mut root_parsed = false;

        self.skip_whitespace();

        while !self.is_eof() {
            self.skip_whitespace();
            if self.is_eof() {
                break;
            }

            if self.remaining().starts_with('<') {
                if self.remaining().starts_with("<?") {
                    let node = self.parse_declaration(doc)?;
                    doc.insert_end_child(root_id, node)?;
                } else if self.remaining().starts_with("<!--") {
                    let node = self.parse_comment(doc)?;
                    doc.insert_end_child(root_id, node)?;
                } else if self.remaining().starts_with("<!") {
                    let node = self.parse_unknown(doc)?;
                    doc.insert_end_child(root_id, node)?;
                } else if self.remaining().starts_with("</") {
                    return Err(XmlError::Parse {
                        kind: ParseErrorKind::Element,
                        line: self.line,
                        message: Some("Stray closing tag outside of element".to_string()),
                    });
                } else {
                    if root_parsed {
                        return Err(XmlError::Parse {
                            kind: ParseErrorKind::Element,
                            line: self.line,
                            message: Some("Multiple root elements found".to_string()),
                        });
                    }
                    let node = self.parse_element(doc, 1)?;
                    doc.insert_end_child(root_id, node)?;
                    root_parsed = true;
                }
            } else {
                return Err(XmlError::Parse {
                    kind: ParseErrorKind::Text,
                    line: self.line,
                    message: Some("Text content not allowed outside of root element".to_string()),
                });
            }
        }

        if !root_parsed {
            return Err(XmlError::EmptyDocument);
        }

        Ok(())
    }

    /// Parses an XML Element recursively.
    #[allow(clippy::too_many_lines)]
    fn parse_element(&mut self, doc: &mut Document, depth: u32) -> Result<NodeId> {
        if depth > self.options.max_depth {
            return Err(XmlError::ElementDepthExceeded {
                line: self.line,
                max_depth: self.options.max_depth,
            });
        }

        if !self.advance_str("<") {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Element,
                line: self.line,
                message: Some("Expected '<' at start of element".to_string()),
            });
        }

        let name_line = self.line;
        let (name, _) =
            crate::util::read_name(self.remaining()).ok_or_else(|| XmlError::Parse {
                kind: ParseErrorKind::Element,
                line: name_line,
                message: Some("Invalid element name".to_string()),
            })?;
        let name = name.to_string();
        self.advance_str(&name);

        let mut attributes = Vec::new();

        loop {
            self.skip_whitespace();

            if self.remaining().starts_with("/>") {
                self.advance_str("/>");
                let el_id = doc.arena.alloc(NodeData::new(
                    NodeKind::Element(ElementData { name, attributes }),
                    name_line,
                ));
                return Ok(el_id);
            }

            if self.remaining().starts_with('>') {
                self.advance_str(">");
                break;
            }

            // Parse attribute
            let attr_line = self.line;
            let (attr_name, _) =
                crate::util::read_name(self.remaining()).ok_or_else(|| XmlError::Parse {
                    kind: ParseErrorKind::Attribute,
                    line: attr_line,
                    message: Some("Invalid attribute name".to_string()),
                })?;
            let attr_name = attr_name.to_string();

            if attributes.iter().any(|a: &Attribute| a.name == attr_name) {
                return Err(XmlError::Parse {
                    kind: ParseErrorKind::Attribute,
                    line: attr_line,
                    message: Some(format!("Duplicate attribute name: '{attr_name}'")),
                });
            }

            self.advance_str(&attr_name);
            self.skip_whitespace();

            if !self.advance_str("=") {
                return Err(XmlError::Parse {
                    kind: ParseErrorKind::Attribute,
                    line: self.line,
                    message: Some(format!("Expected '=' after attribute '{attr_name}'")),
                });
            }

            self.skip_whitespace();

            let quote = self.peek().ok_or_else(|| XmlError::Parse {
                kind: ParseErrorKind::Attribute,
                line: self.line,
                message: Some("Unexpected EOF in attribute value".to_string()),
            })?;

            if quote != '"' && quote != '\'' {
                return Err(XmlError::Parse {
                    kind: ParseErrorKind::Attribute,
                    line: self.line,
                    message: Some("Attribute value must be enclosed in quotes".to_string()),
                });
            }

            self.advance_char(); // consume opening quote

            let val_start = self.pos;
            let mut val_end = self.pos;
            let mut closed = false;

            while let Some(ch) = self.peek() {
                if ch == quote {
                    closed = true;
                    break;
                }
                self.advance_char();
                val_end = self.pos;
            }

            if !closed {
                return Err(XmlError::Parse {
                    kind: ParseErrorKind::Attribute,
                    line: self.line,
                    message: Some("Unclosed attribute value".to_string()),
                });
            }

            self.advance_char(); // consume closing quote

            let raw_val = &self.input[val_start..val_end];
            let value = if self.options.process_entities {
                crate::entity::decode(raw_val)
            } else {
                crate::entity::decode_numeric_only(raw_val)
            };

            attributes.push(Attribute {
                name: attr_name,
                value,
            });
        }

        let el_id = doc.arena.alloc(NodeData::new(
            NodeKind::Element(ElementData {
                name: name.clone(),
                attributes,
            }),
            name_line,
        ));

        while !self.is_eof() {
            if self.remaining().starts_with("</") {
                self.advance_str("</");
                let close_line = self.line;
                let (close_name, _) =
                    crate::util::read_name(self.remaining()).ok_or_else(|| XmlError::Parse {
                        kind: ParseErrorKind::Element,
                        line: close_line,
                        message: Some("Invalid closing tag name".to_string()),
                    })?;
                let close_name = close_name.to_string();
                self.advance_str(&close_name);
                self.skip_whitespace();

                if !self.advance_str(">") {
                    return Err(XmlError::Parse {
                        kind: ParseErrorKind::Element,
                        line: self.line,
                        message: Some("Expected '>' to close tag".to_string()),
                    });
                }

                if close_name != name {
                    return Err(XmlError::MismatchedElement {
                        line: close_line,
                        expected: name,
                        found: close_name,
                    });
                }

                return Ok(el_id);
            }

            if self.remaining().starts_with('<') {
                if self.remaining().starts_with("<?") {
                    let child = self.parse_declaration(doc)?;
                    doc.insert_end_child(el_id, child)?;
                } else if self.remaining().starts_with("<!--") {
                    let child = self.parse_comment(doc)?;
                    doc.insert_end_child(el_id, child)?;
                } else if self.remaining().starts_with("<![CDATA[") {
                    let child = self.parse_cdata(doc)?;
                    doc.insert_end_child(el_id, child)?;
                } else if self.remaining().starts_with("<!") {
                    let child = self.parse_unknown(doc)?;
                    doc.insert_end_child(el_id, child)?;
                } else {
                    let child = self.parse_element(doc, depth + 1)?;
                    doc.insert_end_child(el_id, child)?;
                }
            } else {
                let child = self.parse_text(doc, el_id)?;
                if let Some(child_id) = child {
                    doc.insert_end_child(el_id, child_id)?;
                }
            }
        }

        Err(XmlError::Parse {
            kind: ParseErrorKind::Element,
            line: self.line,
            message: Some(format!("Unclosed element: '{name}'")),
        })
    }

    /// Parses text content between element tags.
    #[allow(clippy::unnecessary_wraps)]
    fn parse_text(&mut self, doc: &mut Document, parent: NodeId) -> Result<Option<NodeId>> {
        let start_line = self.line;
        let start_pos = self.pos;

        while let Some(ch) = self.peek() {
            if ch == '<' {
                break;
            }
            self.advance_char();
        }

        let raw_text = &self.input[start_pos..self.pos];
        if raw_text.is_empty() {
            return Ok(None);
        }

        let is_all_ws = raw_text.chars().all(crate::util::is_whitespace);

        if is_all_ws {
            let keep_ws = self.options.whitespace == Whitespace::Pedantic
                && doc.first_child(parent).is_none()
                && self.remaining().starts_with("</");
            if !keep_ws {
                return Ok(None);
            }
        }

        let mut processed = if self.options.process_entities {
            crate::entity::decode(raw_text)
        } else {
            crate::entity::decode_numeric_only(raw_text)
        };

        processed = match self.options.whitespace {
            Whitespace::Preserve => processed,
            Whitespace::Pedantic => {
                let mut normalized = String::with_capacity(processed.len());
                let mut chars = processed.chars().peekable();
                while let Some(ch) = chars.next() {
                    if ch == '\r' {
                        if chars.peek() == Some(&'\n') {
                            chars.next();
                        }
                        normalized.push('\n');
                    } else {
                        normalized.push(ch);
                    }
                }
                normalized
            }
            Whitespace::Collapse => crate::util::collapse_whitespace(&processed),
        };

        if processed.is_empty() && self.options.whitespace != Whitespace::Pedantic {
            return Ok(None);
        }

        let txt_id = doc.arena.alloc(NodeData::new(
            NodeKind::Text(TextData {
                content: processed,
                is_cdata: false,
            }),
            start_line,
        ));
        Ok(Some(txt_id))
    }

    /// Parses a CDATA section.
    fn parse_cdata(&mut self, doc: &mut Document) -> Result<NodeId> {
        let start_line = self.line;
        if !self.advance_str("<![CDATA[") {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Cdata,
                line: self.line,
                message: Some("Expected '<![CDATA['".to_string()),
            });
        }

        let content_start = self.pos;
        let mut content_end = self.pos;
        let mut closed = false;

        while !self.is_eof() {
            if self.remaining().starts_with("]]>") {
                content_end = self.pos;
                self.advance_str("]]>");
                closed = true;
                break;
            }
            self.advance_char();
        }

        if !closed {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Cdata,
                line: self.line,
                message: Some("Unclosed CDATA section".to_string()),
            });
        }

        let content = self.input[content_start..content_end].to_string();
        let node = doc.arena.alloc(NodeData::new(
            NodeKind::Text(TextData {
                content,
                is_cdata: true,
            }),
            start_line,
        ));
        Ok(node)
    }

    /// Parses an XML comment.
    fn parse_comment(&mut self, doc: &mut Document) -> Result<NodeId> {
        let start_line = self.line;
        if !self.advance_str("<!--") {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Comment,
                line: self.line,
                message: Some("Expected '<!--'".to_string()),
            });
        }

        let content_start = self.pos;
        let mut content_end = self.pos;
        let mut closed = false;

        while !self.is_eof() {
            if self.remaining().starts_with("-->") {
                content_end = self.pos;
                self.advance_str("-->");
                closed = true;
                break;
            }
            self.advance_char();
        }

        if !closed {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Comment,
                line: self.line,
                message: Some("Unclosed comment".to_string()),
            });
        }

        let content = self.input[content_start..content_end].to_string();
        let node = doc
            .arena
            .alloc(NodeData::new(NodeKind::Comment(content), start_line));
        Ok(node)
    }

    /// Parses an XML declaration or processing instruction.
    fn parse_declaration(&mut self, doc: &mut Document) -> Result<NodeId> {
        let start_line = self.line;
        if !self.advance_str("<?") {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Declaration,
                line: self.line,
                message: Some("Expected '<?'".to_string()),
            });
        }

        let content_start = self.pos;
        let mut content_end = self.pos;
        let mut closed = false;

        while !self.is_eof() {
            if self.remaining().starts_with("?>") {
                content_end = self.pos;
                self.advance_str("?>");
                closed = true;
                break;
            }
            if self.peek() == Some('<') {
                break;
            }
            self.advance_char();
        }

        if !closed {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Declaration,
                line: self.line,
                message: Some("Unclosed declaration".to_string()),
            });
        }

        let content = self.input[content_start..content_end].to_string();
        let node = doc.arena.alloc(NodeData::new(
            NodeKind::Declaration(ElementData {
                name: content,
                attributes: Vec::new(),
            }),
            start_line,
        ));
        Ok(node)
    }

    /// Parses an unknown XML node (like DOCTYPE).
    fn parse_unknown(&mut self, doc: &mut Document) -> Result<NodeId> {
        let start_line = self.line;
        if !self.advance_str("<!") {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Unknown,
                line: self.line,
                message: Some("Expected '<!'".to_string()),
            });
        }

        let content_start = self.pos;
        let mut content_end = self.pos;
        let mut closed = false;

        while !self.is_eof() {
            if self.remaining().starts_with('>') {
                content_end = self.pos;
                self.advance_str(">");
                closed = true;
                break;
            }
            if self.peek() == Some('<') {
                break;
            }
            self.advance_char();
        }

        if !closed {
            return Err(XmlError::Parse {
                kind: ParseErrorKind::Unknown,
                line: self.line,
                message: Some("Unclosed unknown node".to_string()),
            });
        }

        let content = self.input[content_start..content_end].to_string();
        let node = doc
            .arena
            .alloc(NodeData::new(NodeKind::Unknown(content), start_line));
        Ok(node)
    }
}
