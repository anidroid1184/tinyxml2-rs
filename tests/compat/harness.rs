#![allow(clippy::unnecessary_debug_formatting)]
mod cpp_runner;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tinyxml2::{Document, NodeId, NodeKind, ParseOptions, Whitespace};

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct CanonicalNode {
    #[serde(rename = "type")]
    kind: String,
    value: Option<String>,
    line: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    attributes: Option<Vec<CanonicalAttr>>,
    children: Vec<CanonicalNode>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct CanonicalAttr {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct CanonicalError {
    #[serde(default)]
    error: bool,
    code: i32,
    name: String,
    line: u32,
}

fn to_canonical_node(doc: &Document, node_id: NodeId) -> CanonicalNode {
    let kind = doc.node_kind(node_id).expect("Node must exist");
    let line = doc.line_num(node_id).unwrap_or(0);

    let (kind_str, value_str) = match kind {
        NodeKind::Document => ("document".to_string(), None),
        NodeKind::Element(el) => ("element".to_string(), Some(el.name.clone())),
        NodeKind::Text(txt) => {
            let k = if txt.is_cdata { "cdata" } else { "text" };
            (k.to_string(), Some(txt.content.clone()))
        }
        NodeKind::Comment(c) => ("comment".to_string(), Some(c.clone())),
        NodeKind::Declaration(d) => ("declaration".to_string(), Some(d.name.clone())),
        NodeKind::Unknown(u) => ("unknown".to_string(), Some(u.clone())),
    };

    let attributes = match kind {
        NodeKind::Element(el) => {
            let mut attrs = Vec::new();
            for attr in &el.attributes {
                attrs.push(CanonicalAttr {
                    name: attr.name.clone(),
                    value: attr.value.clone(),
                });
            }
            Some(attrs)
        }
        _ => None,
    };

    let mut children = Vec::new();
    let mut child = doc.first_child(node_id);
    while let Some(c_id) = child {
        children.push(to_canonical_node(doc, c_id));
        child = doc.next_sibling(c_id);
    }

    CanonicalNode {
        kind: kind_str,
        value: value_str,
        line,
        attributes,
        children,
    }
}

fn get_all_xml_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "xml") {
                files.push(path);
            }
        }
    }
    // Sort to make output deterministic
    files.sort();
    files
}

fn are_errors_equivalent(rust: &str, cpp: &str) -> bool {
    if rust == cpp {
        return true;
    }
    let r_parse = rust.starts_with("XML_ERROR_PARSING") || rust == "XML_ERROR_MISMATCHED_ELEMENT";
    let c_parse = cpp.starts_with("XML_ERROR_PARSING") || cpp == "XML_ERROR_MISMATCHED_ELEMENT";
    r_parse && c_parse
}

fn run_differential_test(xml_path: &Path, whitespace: Whitespace, max_depth: Option<u32>) {
    let ws_str = match whitespace {
        Whitespace::Preserve => "preserve",
        Whitespace::Collapse => "collapse",
        Whitespace::Pedantic => "pedantic",
    };

    let mut opts = ParseOptions::new().with_whitespace(whitespace);
    if let Some(depth) = max_depth {
        opts = opts.with_max_depth(depth);
    }
    let mut doc = Document::with_options(opts);
    println!("Testing: {xml_path:?} (ws: {ws_str})");

    let content = fs::read_to_string(xml_path)
        .unwrap_or_else(|_| panic!("Failed to read XML file {xml_path:?}"));
    // Normalize CRLF to LF to match C++ TinyXML2's internal newline normalization
    // and prevent Windows checkout platform-specific divergences.
    let normalized_content = content.replace("\r\n", "\n").replace('\r', "\n");

    let rust_res = doc.parse_str(&normalized_content);

    // Call C++ reference runner
    let cpp_json_str = cpp_runner::run_cpp_reference(xml_path, ws_str);

    match rust_res {
        Ok(()) => {
            // Rust parsing succeeded, C++ must also succeed and produce the same DOM tree
            let rust_tree = to_canonical_node(&doc, doc.root());

            let mut deserializer = serde_json::Deserializer::from_str(&cpp_json_str);
            deserializer.disable_recursion_limit();
            let cpp_tree = CanonicalNode::deserialize(&mut deserializer)
                .unwrap_or_else(|e| panic!(
                    "Failed to parse C++ JSON output as tree for {xml_path:?} (ws_mode: {ws_str}). Error: {e}.\nJSON:\n{cpp_json_str}"
                ));

            assert_eq!(
                rust_tree, cpp_tree,
                "DOM mismatch on {xml_path:?} with whitespace mode {ws_str}"
            );
        }
        Err(rust_err) => {
            // If Rust failed due to depth limits, allow C++ to either fail or succeed (no enforced depth limit in C++ default)
            if rust_err.name() == "XML_ELEMENT_DEPTH_EXCEEDED" {
                return;
            }

            // Rust parsing failed, C++ must also fail
            assert!(
                cpp_json_str.trim().starts_with("{\"error\":"),
                "Rust failed with error {rust_err:?} but C++ succeeded on {xml_path:?} (ws_mode: {ws_str}).\nC++ Output:\n{cpp_json_str}"
            );

            let cpp_error: CanonicalError = serde_json::from_str(&cpp_json_str)
                .unwrap_or_else(|e| panic!(
                    "Failed to parse C++ JSON output as error for {xml_path:?} (ws_mode: {ws_str}). Error: {e}.\nJSON:\n{cpp_json_str}"
                ));

            // Compare error names/kinds
            let rust_err_name = rust_err.name();
            let cpp_err_name = cpp_error.name.as_str();

            assert!(
                are_errors_equivalent(rust_err_name, cpp_err_name),
                "Error name mismatch on {:?} (ws_mode: {}). Rust: {}, C++: {}",
                xml_path,
                ws_str,
                rust_err_name,
                cpp_error.name
            );

            // Compare error line numbers
            let rust_line = rust_err.line().unwrap_or(0);
            assert_eq!(
                rust_line, cpp_error.line,
                "Error line mismatch on {:?} (ws_mode: {}). Rust: {}, C++: {}",
                xml_path, ws_str, rust_line, cpp_error.line
            );
        }
    }
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        if dir.join("tests").join("corpus").exists() {
            return Some(dir);
        }
        {
            let parent = dir.parent()?;
            dir = parent.to_path_buf();
        }
    }
}

