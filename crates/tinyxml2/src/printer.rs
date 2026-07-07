//! XML output serialization and streaming printer.
//!
//! Exposes [`XmlPrinter`] which provides pretty-printed and compact XML serialization.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::arena::NodeId;
use crate::document::Document;
use crate::node::{Attribute, NodeKind};
use crate::visitor::XmlVisitor;

/// An XML serialization and streaming engine.
///
/// Implements [`XmlVisitor`] to serialize an in-memory [`Document`], and provides
/// a stateful streaming API for manual XML generation without a DOM tree.
pub struct XmlPrinter {
    buffer: String,
    compact: bool,
    depth: usize,
    indent_str: String,
    element_open: bool,
    element_stack: Vec<String>,
    has_children_stack: Vec<bool>,
    write_bom: bool,
}

impl XmlPrinter {
    /// Creates a new pretty-printing printer.
    ///
    /// The default indentation is 4 spaces.
    #[must_use]
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            compact: false,
            depth: 0,
            indent_str: "    ".to_string(),
            element_open: false,
            element_stack: Vec::new(),
            has_children_stack: Vec::new(),
            write_bom: false,
        }
    }

    /// Creates a new compact printer.
    ///
    /// Compact mode outputs XML with zero indentation or newlines between elements.
    #[must_use]
    pub fn new_compact() -> Self {
        Self {
            buffer: String::new(),
            compact: true,
            depth: 0,
            indent_str: String::new(),
            element_open: false,
            element_stack: Vec::new(),
            has_children_stack: Vec::new(),
            write_bom: false,
        }
    }

    /// Creates a new compact printer. Alias for [`Self::new_compact`].
    #[must_use]
    pub fn compact() -> Self {
        Self::new_compact()
    }

    /// Creates a printer with a pre-allocated capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize, compact: bool) -> Self {
        Self {
            buffer: String::with_capacity(capacity),
            compact,
            depth: 0,
            indent_str: if compact {
                String::new()
            } else {
                "    ".to_string()
            },
            element_open: false,
            element_stack: Vec::new(),
            has_children_stack: Vec::new(),
            write_bom: false,
        }
    }

    /// Configures whether to emit the UTF-8 Byte Order Mark (BOM).
    pub fn set_bom(&mut self, use_bom: bool) {
        self.write_bom = use_bom;
    }

    /// Configures the indentation string used in pretty-printed mode.
    pub fn set_indent_str(&mut self, indent_str: &str) {
        self.indent_str = indent_str.to_string();
    }

    /// Clears the printer's state and buffer for reuse.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.depth = 0;
        self.element_open = false;
        self.element_stack.clear();
        self.has_children_stack.clear();
    }

    /// Returns the accumulated XML output as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.buffer
    }

    /// Returns the accumulated XML output as a string slice. Alias for [`Self::as_str`].
    #[must_use]
    pub fn result(&self) -> &str {
        &self.buffer
    }

    /// Consumes the printer and returns the accumulated XML string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.buffer
    }

    /// Returns the current length of the output buffer in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Returns true if the output buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    // --- Streaming API ---

    /// Closes the pending open element tag with `>` if it was left open.
    fn close_pending_open_tag(&mut self) {
        if self.element_open {
            self.buffer.push('>');
            self.element_open = false;
        }
    }

    /// Marks the parent element (top of stack) as having children.
    fn mark_parent_has_children(&mut self) {
        if let Some(top) = self.has_children_stack.last_mut() {
            *top = true;
        }
    }

    /// Writes the XML declaration: `<?xml version="..." encoding="..." standalone="..."?>`
    pub fn push_header(&mut self, version: &str, encoding: Option<&str>, standalone: Option<bool>) {
        self.close_pending_open_tag();
        if !self.compact && !self.buffer.is_empty() {
            self.buffer.push('\n');
        }
        self.buffer.push_str("<?xml version=\"");
        self.buffer.push_str(version);
        self.buffer.push('"');
        if let Some(enc) = encoding {
            self.buffer.push_str(" encoding=\"");
            self.buffer.push_str(enc);
            self.buffer.push('"');
        }
        if let Some(std) = standalone {
            self.buffer.push_str(" standalone=\"");
            self.buffer.push_str(if std { "yes" } else { "no" });
            self.buffer.push('"');
        }
        self.buffer.push_str("?>");
    }

    /// Opens an XML element: `<name` (remains open for attributes).
    pub fn open_element(&mut self, name: &str) {
        self.close_pending_open_tag();
        self.mark_parent_has_children();

        if !self.compact {
            if !self.buffer.is_empty() && !self.buffer.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&self.indent_str.repeat(self.depth));
        }

        self.buffer.push('<');
        self.buffer.push_str(name);
        self.element_open = true;
        self.element_stack.push(name.to_string());
        self.has_children_stack.push(false);

        if !self.compact {
            self.depth += 1;
        }
    }

    /// Pushes an attribute key-value pair to the currently open element.
    ///
    /// # Panics
    ///
    /// Panics if no element is currently open for attributes.
    pub fn push_attribute(&mut self, name: &str, value: &str) {
        assert!(self.element_open, "No element open for attributes");
        self.buffer.push(' ');
        self.buffer.push_str(name);
        self.buffer.push_str("=\"");
        self.buffer
            .push_str(&crate::entity::encode_attribute(value));
        self.buffer.push('"');
    }

    /// Closes the currently open XML element: `/>` or `</name>`.
    pub fn close_element(&mut self) {
        if !self.compact {
            self.depth = self.depth.saturating_sub(1);
        }

        let name = self.element_stack.pop().expect("No element open to close");
        let has_children = self.has_children_stack.pop().unwrap_or(false);

        if self.element_open {
            self.buffer.push_str("/>");
            self.element_open = false;
        } else {
            if !self.compact && has_children {
                if !self.buffer.ends_with('\n') {
                    self.buffer.push('\n');
                }
                self.buffer.push_str(&self.indent_str.repeat(self.depth));
            }
            self.buffer.push_str("</");
            self.buffer.push_str(&name);
            self.buffer.push('>');
        }

        if self.element_stack.is_empty() && !self.compact {
            self.buffer.push('\n');
        }

        if !self.has_children_stack.is_empty() {
            self.mark_parent_has_children();
        }
    }

    /// Pushes entity-escaped text content.
    pub fn push_text(&mut self, text: &str) {
        self.close_pending_open_tag();
        if !self.compact && !self.buffer.is_empty() {
            if let Some(&true) = self.has_children_stack.last() {
                if !self.buffer.ends_with('\n') {
                    self.buffer.push('\n');
                }
                self.buffer.push_str(&self.indent_str.repeat(self.depth));
            }
        }
        self.buffer.push_str(&crate::entity::encode_text(text));
    }

    /// Pushes unescaped raw text content.
    pub fn push_text_raw(&mut self, text: &str) {
        self.close_pending_open_tag();
        if !self.compact && !self.buffer.is_empty() {
            if let Some(&true) = self.has_children_stack.last() {
                if !self.buffer.ends_with('\n') {
                    self.buffer.push('\n');
                }
                self.buffer.push_str(&self.indent_str.repeat(self.depth));
            }
        }
        self.buffer.push_str(text);
    }

    /// Pushes a CDATA section: `<![CDATA[text]]>`.
    pub fn push_cdata(&mut self, text: &str) {
        self.close_pending_open_tag();
        if !self.compact && !self.buffer.is_empty() {
            if let Some(&true) = self.has_children_stack.last() {
                if !self.buffer.ends_with('\n') {
                    self.buffer.push('\n');
                }
                self.buffer.push_str(&self.indent_str.repeat(self.depth));
            }
        }
        self.buffer.push_str("<![CDATA[");
        self.buffer.push_str(text);
        self.buffer.push_str("]]>");
    }

    /// Pushes a comment: `<!--text-->`.
    pub fn push_comment(&mut self, text: &str) {
        self.close_pending_open_tag();
        self.mark_parent_has_children();
        if !self.compact && !self.buffer.is_empty() {
            if !self.buffer.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&self.indent_str.repeat(self.depth));
        }
        self.buffer.push_str("<!--");
        self.buffer.push_str(text);
        self.buffer.push_str("-->");
    }

    /// Pushes a declaration block: `<?text?>`.
    pub fn push_declaration(&mut self, text: &str) {
        self.close_pending_open_tag();
        self.mark_parent_has_children();
        if !self.compact && !self.buffer.is_empty() {
            if !self.buffer.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&self.indent_str.repeat(self.depth));
        }
        self.buffer.push_str("<?");
        self.buffer.push_str(text);
        self.buffer.push_str("?>");
    }

    /// Pushes an unknown block: `<!text>`.
    pub fn push_unknown(&mut self, text: &str) {
        self.close_pending_open_tag();
        self.mark_parent_has_children();
        if !self.compact && !self.buffer.is_empty() {
            if !self.buffer.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&self.indent_str.repeat(self.depth));
        }
        self.buffer.push_str("<!");
        self.buffer.push_str(text);
        self.buffer.push('>');
    }

    /// Pushes a declaration node with attributes: `<?name attr="value"?>`.
    pub fn push_declaration_node(&mut self, name: &str, attrs: &[Attribute]) {
        self.close_pending_open_tag();
        self.mark_parent_has_children();
        if !self.compact && !self.buffer.is_empty() {
            if !self.buffer.ends_with('\n') {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&self.indent_str.repeat(self.depth));
        }
        self.buffer.push_str("<?");
        self.buffer.push_str(name);
        for attr in attrs {
            self.buffer.push(' ');
            self.buffer.push_str(&attr.name);
            self.buffer.push_str("=\"");
            self.buffer
                .push_str(&crate::entity::encode_attribute(&attr.value));
            self.buffer.push('"');
        }
        self.buffer.push_str("?>");
    }

    /// Helper to look ahead in DOM and determine if element contains child elements,
    /// comments, declarations, or unknowns.
    fn is_complex_element(doc: &Document, element: NodeId) -> bool {
        let mut child = doc.first_child(element);
        while let Some(c) = child {
            if let Some(
                NodeKind::Element(_)
                | NodeKind::Comment(_)
                | NodeKind::Declaration(_)
                | NodeKind::Unknown(_),
            ) = doc.node_kind(c)
            {
                return true;
            }
            child = doc.next_sibling(c);
        }
        false
    }
}

