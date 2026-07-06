//! XML document container and DOM tree manipulation.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::{fmt, str};

use crate::ParseOptions;
use crate::arena::{Arena, NodeId};
use crate::error::{Result, XmlError};
use crate::node::{Attribute, ElementData, NodeData, NodeKind, TextData};
use crate::parser::Parser;

/// The main XML document container.
///
/// Owns the node arena, root node, and tracks the current parse state/error.
#[derive(Debug)]
pub struct Document {
    pub(crate) arena: Arena<NodeData>,
    root: NodeId,
    error: Option<XmlError>,
    options: ParseOptions,
    has_bom: bool,
}

impl Document {
    /// Creates a new, empty XML document containing only a root Document node.
    #[must_use]
    pub fn new() -> Self {
        let mut arena = Arena::new();
        let root = arena.alloc(NodeData::new(NodeKind::Document, 1));
        Self {
            arena,
            root,
            error: None,
            options: ParseOptions::default(),
            has_bom: false,
        }
    }

    /// Creates a new, empty XML document with custom parse options.
    #[must_use]
    pub fn with_options(options: ParseOptions) -> Self {
        let mut arena = Arena::new();
        let root = arena.alloc(NodeData::new(NodeKind::Document, 1));
        Self {
            arena,
            root,
            error: None,
            options,
            has_bom: false,
        }
    }

    /// Returns the root `NodeId` of the document.
    #[must_use]
    pub const fn root(&self) -> NodeId {
        self.root
    }

    /// Returns the kind of the specified node, if it exists.
    #[must_use]
    pub fn node_kind(&self, node: NodeId) -> Option<&NodeKind> {
        self.arena.get(node).map(|d| &d.kind)
    }

    /// Returns the 1-based source line number where this node was parsed.
    #[must_use]
    pub fn line_num(&self, node: NodeId) -> Option<u32> {
        self.arena.get(node).map(|d| d.line_num)
    }

    /// Returns the current error state of the document, if any.
    #[must_use]
    pub fn error(&self) -> Option<XmlError> {
        self.error.clone()
    }

    /// Returns the line number of the last error, if available.
    #[must_use]
    pub fn error_line(&self) -> Option<u32> {
        self.error.as_ref().and_then(XmlError::line)
    }

    /// Sets the error state of the document.
    pub(crate) fn set_error(&mut self, err: XmlError) {
        self.error = Some(err);
    }

    /// Resets the document to an empty state, invalidating all existing `NodeId`s.
    pub fn clear(&mut self) {
        self.arena.clear();
        self.error = None;
        self.has_bom = false;
        self.root = self.arena.alloc(NodeData::new(NodeKind::Document, 1));
    }

    /// Returns whether a Byte Order Mark (BOM) was detected during parsing.
    #[must_use]
    pub const fn has_bom(&self) -> bool {
        self.has_bom
    }

    /// Sets whether to output a Byte Order Mark (BOM) when serializing.
    pub fn set_bom(&mut self, use_bom: bool) {
        self.has_bom = use_bom;
    }

    /// Returns a reference to the document's parse options.
    #[must_use]
    pub const fn options(&self) -> &ParseOptions {
        &self.options
    }

    /// Returns a mutable reference to the document's parse options.
    pub fn options_mut(&mut self) -> &mut ParseOptions {
        &mut self.options
    }

    // --- Parsing Entry Points ---

    /// Parses an XML document from a string slice in place.
    ///
    /// Any existing DOM structure is cleared.
    pub fn parse_str(&mut self, xml: &str) -> Result<()> {
        self.clear();

        // Truncate at first null byte to match C++ null-terminated behavior
        let truncated_xml = xml.split('\0').next().unwrap_or("");

        // Detect and skip BOM
        let (xml_after_bom, had_bom) = crate::util::strip_bom(truncated_xml);
        self.has_bom = had_bom;

        let mut parser = Parser::new(xml_after_bom, self.options.clone());
        match parser.parse_document(self) {
            Ok(()) => Ok(()),
            Err(e) => {
                self.set_error(e.clone());
                Err(e)
            }
        }
    }

