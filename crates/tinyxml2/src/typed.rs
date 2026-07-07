//! Typed attribute and text parsing implementations on Document.

use alloc::string::ToString;
use core::str::FromStr;

use crate::arena::NodeId;
use crate::document::Document;
use crate::error::{Result, XmlError};
use crate::node::NodeKind;

// --- Helper Functions for Parsing ---

fn parse_bool(s: &str) -> Result<bool> {
    let s_trimmed = s.trim();
    if s_trimmed.eq_ignore_ascii_case("true") || s_trimmed == "1" {
        Ok(true)
    } else if s_trimmed.eq_ignore_ascii_case("false") || s_trimmed == "0" {
        Ok(false)
    } else {
        Err(XmlError::WrongAttributeType)
    }
}

fn parse_numeric<T: FromStr>(s: &str) -> Result<T> {
    s.trim()
        .parse::<T>()
        .map_err(|_| XmlError::WrongAttributeType)
}

fn parse_bool_text(s: &str) -> Result<bool> {
    let s_trimmed = s.trim();
    if s_trimmed.eq_ignore_ascii_case("true") || s_trimmed == "1" {
        Ok(true)
    } else if s_trimmed.eq_ignore_ascii_case("false") || s_trimmed == "0" {
        Ok(false)
    } else {
        Err(XmlError::CanNotConvertText)
    }
}

fn parse_numeric_text<T: FromStr>(s: &str) -> Result<T> {
    s.trim()
        .parse::<T>()
        .map_err(|_| XmlError::CanNotConvertText)
}

impl Document {
    // --- Attribute Typed Access ---

    /// Queries the value of the attribute as an `i32`.
    pub fn query_int_attribute(&self, el: NodeId, name: &str) -> Result<i32> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_numeric(val)
    }

    /// Queries the value of the attribute as a `u32`.
    pub fn query_unsigned_attribute(&self, el: NodeId, name: &str) -> Result<u32> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_numeric(val)
    }

    /// Queries the value of the attribute as an `i64`.
    pub fn query_int64_attribute(&self, el: NodeId, name: &str) -> Result<i64> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_numeric(val)
    }

    /// Queries the value of the attribute as a `bool`.
    pub fn query_bool_attribute(&self, el: NodeId, name: &str) -> Result<bool> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_bool(val)
    }

    /// Queries the value of the attribute as an `f64`.
    pub fn query_double_attribute(&self, el: NodeId, name: &str) -> Result<f64> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_numeric(val)
    }

    /// Queries the value of the attribute as an `f32`.
    pub fn query_float_attribute(&self, el: NodeId, name: &str) -> Result<f32> {
        let val = self.attribute(el, name).ok_or(XmlError::NoAttribute)?;
        parse_numeric(val)
    }

    /// Returns the value of the attribute as an `i32`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn int_attribute(&self, el: NodeId, name: &str, default: i32) -> i32 {
        self.query_int_attribute(el, name).unwrap_or(default)
    }

    /// Returns the value of the attribute as a `bool`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn bool_attribute(&self, el: NodeId, name: &str, default: bool) -> bool {
        self.query_bool_attribute(el, name).unwrap_or(default)
    }

    /// Returns the value of the attribute as an `f64`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn double_attribute(&self, el: NodeId, name: &str, default: f64) -> f64 {
        self.query_double_attribute(el, name).unwrap_or(default)
    }

    /// Returns the value of the attribute as an `f32`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn float_attribute(&self, el: NodeId, name: &str, default: f32) -> f32 {
        self.query_float_attribute(el, name).unwrap_or(default)
    }

    // --- Text Content Helpers ---

    /// Returns the text content of the first child of `el` that is a Text node, if one exists.
    #[must_use]
    pub fn get_text(&self, el: NodeId) -> Option<&str> {
        let mut child = self.first_child(el);
        while let Some(c) = child {
            if let Some(data) = self.arena.get(c) {
                if let NodeKind::Text(txt_data) = &data.kind {
                    return Some(txt_data.content.as_str());
                }
            }
            child = self.next_sibling(c);
        }
        None
    }

    /// Sets or replaces the text content of the first child Text node of `el`.
    ///
    /// If no child Text node exists, one is created and inserted as the first child.
    pub fn set_text(&mut self, el: NodeId, text: &str) -> Result<()> {
        let mut child = self.first_child(el);
        let mut text_node_id = None;
        while let Some(c) = child {
            if let Some(data) = self.arena.get(c) {
                if let NodeKind::Text(_) = &data.kind {
                    text_node_id = Some(c);
                    break;
                }
            }
            child = self.next_sibling(c);
        }

        if let Some(tid) = text_node_id {
            let data = self.arena.get_mut(tid).ok_or(XmlError::InvalidNodeId)?;
            if let NodeKind::Text(txt_data) = &mut data.kind {
                txt_data.content = text.to_string();
            }
        } else {
            let new_txt = self.new_text(text);
            self.insert_first_child(el, new_txt)?;
        }
        Ok(())
    }

    /// Queries the text content of the first child Text node as an `i32`.
    pub fn query_int_text(&self, el: NodeId) -> Result<i32> {
        let val = self.get_text(el).ok_or(XmlError::NoTextNode)?;
        parse_numeric_text(val)
    }

    /// Queries the text content of the first child Text node as a `bool`.
    pub fn query_bool_text(&self, el: NodeId) -> Result<bool> {
        let val = self.get_text(el).ok_or(XmlError::NoTextNode)?;
        parse_bool_text(val)
    }

    /// Queries the text content of the first child Text node as an `f64`.
    pub fn query_double_text(&self, el: NodeId) -> Result<f64> {
        let val = self.get_text(el).ok_or(XmlError::NoTextNode)?;
        parse_numeric_text(val)
    }

    /// Returns the text content of the first child Text node as an `i32`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn int_text(&self, el: NodeId, default: i32) -> i32 {
        self.query_int_text(el).unwrap_or(default)
    }

    /// Returns the text content of the first child Text node as a `bool`, or `default` if it does not exist or cannot be parsed.
    #[must_use]
    pub fn bool_text(&self, el: NodeId, default: bool) -> bool {
        self.query_bool_text(el).unwrap_or(default)
    }
}