#[test]
fn test_valid_corpus_differential() {
    let Some(root) = find_workspace_root() else {
        println!(
            "Skipping test_valid_corpus_differential: tests/corpus not found (e.g. cargo publish)"
        );
        return;
    };
    let files = get_all_xml_files(&root.join("tests/corpus/valid"));
    assert!(!files.is_empty(), "No files found in tests/corpus/valid");
    for file in &files {
        run_differential_test(file, Whitespace::Preserve, Some(150));
        run_differential_test(file, Whitespace::Collapse, Some(150));
        run_differential_test(file, Whitespace::Pedantic, Some(150));
    }
}

#[test]
fn test_invalid_corpus_differential() {
    let Some(root) = find_workspace_root() else {
        println!(
            "Skipping test_invalid_corpus_differential: tests/corpus not found (e.g. cargo publish)"
        );
        return;
    };
    let files = get_all_xml_files(&root.join("tests/corpus/invalid"));
    assert!(!files.is_empty(), "No files found in tests/corpus/invalid");
    for file in &files {
        // Run with Preserve (which is the default) and a depth limit of 80 to prevent stack overflows
        run_differential_test(file, Whitespace::Preserve, Some(80));
    }
}

#[test]
fn test_unicode_corpus_differential() {
    let Some(root) = find_workspace_root() else {
        println!(
            "Skipping test_unicode_corpus_differential: tests/corpus not found (e.g. cargo publish)"
        );
        return;
    };
    let files = get_all_xml_files(&root.join("tests/corpus/unicode"));
    assert!(!files.is_empty(), "No files found in tests/corpus/unicode");
    for file in &files {
        run_differential_test(file, Whitespace::Preserve, Some(150));
        run_differential_test(file, Whitespace::Collapse, Some(150));
    }
}

// ============================================================
// Property-Based Tests (proptest)
// ============================================================

use proptest::prelude::*;

proptest! {
    // 1. Entity round-tripping: decode(encode(s)) == s
    #[test]
    fn prop_entity_roundtrip(s in "\\PC*") {
        let encoded_text = tinyxml2::entity::encode_text(&s);
        let decoded_text = tinyxml2::entity::decode(&encoded_text);
        prop_assert_eq!(&s, &decoded_text);

        let encoded_attr = tinyxml2::entity::encode_attribute(&s);
        let decoded_attr = tinyxml2::entity::decode(&encoded_attr);
        prop_assert_eq!(&s, &decoded_attr);
    }

    // 2. Navigation Consistency: parent(first_child(n)) == n
    #[test]
    fn prop_navigation_consistency(xml in "<root><child><grandchild/></child><child2 attr=\"val\"/></root>") {
        let doc = Document::parse(&xml).unwrap();
        let root = doc.root();

        let mut first = doc.first_child(root);
        while let Some(child) = first {
            let parent = doc.parent(child).unwrap();
            prop_assert_eq!(parent, root);

            let mut g_first = doc.first_child(child);
            while let Some(g_child) = g_first {
                let g_parent = doc.parent(g_child).unwrap();
                prop_assert_eq!(g_parent, child);
                g_first = doc.next_sibling(g_child);
            }
            first = doc.next_sibling(child);
        }
    }

    // 3. Clone Independence: mutations to deep_clone(n) don't affect original
    #[test]
    fn prop_clone_independence(xml in "<root><child attr=\"original_val\">original_text</child></root>") {
        let mut doc = Document::parse(&xml).unwrap();
        let root_doc = doc.root();
        let root_el = doc.first_child(root_doc).unwrap();
        let child = doc.first_child(root_el).unwrap();

        // Deep clone
        let cloned_child = doc.deep_clone(child).unwrap();

        // Mutate clone
        doc.set_attribute(cloned_child, "attr", "mutated_val").unwrap();

        // Assert original is unchanged
        let original_attr = doc.attribute(child, "attr").unwrap();
        prop_assert_eq!(original_attr, "original_val");

        let cloned_attr = doc.attribute(cloned_child, "attr").unwrap();
        prop_assert_eq!(cloned_attr, "mutated_val");
    }
}