    /// Parses an XML document from a byte slice in place.
    ///
    /// Rejects non-UTF-8 inputs. Any existing DOM structure is cleared.
    pub fn parse_bytes_mut(&mut self, bytes: &[u8]) -> Result<()> {
        let s = str::from_utf8(bytes).map_err(|e| {
            let err = XmlError::Parse {
                kind: crate::error::ParseErrorKind::General,
                line: 1,
                message: Some(format!("Invalid UTF-8 sequence: {e}")),
            };
            self.set_error(err.clone());
            err
        })?;
        self.parse_str(s)
    }

    /// Loads and parses an XML file from the given path in place.
    ///
    /// Any existing DOM structure is cleared.
    #[cfg(feature = "std")]
    pub fn load_file_mut(&mut self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let bytes = std::fs::read(path)?;
        self.parse_bytes_mut(&bytes)
    }

    /// Parses an XML document from a string slice, returning the new Document.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse(r#"<book isbn="9780441172719"><title>Dune</title></book>"#)?;
    /// let book = doc.root_element().expect("book element");
    ///
    /// assert_eq!(doc.attribute(book, "isbn"), Some("9780441172719"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn parse(xml: &str) -> Result<Self> {
        let mut doc = Self::new();
        doc.parse_str(xml)?;
        Ok(doc)
    }

    /// Parses an XML document from a byte slice, returning the new Document.
    ///
    /// Rejects non-UTF-8 inputs.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self> {
        let mut doc = Self::new();
        doc.parse_bytes_mut(bytes)?;
        Ok(doc)
    }

    /// Loads and parses an XML file from the given path, returning the new Document.
    #[cfg(feature = "std")]
    pub fn load_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let mut doc = Self::new();
        doc.load_file_mut(path)?;
        Ok(doc)
    }

    // --- Factory Methods ---

    /// Creates a new detached Element node in the document's arena.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let mut doc = Document::new();
    /// let library = doc.new_element("library");
    /// doc.set_attribute(library, "name", "Central")?;
    /// doc.insert_end_child(doc.root(), library)?;
    ///
    /// assert_eq!(doc.root_element(), Some(library));
    /// assert_eq!(doc.attribute(library, "name"), Some("Central"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn new_element(&mut self, name: &str) -> NodeId {
        let kind = NodeKind::Element(ElementData {
            name: name.to_string(),
            attributes: Vec::new(),
        });
        self.arena.alloc(NodeData::new(kind, 1))
    }

    /// Creates a new detached Text node in the document's arena.
    pub fn new_text(&mut self, text: &str) -> NodeId {
        let kind = NodeKind::Text(TextData {
            content: text.to_string(),
            is_cdata: false,
        });
        self.arena.alloc(NodeData::new(kind, 1))
    }

    /// Creates a new detached CDATA Text node in the document's arena.
    pub fn new_cdata(&mut self, text: &str) -> NodeId {
        let kind = NodeKind::Text(TextData {
            content: text.to_string(),
            is_cdata: true,
        });
        self.arena.alloc(NodeData::new(kind, 1))
    }

    /// Creates a new detached Comment node in the document's arena.
    pub fn new_comment(&mut self, text: &str) -> NodeId {
        let kind = NodeKind::Comment(text.to_string());
        self.arena.alloc(NodeData::new(kind, 1))
    }

    /// Creates a new detached Declaration node in the document's arena.
    pub fn new_declaration(&mut self, decl: &str) -> NodeId {
        let kind = NodeKind::Declaration(ElementData {
            name: decl.to_string(),
            attributes: Vec::new(),
        });
        self.arena.alloc(NodeData::new(kind, 1))
    }

    /// Creates a new detached Unknown node in the document's arena.
    pub fn new_unknown(&mut self, text: &str) -> NodeId {
        let kind = NodeKind::Unknown(text.to_string());
        self.arena.alloc(NodeData::new(kind, 1))
    }

    // --- Navigation APIs ---

    /// Returns the parent of the specified node, if it exists.
    #[must_use]
    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node).and_then(|d| d.parent)
    }

    /// Returns the first child of the specified node, if it exists.
    #[must_use]
    pub fn first_child(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node).and_then(|d| d.first_child)
    }

    /// Returns the last child of the specified node, if it exists.
    #[must_use]
    pub fn last_child(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node).and_then(|d| d.last_child)
    }

    /// Returns the previous sibling of the specified node, if it exists.
    #[must_use]
    pub fn prev_sibling(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node).and_then(|d| d.prev_sibling)
    }

    /// Returns the next sibling of the specified node, if it exists.
    #[must_use]
    pub fn next_sibling(&self, node: NodeId) -> Option<NodeId> {
        self.arena.get(node).and_then(|d| d.next_sibling)
    }

    /// Returns the first child Element of the specified node, optionally matching a tag name.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<feed><title>News</title><entry id=\"1\"/></feed>")?;
    /// let feed = doc.root_element().expect("feed element");
    /// let entry = doc.first_child_element(feed, Some("entry")).expect("entry element");
    ///
    /// assert_eq!(doc.attribute(entry, "id"), Some("1"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    #[must_use]
    pub fn first_child_element(&self, node: NodeId, name: Option<&str>) -> Option<NodeId> {
        let mut current = self.first_child(node);
        while let Some(curr) = current {
            if let Some(data) = self.arena.get(curr) {
                if let NodeKind::Element(el_data) = &data.kind {
                    if name.is_none_or(|n| el_data.name == n) {
                        return Some(curr);
                    }
                }
            }
            current = self.next_sibling(curr);
        }
        None
    }

    /// Returns the last child Element of the specified node, optionally matching a tag name.
    #[must_use]
    pub fn last_child_element(&self, node: NodeId, name: Option<&str>) -> Option<NodeId> {
        let mut current = self.last_child(node);
        while let Some(curr) = current {
            if let Some(data) = self.arena.get(curr) {
                if let NodeKind::Element(el_data) = &data.kind {
                    if name.is_none_or(|n| el_data.name == n) {
                        return Some(curr);
                    }
                }
            }
            current = self.prev_sibling(curr);
        }
        None
    }

    /// Returns the next sibling Element of the specified node, optionally matching a tag name.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<playlist><track/><note>skip me</note><track id=\"next\"/></playlist>")?;
    /// let playlist = doc.root_element().expect("playlist element");
    /// let first_track = doc.first_child_element(playlist, Some("track")).expect("first track");
    /// let next_track = doc.next_sibling_element(first_track, Some("track")).expect("next track");
    ///
    /// assert_eq!(doc.attribute(next_track, "id"), Some("next"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    #[must_use]
    pub fn next_sibling_element(&self, node: NodeId, name: Option<&str>) -> Option<NodeId> {
        let mut current = self.next_sibling(node);
        while let Some(curr) = current {
            if let Some(data) = self.arena.get(curr) {
                if let NodeKind::Element(el_data) = &data.kind {
                    if name.is_none_or(|n| el_data.name == n) {
                        return Some(curr);
                    }
                }
            }
            current = self.next_sibling(curr);
        }
        None
    }

    /// Returns the previous sibling Element of the specified node, optionally matching a tag name.
    #[must_use]
    pub fn prev_sibling_element(&self, node: NodeId, name: Option<&str>) -> Option<NodeId> {
        let mut current = self.prev_sibling(node);
        while let Some(curr) = current {
            if let Some(data) = self.arena.get(curr) {
                if let NodeKind::Element(el_data) = &data.kind {
                    if name.is_none_or(|n| el_data.name == n) {
                        return Some(curr);
                    }
                }
            }
            current = self.prev_sibling(curr);
        }
        None
    }

    /// Returns the root Element of the document (the first element child of the root document node).
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<config><theme>dark</theme></config>")?;
    /// let root = doc.root_element().expect("root element");
    ///
    /// assert_eq!(doc.element_ref(root).expect("element").name(), "config");
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    #[must_use]
    pub fn root_element(&self) -> Option<NodeId> {
        self.first_child_element(self.root, None)
    }

    // --- Invariant Checks & Tree Linkage Helpers ---

    /// Returns `true` if `ancestor` is an ancestor of `descendant` (or the same node).
    fn is_ancestor(&self, ancestor: NodeId, mut descendant: NodeId) -> bool {
        if ancestor == descendant {
            return true;
        }
        while let Some(parent) = self.parent(descendant) {
            if parent == ancestor {
                return true;
            }
            descendant = parent;
        }
        false
    }

    /// Unlinks a node from its parent and siblings.
    fn unlink(&mut self, node: NodeId) -> Result<()> {
        let data = self.arena.get(node).ok_or(XmlError::InvalidNodeId)?.clone();
        if let Some(parent) = data.parent {
            let p_data = self.arena.get_mut(parent).ok_or(XmlError::InvalidNodeId)?;
            if p_data.first_child == Some(node) {
                p_data.first_child = data.next_sibling;
            }
            if p_data.last_child == Some(node) {
                p_data.last_child = data.prev_sibling;
            }
        }
        if let Some(prev) = data.prev_sibling {
            if let Some(prev_node) = self.arena.get_mut(prev) {
                prev_node.next_sibling = data.next_sibling;
            }
        }
        if let Some(next) = data.next_sibling {
            if let Some(next_node) = self.arena.get_mut(next) {
                next_node.prev_sibling = data.prev_sibling;
            }
        }

        let node_mut = self.arena.get_mut(node).ok_or(XmlError::InvalidNodeId)?;
        node_mut.parent = None;
        node_mut.prev_sibling = None;
        node_mut.next_sibling = None;
        Ok(())
    }

    // --- Tree Mutation APIs ---

    /// Inserts `child` as the last child of `parent`.
    pub fn insert_end_child(&mut self, parent: NodeId, child: NodeId) -> Result<NodeId> {
        if !self.arena.contains(parent) || !self.arena.contains(child) {
            return Err(XmlError::InvalidNodeId);
        }
        if self.is_ancestor(child, parent) {
            return Err(XmlError::InvalidNodeId);
        }

        self.unlink(child)?;

        let parent_data = self.arena.get(parent).ok_or(XmlError::InvalidNodeId)?;
        let old_last = parent_data.last_child;

        if let Some(last) = old_last {
            let last_node = self.arena.get_mut(last).ok_or(XmlError::InvalidNodeId)?;
            last_node.next_sibling = Some(child);
        }

        let child_node = self.arena.get_mut(child).ok_or(XmlError::InvalidNodeId)?;
        child_node.parent = Some(parent);
        child_node.prev_sibling = old_last;
        child_node.next_sibling = None;

        let parent_node = self.arena.get_mut(parent).ok_or(XmlError::InvalidNodeId)?;
        if parent_node.first_child.is_none() {
            parent_node.first_child = Some(child);
        }
        parent_node.last_child = Some(child);

        Ok(child)
    }

    /// Inserts `child` as the first child of `parent`.
    pub fn insert_first_child(&mut self, parent: NodeId, child: NodeId) -> Result<NodeId> {
        if !self.arena.contains(parent) || !self.arena.contains(child) {
            return Err(XmlError::InvalidNodeId);
        }
        if self.is_ancestor(child, parent) {
            return Err(XmlError::InvalidNodeId);
        }

        self.unlink(child)?;

        let parent_data = self.arena.get(parent).ok_or(XmlError::InvalidNodeId)?;
        let old_first = parent_data.first_child;

        if let Some(first) = old_first {
            let first_node = self.arena.get_mut(first).ok_or(XmlError::InvalidNodeId)?;
            first_node.prev_sibling = Some(child);
        }

        let child_node = self.arena.get_mut(child).ok_or(XmlError::InvalidNodeId)?;
        child_node.parent = Some(parent);
        child_node.prev_sibling = None;
        child_node.next_sibling = old_first;

        let parent_node = self.arena.get_mut(parent).ok_or(XmlError::InvalidNodeId)?;
        if parent_node.last_child.is_none() {
            parent_node.last_child = Some(child);
        }
        parent_node.first_child = Some(child);

        Ok(child)
    }

    /// Inserts `child` immediately after `after`.
    pub fn insert_after_child(&mut self, after: NodeId, child: NodeId) -> Result<NodeId> {
        if !self.arena.contains(after) || !self.arena.contains(child) {
            return Err(XmlError::InvalidNodeId);
        }
        let parent = self.parent(after).ok_or(XmlError::InvalidNodeId)?;
        if self.is_ancestor(child, parent) {
            return Err(XmlError::InvalidNodeId);
        }

        self.unlink(child)?;

        let after_data = self.arena.get(after).ok_or(XmlError::InvalidNodeId)?;
        let old_next = after_data.next_sibling;

        if let Some(next) = old_next {
            let next_node = self.arena.get_mut(next).ok_or(XmlError::InvalidNodeId)?;
            next_node.prev_sibling = Some(child);
        }

        let child_node = self.arena.get_mut(child).ok_or(XmlError::InvalidNodeId)?;
        child_node.parent = Some(parent);
        child_node.prev_sibling = Some(after);
        child_node.next_sibling = old_next;

        let after_node = self.arena.get_mut(after).ok_or(XmlError::InvalidNodeId)?;
        after_node.next_sibling = Some(child);

        let parent_node = self.arena.get_mut(parent).ok_or(XmlError::InvalidNodeId)?;
        if parent_node.last_child == Some(after) {
            parent_node.last_child = Some(child);
        }

        Ok(child)
    }

    /// Helper for recursive deallocation of a node and all of its descendants.
    fn delete_recursive(&mut self, node: NodeId) {
        let mut next_child = self.first_child(node);
        while let Some(child) = next_child {
            let sibling = self.next_sibling(child);
            self.delete_recursive(child);
            next_child = sibling;
        }
        self.arena.dealloc(node);
    }

    /// Removes `child` from `parent` and deallocates it recursively.
    pub fn delete_child(&mut self, parent: NodeId, child: NodeId) -> Result<()> {
        if !self.arena.contains(parent) || !self.arena.contains(child) {
            return Err(XmlError::InvalidNodeId);
        }
        if self.parent(child) != Some(parent) {
            return Err(XmlError::InvalidNodeId);
        }

        self.unlink(child)?;
        self.delete_recursive(child);
        Ok(())
    }

    /// Removes and deallocates all children of `parent`.
    pub fn delete_children(&mut self, parent: NodeId) -> Result<()> {
        if !self.arena.contains(parent) {
            return Err(XmlError::InvalidNodeId);
        }

        let mut next_child = self.first_child(parent);
        while let Some(child) = next_child {
            let sibling = self.next_sibling(child);
            self.unlink(child)?;
            self.delete_recursive(child);
            next_child = sibling;
        }

        let parent_node = self.arena.get_mut(parent).ok_or(XmlError::InvalidNodeId)?;
        parent_node.first_child = None;
        parent_node.last_child = None;
        Ok(())
    }

    /// Unlinks `node` from its parent and recursively deallocates it.
    pub fn delete_node(&mut self, node: NodeId) -> Result<()> {
        if !self.arena.contains(node) {
            return Err(XmlError::InvalidNodeId);
        }
        if node == self.root {
            return Err(XmlError::InvalidNodeId);
        }

        self.unlink(node)?;
        self.delete_recursive(node);
        Ok(())
    }

    // --- Clone Operations ---

    /// Creates a shallow clone of the node (clones type & data only; no children, detached).
    pub fn shallow_clone(&mut self, node: NodeId) -> Result<NodeId> {
        let data = self.arena.get(node).ok_or(XmlError::InvalidNodeId)?.clone();
        let cloned_kind = match &data.kind {
            NodeKind::Document => NodeKind::Document,
            NodeKind::Element(el) => NodeKind::Element(el.clone()),
            NodeKind::Text(txt) => NodeKind::Text(txt.clone()),
            NodeKind::Comment(c) => NodeKind::Comment(c.clone()),
            NodeKind::Declaration(d) => NodeKind::Declaration(d.clone()),
            NodeKind::Unknown(u) => NodeKind::Unknown(u.clone()),
        };
        let cloned_data = NodeData::new(cloned_kind, data.line_num);
        let cloned_id = self.arena.alloc(cloned_data);
        Ok(cloned_id)
    }

    /// Recursively clones a node and all of its descendants.
    pub fn deep_clone(&mut self, node: NodeId) -> Result<NodeId> {
        let cloned_id = self.shallow_clone(node)?;
        let mut next_child = self.first_child(node);
        while let Some(child) = next_child {
            let cloned_child = self.deep_clone(child)?;
            self.insert_end_child(cloned_id, cloned_child)?;
            next_child = self.next_sibling(child);
        }
        Ok(cloned_id)
    }

    // --- Attribute Manipulation ---

    /// Returns the string value of the attribute on element `el` if it exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse(r#"<server host="localhost" port="8080"/>"#)?;
    /// let server = doc.root_element().expect("server element");
    ///
    /// assert_eq!(doc.attribute(server, "host"), Some("localhost"));
    /// assert_eq!(doc.attribute(server, "missing"), None);
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    #[must_use]
    pub fn attribute(&self, el: NodeId, name: &str) -> Option<&str> {
        let data = self.arena.get(el)?;
        match &data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => el_data
                .attributes
                .iter()
                .find(|attr| attr.name == name)
                .map(|attr| attr.value.as_str()),
            _ => None,
        }
    }

    /// Sets the value of an attribute on element `el` (inserts or updates).
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let mut doc = Document::parse("<feature/>")?;
    /// let feature = doc.root_element().expect("feature element");
    ///
    /// doc.set_attribute(feature, "enabled", "true")?;
    /// assert_eq!(doc.attribute(feature, "enabled"), Some("true"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn set_attribute(&mut self, el: NodeId, name: &str, value: &str) -> Result<()> {
        let data = self.arena.get_mut(el).ok_or(XmlError::InvalidNodeId)?;
        match &mut data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                if let Some(attr) = el_data.attributes.iter_mut().find(|attr| attr.name == name) {
                    attr.value = value.to_string();
                } else {
                    el_data.attributes.push(Attribute {
                        name: name.to_string(),
                        value: value.to_string(),
                    });
                }
                Ok(())
            }
            _ => Err(XmlError::InvalidNodeId),
        }
    }

    /// Removes an attribute from element `el`.
    pub fn delete_attribute(&mut self, el: NodeId, name: &str) -> Result<()> {
        let data = self.arena.get_mut(el).ok_or(XmlError::InvalidNodeId)?;
        match &mut data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                if let Some(pos) = el_data.attributes.iter().position(|attr| attr.name == name) {
                    el_data.attributes.remove(pos);
                    Ok(())
                } else {
                    Err(XmlError::NoAttribute)
                }
            }
            _ => Err(XmlError::InvalidNodeId),
        }
    }

    /// Returns a reference to the first Attribute on element `el`.
    #[must_use]
    pub fn first_attribute(&self, el: NodeId) -> Option<&Attribute> {
        let data = self.arena.get(el)?;
        match &data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                el_data.attributes.first()
            }
            _ => None,
        }
    }

    /// Returns the number of attributes on element `el`.
    #[must_use]
    pub fn attribute_count(&self, el: NodeId) -> usize {
        let Some(data) = self.arena.get(el) else {
            return 0;
        };
        match &data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => el_data.attributes.len(),
            _ => 0,
        }
    }

    /// Finds a reference to the Attribute with the specified name.
    #[must_use]
    pub fn find_attribute(&self, el: NodeId, name: &str) -> Option<&Attribute> {
        let data = self.arena.get(el)?;
        match &data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                el_data.attributes.iter().find(|attr| attr.name == name)
            }
            _ => None,
        }
    }

    /// Returns an iterator over all attributes of element `el`.
    pub fn iterate_attributes(&self, el: NodeId) -> impl Iterator<Item = &Attribute> {
        let attrs = match self.arena.get(el) {
            Some(data) => match &data.kind {
                NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                    &el_data.attributes[..]
                }
                _ => &[],
            },
            None => &[],
        };
        attrs.iter()
    }

    // --- Iterator & Ref Convenience APIs ---

    /// Returns an iterator over the direct children of `parent`.
    pub fn children(&self, parent: NodeId) -> crate::iter::Children<'_> {
        crate::iter::Children::new(self, parent)
    }

    /// Returns an iterator over the direct child elements of `parent`,
    /// optionally filtered by tag name.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<catalog><book/><magazine/><book/></catalog>")?;
    /// let catalog = doc.root_element().expect("catalog element");
    /// let book_count = doc.child_elements(catalog, Some("book")).count();
    ///
    /// assert_eq!(book_count, 2);
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn child_elements(
        &self,
        parent: NodeId,
        name: Option<&str>,
    ) -> crate::iter::ChildElements<'_> {
        crate::iter::ChildElements::new(self, parent, name)
    }

    /// Returns an iterator over the following siblings of `node`.
    pub fn siblings(&self, node: NodeId) -> crate::iter::Siblings<'_> {
        crate::iter::Siblings::new(self, node)
    }

    /// Returns a depth-first pre-order iterator over all descendants of `root`.
    pub fn descendants(&self, root: NodeId) -> crate::iter::Descendants<'_> {
        crate::iter::Descendants::new(self, root)
    }

    /// Returns an iterator over the attributes of element `el`.
    pub fn attributes(&self, el: NodeId) -> crate::iter::Attributes<'_> {
        let Some(data) = self.arena.get(el) else {
            return crate::iter::Attributes::empty();
        };
        match &data.kind {
            NodeKind::Element(el_data) | NodeKind::Declaration(el_data) => {
                crate::iter::Attributes::new(&el_data.attributes)
            }
            _ => crate::iter::Attributes::empty(),
        }
    }

    /// Creates an immutable navigation [`Handle`](crate::handle::Handle) for the given node.
    pub fn handle(&self, node: NodeId) -> crate::handle::Handle<'_> {
        crate::handle::Handle::new(self, node)
    }

    /// Creates a mutable navigation [`HandleMut`](crate::handle::HandleMut) for the given node.
    pub fn handle_mut(&mut self, node: NodeId) -> crate::handle::HandleMut<'_> {
        crate::handle::HandleMut::new(self, node)
    }

    /// Returns a [`NodeRef`](crate::refs::NodeRef) for the given node, if it exists.
    pub fn node_ref(&self, id: NodeId) -> Option<crate::refs::NodeRef<'_>> {
        if self.arena.contains(id) {
            Some(crate::refs::NodeRef::new(self, id))
        } else {
            None
        }
    }

    /// Returns an [`ElementRef`](crate::refs::ElementRef) for the given node,
    /// if it exists and is an Element.
    pub fn element_ref(&self, id: NodeId) -> Option<crate::refs::ElementRef<'_>> {
        let data = self.arena.get(id)?;
        match &data.kind {
            NodeKind::Element(_) => Some(crate::refs::ElementRef::new(self, id)),
            _ => None,
        }
    }

    // --- Visitor / Traversal APIs ---

    /// Walk the DOM tree starting at the document root, driving the visitor.
    pub fn accept(&self, visitor: &mut dyn crate::visitor::XmlVisitor) -> bool {
        self.accept_node(self.root, visitor)
    }

    /// Walk the DOM tree starting at the specified node, driving the visitor.
    pub fn accept_node(&self, node: NodeId, visitor: &mut dyn crate::visitor::XmlVisitor) -> bool {
        if !self.arena.contains(node) {
            return false;
        }

        let Some(data) = self.arena.get(node) else {
            return false;
        };

        match &data.kind {
            NodeKind::Document => {
                if !visitor.visit_enter_document(self) {
                    return false;
                }
                let mut current = self.first_child(node);
                while let Some(child) = current {
                    if !self.accept_node(child, visitor) {
                        return false;
                    }
                    current = self.next_sibling(child);
                }
                if !visitor.visit_exit_document(self) {
                    return false;
                }
            }
            NodeKind::Element(_) => {
                if !visitor.visit_enter_element(self, node) {
                    return false;
                }
                let mut current = self.first_child(node);
                while let Some(child) = current {
                    if !self.accept_node(child, visitor) {
                        return false;
                    }
                    current = self.next_sibling(child);
                }
                if !visitor.visit_exit_element(self, node) {
                    return false;
                }
            }
            NodeKind::Text(_) => {
                if !visitor.visit_text(self, node) {
                    return false;
                }
            }
            NodeKind::Comment(_) => {
                if !visitor.visit_comment(self, node) {
                    return false;
                }
            }
            NodeKind::Declaration(_) => {
                if !visitor.visit_declaration(self, node) {
                    return false;
                }
            }
            NodeKind::Unknown(_) => {
                if !visitor.visit_unknown(self, node) {
                    return false;
                }
            }
        }
        true
    }

    // --- Serialization APIs ---

    /// Pretty-prints the entire document to a String.
    #[must_use]
    #[allow(clippy::inherent_to_string_shadow_display)]
    pub fn to_string(&self) -> String {
        let mut printer = crate::printer::XmlPrinter::new();
        self.accept(&mut printer);
        printer.into_string()
    }

    /// Compact-prints the entire document to a String.
    #[must_use]
    pub fn to_string_compact(&self) -> String {
        let mut printer = crate::printer::XmlPrinter::new_compact();
        self.accept(&mut printer);
        printer.into_string()
    }

    /// Saves the pretty-printed document to the given file path.
    #[cfg(feature = "std")]
    pub fn save_file(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path)?;
        self.save_writer(file)
    }

    /// Saves the compact-printed document to the given file path.
    #[cfg(feature = "std")]
    pub fn save_file_compact(&self, path: impl AsRef<std::path::Path>) -> Result<()> {
        let file = std::fs::File::create(path)?;
        self.save_writer_compact(file)
    }

    /// Pretty-prints the document to the given `std::io::Write` sink.
    #[cfg(feature = "std")]
    pub fn save_writer(&self, mut writer: impl std::io::Write) -> Result<()> {
        let s = self.to_string();
        writer.write_all(s.as_bytes())?;
        Ok(())
    }

    /// Compact-prints the document to the given `std::io::Write` sink.
    #[cfg(feature = "std")]
    pub fn save_writer_compact(&self, mut writer: impl std::io::Write) -> Result<()> {
        let s = self.to_string_compact();
        writer.write_all(s.as_bytes())?;
        Ok(())
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}
