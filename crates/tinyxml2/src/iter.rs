//! Iterator adapters for DOM tree traversal.
//!
//! Provides standard Rust iterators over DOM linkages, yielding typed
//! reference wrappers ([`NodeRef`] and [`ElementRef`]) for safe,
//! ergonomic tree navigation.

use alloc::string::String;
use core::{iter::FusedIterator, slice};

use crate::arena::NodeId;
use crate::document::Document;
use crate::node::{Attribute, NodeKind};
use crate::refs::{ElementRef, NodeRef};

/// An iterator over the direct children of a node, yielding [`NodeRef`].
///
/// Supports both forward and backward iteration via [`DoubleEndedIterator`].
///
/// Created by [`NodeRef::children`], [`ElementRef::children`], or
/// [`Document::children`].
#[derive(Debug, Clone)]
pub struct Children<'a> {
    doc: &'a Document,
    front: Option<NodeId>,
    back: Option<NodeId>,
}

impl<'a> Children<'a> {
    /// Creates a new `Children` iterator over the children of `parent`.
    pub(crate) fn new(doc: &'a Document, parent: NodeId) -> Self {
        Self {
            doc,
            front: doc.first_child(parent),
            back: doc.last_child(parent),
        }
    }
}

impl<'a> Iterator for Children<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.front?;
        if self.front == self.back {
            self.front = None;
            self.back = None;
        } else {
            self.front = self.doc.next_sibling(id);
        }
        Some(NodeRef::new(self.doc, id))
    }
}

impl DoubleEndedIterator for Children<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let id = self.back?;
        if self.front == self.back {
            self.front = None;
            self.back = None;
        } else {
            self.back = self.doc.prev_sibling(id);
        }
        Some(NodeRef::new(self.doc, id))
    }
}

impl FusedIterator for Children<'_> {}

/// An iterator over direct child elements of a node, yielding [`ElementRef`].
///
/// Optionally filters by element tag name. Created by
/// [`ElementRef::child_elements`] or [`Document::child_elements`].
#[derive(Debug, Clone)]
pub struct ChildElements<'a> {
    doc: &'a Document,
    current: Option<NodeId>,
    name_filter: Option<String>,
}

impl<'a> ChildElements<'a> {
    /// Creates a new `ChildElements` iterator starting from the first child
    /// of `parent`, optionally filtered to elements with the given `name`.
    pub(crate) fn new(doc: &'a Document, parent: NodeId, name: Option<&str>) -> Self {
        Self {
            doc,
            current: doc.first_child(parent),
            name_filter: name.map(String::from),
        }
    }
}

impl<'a> Iterator for ChildElements<'a> {
    type Item = ElementRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(id) = self.current {
            self.current = self.doc.next_sibling(id);
            if let Some(data) = self.doc.arena.get(id) {
                if let NodeKind::Element(ref el_data) = data.kind {
                    if self.name_filter.as_ref().is_none_or(|n| n == &el_data.name) {
                        return Some(ElementRef::new(self.doc, id));
                    }
                }
            }
        }
        None
    }
}

impl FusedIterator for ChildElements<'_> {}

/// An iterator over the following siblings of a node, yielding [`NodeRef`].
///
/// Does **not** include the starting node itself. Created by
/// [`NodeRef::siblings`] or [`Document::siblings`].
#[derive(Debug, Clone)]
pub struct Siblings<'a> {
    doc: &'a Document,
    current: Option<NodeId>,
}

impl<'a> Siblings<'a> {
    /// Creates a new `Siblings` iterator starting after `node`.
    pub(crate) fn new(doc: &'a Document, node: NodeId) -> Self {
        Self {
            doc,
            current: doc.next_sibling(node),
        }
    }
}

impl<'a> Iterator for Siblings<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        self.current = self.doc.next_sibling(id);
        Some(NodeRef::new(self.doc, id))
    }
}

impl FusedIterator for Siblings<'_> {}

/// An iterator over an element's attributes, yielding `(&str, &str)` pairs.
///
/// Supports both forward and backward iteration via [`DoubleEndedIterator`].
///
/// Created by [`ElementRef::attributes`] or [`Document::attributes`].
#[derive(Debug, Clone)]
pub struct Attributes<'a> {
    inner: slice::Iter<'a, Attribute>,
}

impl<'a> Attributes<'a> {
    /// Creates a new `Attributes` iterator from an attribute slice.
    pub(crate) fn new(attrs: &'a [Attribute]) -> Self {
        Self {
            inner: attrs.iter(),
        }
    }

    /// Creates an empty `Attributes` iterator.
    pub(crate) fn empty() -> Self {
        Self { inner: [].iter() }
    }
}

impl<'a> Iterator for Attributes<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner
            .next()
            .map(|attr| (attr.name.as_str(), attr.value.as_str()))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl DoubleEndedIterator for Attributes<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner
            .next_back()
            .map(|attr| (attr.name.as_str(), attr.value.as_str()))
    }
}

impl ExactSizeIterator for Attributes<'_> {}
impl FusedIterator for Attributes<'_> {}

/// A depth-first pre-order iterator over all descendants of a node,
/// yielding [`NodeRef`].
///
/// Does **not** include the root node itself. Created by
/// [`NodeRef::descendants`] or [`Document::descendants`].
#[derive(Debug, Clone)]
pub struct Descendants<'a> {
    doc: &'a Document,
    root: NodeId,
    current: Option<NodeId>,
}

impl<'a> Descendants<'a> {
    /// Creates a new `Descendants` iterator starting from the first child
    /// of `root` in depth-first pre-order.
    pub(crate) fn new(doc: &'a Document, root: NodeId) -> Self {
        Self {
            doc,
            root,
            current: doc.first_child(root),
        }
    }

    /// Advances to the next node in depth-first pre-order.
    fn advance(&self, node: NodeId) -> Option<NodeId> {
        // Try going deeper (first child)
        if let Some(child) = self.doc.first_child(node) {
            return Some(child);
        }
        // Try going sideways or up
        let mut current = node;
        loop {
            if current == self.root {
                return None;
            }
            if let Some(sibling) = self.doc.next_sibling(current) {
                return Some(sibling);
            }
            // Go up to parent
            current = self.doc.parent(current)?;
        }
    }
}

impl<'a> Iterator for Descendants<'a> {
    type Item = NodeRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.current?;
        self.current = self.advance(id);
        Some(NodeRef::new(self.doc, id))
    }
}

impl FusedIterator for Descendants<'_> {}
