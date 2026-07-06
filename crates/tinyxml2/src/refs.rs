//! Typed, lifetime-bounded reference wrappers for DOM nodes.
//!
//! [`NodeRef`] and [`ElementRef`] wrap a `&Document` reference and a
//! [`NodeId`], providing safe, ergonomic access to node data without
//! requiring the caller to pass the document around separately.

use core::ptr;

use crate::arena::NodeId;
use crate::document::Document;
use crate::iter::{Attributes, ChildElements, Children, Descendants, Siblings};
use crate::node::NodeKind;

/// A lightweight, immutable reference to any node in the DOM tree.
///
/// Wraps a `&Document` and a [`NodeId`], providing convenient access to
/// the node's data, navigation, and iterators without requiring the caller
/// to thread the `Document` reference manually.
///
/// # Lifetime
///
/// The `'a` lifetime ties this reference to the borrowed `Document`,
/// preventing iterator invalidation.
#[derive(Debug, Clone, Copy)]
pub struct NodeRef<'a> {
    doc: &'a Document,
    id: NodeId,
}

impl<'a> NodeRef<'a> {
    /// Creates a new `NodeRef`.
    pub(crate) fn new(doc: &'a Document, id: NodeId) -> Self {
        Self { doc, id }
    }

    /// Returns the [`NodeId`] of this node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a reference to the underlying [`Document`].
    pub fn document(&self) -> &'a Document {
        self.doc
    }

    /// Returns the [`NodeKind`] of this node.
    ///
    /// # Panics
    ///
    /// Panics if the node no longer exists in the arena (should not happen
    /// while this reference is alive due to the shared borrow on `Document`).
    pub fn kind(&self) -> &'a NodeKind {
        &self
            .doc
            .arena
            .get(self.id)
            .expect("NodeRef holds invalid id")
            .kind
    }

    /// Returns the parent of this node, if it has one.
    pub fn parent(&self) -> Option<NodeRef<'a>> {
        self.doc
            .parent(self.id)
            .map(|pid| NodeRef::new(self.doc, pid))
    }

    /// Returns an iterator over the direct children of this node.
    pub fn children(&self) -> Children<'a> {
        Children::new(self.doc, self.id)
    }

    /// Returns an iterator over the following siblings of this node.
    pub fn siblings(&self) -> Siblings<'a> {
        Siblings::new(self.doc, self.id)
    }

    /// Returns an iterator over all descendants in depth-first pre-order.
    pub fn descendants(&self) -> Descendants<'a> {
        Descendants::new(self.doc, self.id)
    }

    /// Returns the "value" of this node based on its kind.
    ///
    /// - `Element` → tag name
    /// - `Text` → text content
    /// - `Comment` → comment text
    /// - `Declaration` → declaration name
    /// - `Unknown` → raw content
    /// - `Document` → empty string
    pub fn value(&self) -> &'a str {
        let Some(data) = self.doc.arena.get(self.id) else {
            return "";
        };
        match &data.kind {
            NodeKind::Document => "",
            NodeKind::Element(el) | NodeKind::Declaration(el) => &el.name,
            NodeKind::Text(txt) => &txt.content,
            NodeKind::Comment(s) | NodeKind::Unknown(s) => s,
        }
    }

    /// Returns the 1-based source line number where this node was parsed.
    pub fn line(&self) -> u32 {
        self.doc.line_num(self.id).unwrap_or(0)
    }

    /// Attempts to downcast this node reference to an [`ElementRef`].
    ///
    /// Returns `Some(ElementRef)` if the node is an `Element`, otherwise `None`.
    pub fn as_element(&self) -> Option<ElementRef<'a>> {
        let data = self.doc.arena.get(self.id)?;
        match &data.kind {
            NodeKind::Element(_) => Some(ElementRef::new(self.doc, self.id)),
            _ => None,
        }
    }
}

impl PartialEq for NodeRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.doc, other.doc) && self.id == other.id
    }
}

impl Eq for NodeRef<'_> {}

/// A typed, immutable reference to an Element node in the DOM tree.
///
/// Constructed via [`NodeRef::as_element`], [`Document::element_ref`],
/// or yielded by [`ChildElements`] iterators. Guaranteed to reference
/// an `Element` node.
///
/// # Lifetime
///
/// The `'a` lifetime ties this reference to the borrowed `Document`,
/// preventing iterator invalidation.
#[derive(Debug, Clone, Copy)]
pub struct ElementRef<'a> {
    doc: &'a Document,
    id: NodeId,
}

