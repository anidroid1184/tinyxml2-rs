//! XML error types compatible with TinyXML2's error codes.
//!
//! This module provides [`XmlError`], a comprehensive error enum covering all
//! error conditions that TinyXML2 reports. Parse errors carry line numbers for
//! diagnostic reporting.
//!
//! # Compatibility with TinyXML2
//!
//! Every `XMLError` variant from TinyXML2 has a corresponding [`XmlError`] variant.
//! The Rust API uses `Result<T, XmlError>` instead of TinyXML2's pattern of returning
//! error codes and polling `Document::Error()`.

use alloc::string::String;
#[cfg(feature = "std")]
use alloc::string::ToString;
use core::fmt;

/// Result type alias using [`XmlError`] as the error type.
pub type Result<T> = core::result::Result<T, XmlError>;

/// Comprehensive XML error type compatible with TinyXML2's error codes.
///
/// Parse errors include the line number where the error was detected, enabling
/// precise diagnostic reporting. With the `std` feature, file I/O errors wrap the underlying
/// [`std::io::Error`].
///
/// # TinyXML2 Mapping
///
/// | TinyXML2 | tinyxml2-rs |
/// |----------|-------------|
/// | `XML_SUCCESS` | `Ok(())` |
/// | `XML_NO_ATTRIBUTE` | `XmlError::NoAttribute` |
/// | `XML_WRONG_ATTRIBUTE_TYPE` | `XmlError::WrongAttributeType` |
/// | `XML_ERROR_FILE_*` | `XmlError::Io` |
/// | `XML_ERROR_PARSING_*` | `XmlError::Parse` with specific `kind` |
/// | `XML_ERROR_EMPTY_DOCUMENT` | `XmlError::EmptyDocument` |
/// | `XML_ERROR_MISMATCHED_ELEMENT` | `XmlError::MismatchedElement` |
/// | `XML_CAN_NOT_CONVERT_TEXT` | `XmlError::CanNotConvertText` |
/// | `XML_NO_TEXT_NODE` | `XmlError::NoTextNode` |
/// | `XML_ELEMENT_DEPTH_EXCEEDED` | `XmlError::ElementDepthExceeded` |
#[derive(Debug)]
pub enum XmlError {
    // --- Attribute errors ---
    /// The requested attribute does not exist on the element.
    ///
    /// Equivalent to `XML_NO_ATTRIBUTE`.
    NoAttribute,

    /// The attribute value could not be converted to the requested type.
    ///
    /// Equivalent to `XML_WRONG_ATTRIBUTE_TYPE`.
    WrongAttributeType,

    // --- File I/O errors ---
    /// An I/O error occurred during file loading or saving.
    ///
    /// Wraps [`std::io::Error`] and covers TinyXML2's `XML_ERROR_FILE_NOT_FOUND`,
    /// `XML_ERROR_FILE_COULD_NOT_BE_OPENED`, and `XML_ERROR_FILE_READ_ERROR`.
    #[cfg(feature = "std")]
    Io(std::io::Error),

    // --- Parse errors ---
    /// A parse error occurred at the specified line.
    ///
    /// The `kind` field specifies what was being parsed when the error occurred.
    Parse {
        /// What XML construct was being parsed.
        kind: ParseErrorKind,
        /// The 1-based line number where the error was detected.
        line: u32,
        /// Optional human-readable detail message.
        message: Option<String>,
    },

    /// The document was empty or contained no XML content.
    ///
    /// Equivalent to `XML_ERROR_EMPTY_DOCUMENT`.
    EmptyDocument,

    /// An element's closing tag did not match its opening tag.
    ///
    /// Equivalent to `XML_ERROR_MISMATCHED_ELEMENT`.
    MismatchedElement {
        /// The 1-based line number where the mismatch was detected.
        line: u32,
        /// The expected element name.
        expected: String,
        /// The actual element name found.
        found: String,
    },

