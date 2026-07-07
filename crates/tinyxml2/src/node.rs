//! XML DOM node types.

use alloc::string::String;
use alloc::vec::Vec;

use crate::arena::NodeId;

/// An XML attribute name-value pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name of the attribute.
    pub name: String,
    /// The value of the attribute.
    pub value: String,
}

/// Node-specific data for XML elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementData {
    /// The tag name of the element.
    pub name: String,
    /// The attributes of the element.
    pub attributes: Vec<Attribute>,
}

/// Node-specific data for text nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextData {
    /// The text content.
    pub content: String,
    /// Whether the text was parsed as a CDATA section.
    pub is_cdata: bool,
}

/// The kind of XML node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// The top-level document node.
    Document,
    /// An XML element (e.g., `<element name="value">`).
    Element(ElementData),
    /// A text node or CDATA section.
    Text(TextData),
    /// A comment (e.g., `<!-- comment -->`).
    Comment(String),
    /// An XML declaration (e.g., `<?xml version="1.0"?>`).
    Declaration(ElementData),
    /// An unrecognized structure (e.g., `<!DOCTYPE ...>`).
    Unknown(String),
}

/// Represents a single node's data in the DOM tree, including tree linkages.
#[derive(Debug, Clone)]
pub struct NodeData {
    /// The specific kind and data of this node.
    pub kind: NodeKind,
    /// The parent node.
    pub parent: Option<NodeId>,
    /// The first child node.
    pub first_child: Option<NodeId>,
    /// The last child node.
    pub last_child: Option<NodeId>,
    /// The previous sibling node.
    pub prev_sibling: Option<NodeId>,
    /// The next sibling node.
    pub next_sibling: Option<NodeId>,
    /// The 1-based source line number where this node was parsed.
    pub line_num: u32,
}

impl NodeData {
    /// Creates a new, detached `NodeData` with the specified kind and line number.
    #[must_use]
    pub const fn new(kind: NodeKind, line_num: u32) -> Self {
        Self {
            kind,
            parent: None,
            first_child: None,
            last_child: None,
            prev_sibling: None,
            next_sibling: None,
            line_num,
        }
    }
}
