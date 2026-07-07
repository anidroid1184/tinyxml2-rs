//! # tinyxml2
//!
//! A ground-up Rust implementation of the [TinyXML2](https://github.com/leethomason/tinyxml2)
//! API — behavioral compatibility with idiomatic Rust internals.
//!
//! ## Overview
//!
//! `tinyxml2` provides a lightweight, DOM-based XML parser and serializer that
//! is behaviorally compatible with the C++ TinyXML2 library. TinyXML2 is treated
//! as the **specification** — this crate matches its parsing semantics,
//! serialization output, entity handling, and error behavior while using Rust's
//! type system, ownership model, and standard library conventions internally.
//!
//! ## What this crate is NOT
//!
//! - **Not a wrapper** around the C++ TinyXML2 library
//! - **Not a line-by-line translation** of C++ code
//! - **Not a fork** — it is an entirely new implementation
//!
//! ## Architecture
//!
//! The DOM is backed by a **generational arena allocator** where the `Document`
//! owns all nodes. This matches TinyXML2's ownership model (`XMLDocument` owns
//! everything) while providing Rust's memory safety guarantees without `unsafe`.
//!
//! ### Modules
//!
//! | Module | Purpose |
//! |--------|---------|
//! | [`error`] | Error types compatible with TinyXML2's error codes |
//! | [`entity`] | XML entity encoding and decoding |
//! | [`util`] | Character classification and string utilities |
//! | [`arena`] | Generational arena allocator for DOM nodes |
//!
//! ## Status
//!
//! This crate is under active development. See the
//! [roadmap](https://github.com/Teebot/tinyxml2-rs/blob/main/ROADMAP.md) for
//! the current implementation phase.

// Enforce no unsafe in the core crate
#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
// Documentation quality
#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

extern crate alloc;

pub mod arena;
pub mod document;
pub mod entity;
pub mod error;
pub mod handle;
pub mod iter;
pub mod node;
pub(crate) mod parser;
pub mod printer;
pub mod refs;
pub mod typed;
pub mod util;
pub mod visitor;

// Re-export primary types at crate root for convenience
pub use arena::NodeId;
pub use document::Document;
pub use error::{ParseErrorKind, Result, XmlError};
pub use handle::{Handle, HandleMut};
pub use iter::{Attributes, ChildElements, Children, Descendants, Siblings};
pub use node::{Attribute, ElementData, NodeData, NodeKind, TextData};
pub use printer::XmlPrinter;
pub use refs::{ElementRef, NodeRef};
pub use visitor::XmlVisitor;

/// Type alias for [`XmlPrinter`] to maintain compatibility with C++ naming conventions.
pub type Printer = XmlPrinter;

/// Whitespace handling mode, matching TinyXML2's `Whitespace` enum.
///
/// Set at document construction time via [`ParseOptions`] to control how
/// whitespace in text content is processed during parsing.
///
/// # TinyXML2 Compatibility
///
/// | TinyXML2 | tinyxml2-rs |
/// |----------|-------------|
/// | `PRESERVE_WHITESPACE` | [`Whitespace::Preserve`] |
/// | `COLLAPSE_WHITESPACE` | [`Whitespace::Collapse`] |
/// | `PEDANTIC_WHITESPACE` | [`Whitespace::Pedantic`] |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Whitespace {
    /// Preserve all whitespace exactly as it appears in the source.
    ///
    /// This is the default mode, matching TinyXML2's `PRESERVE_WHITESPACE`.
    #[default]
    Preserve,

    /// Collapse whitespace: strip leading/trailing, collapse internal runs
    /// to single spaces, convert all whitespace characters to spaces.
    ///
    /// Matches TinyXML2's `COLLAPSE_WHITESPACE`.
    Collapse,

    /// Pedantic whitespace preservation: like [`Preserve`](Whitespace::Preserve)
    /// but also keeps whitespace-only text nodes that would otherwise be
    /// discarded.
    ///
    /// Matches TinyXML2's `PEDANTIC_WHITESPACE`.
    Pedantic,
}

/// Options for parsing XML documents.
///
/// Replaces TinyXML2's `XMLDocument` constructor parameters with a builder-style
/// options struct for clarity and extensibility.
///
/// # Examples
///
/// ```
/// use tinyxml2::{ParseOptions, Whitespace};
///
/// let opts = ParseOptions::new()
///     .with_whitespace(Whitespace::Collapse)
///     .with_process_entities(false)
///     .with_max_depth(100);
/// ```
#[derive(Debug, Clone)]
pub struct ParseOptions {
    /// Whether to process XML entities during parsing.
    ///
    /// When `true` (default), entity references like `&amp;` are decoded.
    /// When `false`, entity text is preserved as-is.
    pub process_entities: bool,

    /// Whitespace handling mode.
    pub whitespace: Whitespace,

    /// Maximum allowed element nesting depth.
    ///
    /// Prevents stack overflow on deeply nested (potentially malicious) input.
    /// Default is 500, matching TinyXML2's `TINYXML2_MAX_ELEMENT_DEPTH`.
    pub max_depth: u32,
}

impl ParseOptions {
    /// Creates default parse options.
    ///
    /// - `process_entities`: `true`
    /// - `whitespace`: [`Whitespace::Preserve`]
    /// - `max_depth`: 500
    #[must_use]
    pub const fn new() -> Self {
        Self {
            process_entities: true,
            whitespace: Whitespace::Preserve,
            max_depth: 500,
        }
    }

    /// Sets the whitespace handling mode.
    #[must_use]
    pub const fn with_whitespace(mut self, whitespace: Whitespace) -> Self {
        self.whitespace = whitespace;
        self
    }

    /// Sets whether entities should be processed during parsing.
    #[must_use]
    pub const fn with_process_entities(mut self, process: bool) -> Self {
        self.process_entities = process;
        self
    }

    /// Sets the maximum element nesting depth.
    #[must_use]
    pub const fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_default() {
        assert_eq!(Whitespace::default(), Whitespace::Preserve);
    }

    #[test]
    fn parse_options_defaults() {
        let opts = ParseOptions::new();
        assert!(opts.process_entities);
        assert_eq!(opts.whitespace, Whitespace::Preserve);
        assert_eq!(opts.max_depth, 500);
    }

    #[test]
    fn parse_options_builder() {
        let opts = ParseOptions::new()
            .with_whitespace(Whitespace::Collapse)
            .with_process_entities(false)
            .with_max_depth(100);

        assert!(!opts.process_entities);
        assert_eq!(opts.whitespace, Whitespace::Collapse);
        assert_eq!(opts.max_depth, 100);
    }

    #[test]
    fn parse_options_default_trait() {
        let opts = ParseOptions::default();
        assert_eq!(opts.max_depth, 500);
    }
}