    /// Text content could not be converted to the requested type.
    ///
    /// Equivalent to `XML_CAN_NOT_CONVERT_TEXT`.
    CanNotConvertText,

    /// No text node was found where one was expected.
    ///
    /// Equivalent to `XML_NO_TEXT_NODE`.
    NoTextNode,

    /// The maximum element nesting depth was exceeded.
    ///
    /// This is a security limit to prevent stack overflow on deeply nested
    /// (potentially malicious) XML input.
    ///
    /// Equivalent to `XML_ELEMENT_DEPTH_EXCEEDED`.
    ElementDepthExceeded {
        /// The 1-based line number where the limit was exceeded.
        line: u32,
        /// The configured maximum depth.
        max_depth: u32,
    },

    /// A node ID was invalid (the node was deleted or never existed).
    ///
    /// This error has no TinyXML2 equivalent — it arises from the arena-based
    /// memory model where stale `NodeId` values are detected at runtime.
    InvalidNodeId,
}

/// Specifies what XML construct was being parsed when an error occurred.
///
/// Each variant maps to a TinyXML2 `XML_ERROR_PARSING_*` error code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    /// Error parsing an element tag.
    Element,
    /// Error parsing an attribute.
    Attribute,
    /// Error parsing text content.
    Text,
    /// Error parsing a CDATA section.
    Cdata,
    /// Error parsing a comment.
    Comment,
    /// Error parsing an XML declaration.
    Declaration,
    /// Error parsing an unknown construct.
    Unknown,
    /// General parse error not specific to any construct.
    General,
}

impl XmlError {
    /// Returns the TinyXML2-compatible error name string.
    ///
    /// # Examples
    ///
    /// ```
    /// use tinyxml2::XmlError;
    ///
    /// let err = XmlError::NoAttribute;
    /// assert_eq!(err.name(), "XML_NO_ATTRIBUTE");
    /// ```
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::NoAttribute => "XML_NO_ATTRIBUTE",
            Self::WrongAttributeType => "XML_WRONG_ATTRIBUTE_TYPE",
            #[cfg(feature = "std")]
            Self::Io(_) => "XML_ERROR_FILE_READ_ERROR",
            Self::Parse { kind, .. } => kind.error_name(),
            Self::EmptyDocument => "XML_ERROR_EMPTY_DOCUMENT",
            Self::MismatchedElement { .. } => "XML_ERROR_MISMATCHED_ELEMENT",
            Self::CanNotConvertText => "XML_CAN_NOT_CONVERT_TEXT",
            Self::NoTextNode => "XML_NO_TEXT_NODE",
            Self::ElementDepthExceeded { .. } => "XML_ELEMENT_DEPTH_EXCEEDED",
            Self::InvalidNodeId => "XML_INVALID_NODE_ID",
        }
    }

    /// Returns the line number associated with this error, if any.
    ///
    /// Parse errors and mismatched element errors include line numbers.
    /// Other error types return `None`.
    #[must_use]
    pub fn line(&self) -> Option<u32> {
        match self {
            Self::Parse { line, .. }
            | Self::MismatchedElement { line, .. }
            | Self::ElementDepthExceeded { line, .. } => Some(*line),
            _ => None,
        }
    }
}

