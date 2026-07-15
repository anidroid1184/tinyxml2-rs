//! Parses malformed XML strings and demonstrates how to handle different
//! [`XmlError`] variants with pattern matching.

use tinyxml2::{Document, XmlError};

fn main() {
    try_parse("<root><child></root>", "mismatched tags");
    try_parse("", "empty document");
    try_parse("<root>", "unclosed tag");
}

fn try_parse(xml: &str, desc: &str) {
    match Document::parse(xml) {
        Ok(_doc) => {
            // Successful parse
        }
        Err(e) => {
            println!("--- {desc} ---");
            println!("  error:  {e}");
            match &e {
                XmlError::EmptyDocument => {
                    println!("  cause:  the input contains no XML content");
                }
                XmlError::MismatchedElement {
                    expected,
                    found,
                    line,
                } => {
                    println!("  cause:  expected </{expected}>, found </{found}> at line {line}");
                }
                XmlError::Parse {
                    kind,
                    line,
                    message,
                } => {
                    println!("  cause:  {kind:?} error at line {line}");
                    if let Some(msg) = message {
                        println!("  detail: {msg}");
                    }
                }
                _ => {
                    println!("  cause:  unexpected error — {e}");
                }
            }
            println!();
        }
    }
}