impl<'a> ElementRef<'a> {
    /// Creates a new `ElementRef`.
    ///
    /// # Safety contract (not `unsafe`, but a logical invariant)
    ///
    /// Callers within the crate must ensure `id` refers to an `Element` node.
    pub(crate) fn new(doc: &'a Document, id: NodeId) -> Self {
        Self { doc, id }
    }

    /// Returns the [`NodeId`] of this element.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Returns a reference to the underlying [`Document`].
    pub fn document(&self) -> &'a Document {
        self.doc
    }

    /// Upcasts this `ElementRef` to a general [`NodeRef`].
    pub fn as_node(&self) -> NodeRef<'a> {
        NodeRef::new(self.doc, self.id)
    }

    /// Returns the tag name of this element.
    ///
    /// # Panics
    ///
    /// Panics if the underlying node is not an `Element` (should not happen
    /// if the `ElementRef` was constructed correctly).
    pub fn name(&self) -> &'a str {
        let data = self
            .doc
            .arena
            .get(self.id)
            .expect("ElementRef holds invalid id");
        match &data.kind {
            NodeKind::Element(el) => &el.name,
            _ => panic!("ElementRef references a non-Element node"),
        }
    }

    /// Returns the string value of the named attribute, if it exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse(r#"<package name="tinyxml2" version="1.0"/>"#)?;
    /// let package = doc.element_ref(doc.root_element().expect("package element")).expect("element");
    ///
    /// assert_eq!(package.attribute("name"), Some("tinyxml2"));
    /// assert_eq!(package.attribute("missing"), None);
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn attribute(&self, name: &str) -> Option<&'a str> {
        self.doc.attribute(self.id, name)
    }

    /// Returns an iterator over all attributes as `(&str, &str)` pairs.
    pub fn attributes(&self) -> Attributes<'a> {
        let Some(data) = self.doc.arena.get(self.id) else {
            return Attributes::empty();
        };
        match &data.kind {
            NodeKind::Element(el) => Attributes::new(&el.attributes),
            _ => Attributes::empty(),
        }
    }

    /// Returns an iterator over the direct children of this element.
    pub fn children(&self) -> Children<'a> {
        Children::new(self.doc, self.id)
    }

    /// Returns an iterator over child elements.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<menu><item>Tea</item><item>Coffee</item></menu>")?;
    /// let menu = doc.element_ref(doc.root_element().expect("menu element")).expect("element");
    /// let names: Vec<&str> = menu.child_elements().map(|item| item.name()).collect();
    ///
    /// assert_eq!(names, vec!["item", "item"]);
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn child_elements(&self) -> ChildElements<'a> {
        ChildElements::new(self.doc, self.id, None)
    }

    /// Returns an iterator over child elements filtered by tag name.
    pub fn child_elements_by_name(&self, name: &str) -> ChildElements<'a> {
        ChildElements::new(self.doc, self.id, Some(name))
    }

    /// Returns the text content of the first child Text node, if one exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::Document;
    ///
    /// let doc = Document::parse("<status>ready</status>")?;
    /// let status = doc.element_ref(doc.root_element().expect("status element")).expect("element");
    ///
    /// assert_eq!(status.text(), Some("ready"));
    /// # Ok::<(), tinyxml2::XmlError>(())
    /// ```
    pub fn text(&self) -> Option<&'a str> {
        self.doc.get_text(self.id)
    }

    /// Returns the value of the named attribute as an `i32`, or `default`
    /// if the attribute is missing or cannot be parsed.
    pub fn int_attribute(&self, name: &str, default: i32) -> i32 {
        self.doc.int_attribute(self.id, name, default)
    }

    /// Returns the value of the named attribute as a `bool`, or `default`
    /// if the attribute is missing or cannot be parsed.
    pub fn bool_attribute(&self, name: &str, default: bool) -> bool {
        self.doc.bool_attribute(self.id, name, default)
    }

    /// Returns the value of the named attribute as an `f64`, or `default`
    /// if the attribute is missing or cannot be parsed.
    pub fn double_attribute(&self, name: &str, default: f64) -> f64 {
        self.doc.double_attribute(self.id, name, default)
    }
}

impl PartialEq for ElementRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.doc, other.doc) && self.id == other.id
    }
}

impl Eq for ElementRef<'_> {}