impl Default for XmlPrinter {
    fn default() -> Self {
        Self::new()
    }
}

impl XmlVisitor for XmlPrinter {
    fn visit_enter_document(&mut self, doc: &Document) -> bool {
        if self.write_bom && doc.has_bom() {
            self.buffer.push('\u{FEFF}');
        }
        true
    }

    fn visit_exit_document(&mut self, _doc: &Document) -> bool {
        if !self.compact && !self.buffer.is_empty() && !self.buffer.ends_with('\n') {
            self.buffer.push('\n');
        }
        true
    }

    fn visit_enter_element(&mut self, doc: &Document, element: NodeId) -> bool {
        if let Some(NodeKind::Element(el_data)) = doc.node_kind(element) {
            let is_complex = Self::is_complex_element(doc, element);
            self.open_element(&el_data.name);

            // If complex element, mark has_children as true immediately
            // so nested text/elements are formatted on new indented lines
            if is_complex {
                if let Some(top) = self.has_children_stack.last_mut() {
                    *top = true;
                }
            }

            for attr in &el_data.attributes {
                self.push_attribute(&attr.name, &attr.value);
            }
        }
        true
    }

    fn visit_exit_element(&mut self, _doc: &Document, _element: NodeId) -> bool {
        self.close_element();
        true
    }

    fn visit_text(&mut self, doc: &Document, text: NodeId) -> bool {
        if let Some(NodeKind::Text(text_data)) = doc.node_kind(text) {
            if text_data.is_cdata {
                self.push_cdata(&text_data.content);
            } else {
                self.push_text(&text_data.content);
            }
        }
        true
    }

    fn visit_comment(&mut self, doc: &Document, comment: NodeId) -> bool {
        if let Some(NodeKind::Comment(text)) = doc.node_kind(comment) {
            self.push_comment(text);
        }
        true
    }

    fn visit_declaration(&mut self, doc: &Document, declaration: NodeId) -> bool {
        if let Some(NodeKind::Declaration(el_data)) = doc.node_kind(declaration) {
            let mut attrs = Vec::new();
            for attr in doc.iterate_attributes(declaration) {
                attrs.push(attr.clone());
            }
            self.push_declaration_node(&el_data.name, &attrs);
        }
        true
    }

    fn visit_unknown(&mut self, doc: &Document, unknown: NodeId) -> bool {
        if let Some(NodeKind::Unknown(text)) = doc.node_kind(unknown) {
            self.push_unknown(text);
        }
        true
    }
}