impl ParseErrorKind {
    /// Returns the TinyXML2-compatible error name for this parse error kind.
    #[must_use]
    pub const fn error_name(self) -> &'static str {
        match self {
            Self::Element => "XML_ERROR_PARSING_ELEMENT",
            Self::Attribute => "XML_ERROR_PARSING_ATTRIBUTE",
            Self::Text => "XML_ERROR_PARSING_TEXT",
            Self::Cdata => "XML_ERROR_PARSING_CDATA",
            Self::Comment => "XML_ERROR_PARSING_COMMENT",
            Self::Declaration => "XML_ERROR_PARSING_DECLARATION",
            Self::Unknown => "XML_ERROR_PARSING_UNKNOWN",
            Self::General => "XML_ERROR_PARSING",
        }
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAttribute => write!(f, "attribute not found"),
            Self::WrongAttributeType => write!(f, "wrong attribute type"),
            #[cfg(feature = "std")]
            Self::Io(err) => write!(f, "I/O error: {err}"),
            Self::Parse {
                kind,
                line,
                message,
            } => {
                write!(f, "parse error ({}) at line {line}", kind.error_name())?;
                if let Some(msg) = message {
                    write!(f, ": {msg}")?;
                }
                Ok(())
            }
            Self::EmptyDocument => write!(f, "empty document"),
            Self::MismatchedElement {
                line,
                expected,
                found,
            } => write!(
                f,
                "mismatched element at line {line}: expected </{expected}>, found </{found}>"
            ),
            Self::CanNotConvertText => write!(f, "cannot convert text to requested type"),
            Self::NoTextNode => write!(f, "no text node found"),
            Self::ElementDepthExceeded { line, max_depth } => {
                write!(
                    f,
                    "element depth exceeded maximum of {max_depth} at line {line}"
                )
            }
            Self::InvalidNodeId => write!(f, "invalid node ID (node was deleted or never existed)"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for XmlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for XmlError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl Clone for XmlError {
    fn clone(&self) -> Self {
        match self {
            Self::NoAttribute => Self::NoAttribute,
            Self::WrongAttributeType => Self::WrongAttributeType,
            #[cfg(feature = "std")]
            Self::Io(err) => Self::Io(std::io::Error::new(err.kind(), err.to_string())),
            Self::Parse {
                kind,
                line,
                message,
            } => Self::Parse {
                kind: *kind,
                line: *line,
                message: message.clone(),
            },
            Self::EmptyDocument => Self::EmptyDocument,
            Self::MismatchedElement {
                line,
                expected,
                found,
            } => Self::MismatchedElement {
                line: *line,
                expected: expected.clone(),
                found: found.clone(),
            },
            Self::CanNotConvertText => Self::CanNotConvertText,
            Self::NoTextNode => Self::NoTextNode,
            Self::ElementDepthExceeded { line, max_depth } => Self::ElementDepthExceeded {
                line: *line,
                max_depth: *max_depth,
            },
            Self::InvalidNodeId => Self::InvalidNodeId,
        }
    }
}

impl PartialEq for XmlError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NoAttribute, Self::NoAttribute)
            | (Self::WrongAttributeType, Self::WrongAttributeType)
            | (Self::EmptyDocument, Self::EmptyDocument)
            | (Self::CanNotConvertText, Self::CanNotConvertText)
            | (Self::NoTextNode, Self::NoTextNode)
            | (Self::InvalidNodeId, Self::InvalidNodeId) => true,
            (
                Self::Parse {
                    kind: k1, line: l1, ..
                },
                Self::Parse {
                    kind: k2, line: l2, ..
                },
            ) => k1 == k2 && l1 == l2,
            (
                Self::MismatchedElement {
                    line: l1,
                    expected: e1,
                    found: f1,
                },
                Self::MismatchedElement {
                    line: l2,
                    expected: e2,
                    found: f2,
                },
            ) => l1 == l2 && e1 == e2 && f1 == f2,
            (
                Self::ElementDepthExceeded {
                    line: l1,
                    max_depth: m1,
                },
                Self::ElementDepthExceeded {
                    line: l2,
                    max_depth: m2,
                },
            ) => l1 == l2 && m1 == m2,
            #[cfg(feature = "std")]
            (Self::Io(a), Self::Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}

impl Eq for XmlError {}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    use super::*;

    #[test]
    fn error_names_match_tinyxml2() {
        assert_eq!(XmlError::NoAttribute.name(), "XML_NO_ATTRIBUTE");
        assert_eq!(
            XmlError::WrongAttributeType.name(),
            "XML_WRONG_ATTRIBUTE_TYPE"
        );
        assert_eq!(XmlError::EmptyDocument.name(), "XML_ERROR_EMPTY_DOCUMENT");
        assert_eq!(
            XmlError::CanNotConvertText.name(),
            "XML_CAN_NOT_CONVERT_TEXT"
        );
        assert_eq!(XmlError::NoTextNode.name(), "XML_NO_TEXT_NODE");
        assert_eq!(
            XmlError::ElementDepthExceeded {
                line: 1,
                max_depth: 500
            }
            .name(),
            "XML_ELEMENT_DEPTH_EXCEEDED"
        );
    }

    #[test]
    fn parse_error_names() {
        let cases = [
            (ParseErrorKind::Element, "XML_ERROR_PARSING_ELEMENT"),
            (ParseErrorKind::Attribute, "XML_ERROR_PARSING_ATTRIBUTE"),
            (ParseErrorKind::Text, "XML_ERROR_PARSING_TEXT"),
            (ParseErrorKind::Cdata, "XML_ERROR_PARSING_CDATA"),
            (ParseErrorKind::Comment, "XML_ERROR_PARSING_COMMENT"),
            (ParseErrorKind::Declaration, "XML_ERROR_PARSING_DECLARATION"),
            (ParseErrorKind::Unknown, "XML_ERROR_PARSING_UNKNOWN"),
            (ParseErrorKind::General, "XML_ERROR_PARSING"),
        ];
        for (kind, expected_name) in cases {
            let err = XmlError::Parse {
                kind,
                line: 1,
                message: None,
            };
            assert_eq!(err.name(), expected_name);
        }
    }

    #[test]
    fn error_line_numbers() {
        assert_eq!(XmlError::NoAttribute.line(), None);
        assert_eq!(
            XmlError::Parse {
                kind: ParseErrorKind::Element,
                line: 42,
                message: None,
            }
            .line(),
            Some(42)
        );
        assert_eq!(
            XmlError::MismatchedElement {
                line: 10,
                expected: "foo".into(),
                found: "bar".into(),
            }
            .line(),
            Some(10)
        );
        assert_eq!(
            XmlError::ElementDepthExceeded {
                line: 500,
                max_depth: 100,
            }
            .line(),
            Some(500)
        );
    }

    #[test]
    fn display_formatting() {
        assert_eq!(XmlError::NoAttribute.to_string(), "attribute not found");
        assert_eq!(XmlError::EmptyDocument.to_string(), "empty document");

        let parse_err = XmlError::Parse {
            kind: ParseErrorKind::Element,
            line: 5,
            message: Some("unexpected character".into()),
        };
        assert_eq!(
            parse_err.to_string(),
            "parse error (XML_ERROR_PARSING_ELEMENT) at line 5: unexpected character"
        );

        let mismatch = XmlError::MismatchedElement {
            line: 3,
            expected: "div".into(),
            found: "span".into(),
        };
        assert_eq!(
            mismatch.to_string(),
            "mismatched element at line 3: expected </div>, found </span>"
        );
    }

    #[test]
    fn error_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<XmlError>();
        assert_sync::<XmlError>();
    }

    #[test]
    #[cfg(feature = "std")]
    fn io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let xml_err: XmlError = io_err.into();
        assert_eq!(xml_err.name(), "XML_ERROR_FILE_READ_ERROR");
        assert!(xml_err.to_string().contains("file not found"));
    }

    #[test]
    fn error_clone_and_eq() {
        let err = XmlError::Parse {
            kind: ParseErrorKind::Element,
            line: 10,
            message: Some("test".into()),
        };
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    #[test]
    #[cfg(feature = "std")]
    fn xml_error_as_box_dyn_error() {
        let err = XmlError::NoAttribute;
        let boxed: Box<dyn std::error::Error> = Box::new(err);
        let _display = boxed.to_string();
        let _source = boxed.source();
    }

    #[test]
    #[cfg(feature = "std")]
    fn std_error_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let xml_err = XmlError::Io(io_err);
        assert!(std::error::Error::source(&xml_err).is_some());

        assert!(std::error::Error::source(&XmlError::NoAttribute).is_none());
    }
}
