#[cfg(feature = "std")]
use std::fs;
use tinyxml2::{Document, NodeKind, ParseOptions, Result, Whitespace, XmlError};

#[test]
fn test_basic_parsing() -> Result<()> {
    let xml = r#"<root attr="value"><child>hello</child></root>"#;
    let doc = Document::parse(xml)?;

    let root = doc.root_element().unwrap();
    assert_eq!(doc.line_num(root), Some(1));

    if let Some(NodeKind::Element(el)) = doc.node_kind(root) {
        assert_eq!(el.name, "root");
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "attr");
        assert_eq!(el.attributes[0].value, "value");
    } else {
        panic!("Root is not an element");
    }

    let child = doc.first_child_element(root, None).unwrap();
    if let Some(NodeKind::Element(el)) = doc.node_kind(child) {
        assert_eq!(el.name, "child");
    } else {
        panic!("Child is not an element");
    }

    assert_eq!(doc.get_text(child), Some("hello"));
    Ok(())
}

#[test]
fn test_self_closing_and_siblings() -> Result<()> {
    let xml = r#"<root><sibling1/><sibling2 attr="val"/></root>"#;
    let doc = Document::parse(xml)?;

    let root = doc.root_element().unwrap();
    let sib1 = doc.first_child_element(root, None).unwrap();
    let sib2 = doc.next_sibling_element(sib1, None).unwrap();

    if let Some(NodeKind::Element(el)) = doc.node_kind(sib1) {
        assert_eq!(el.name, "sibling1");
        assert!(el.attributes.is_empty());
    } else {
        panic!("sib1 is not an element");
    }

    if let Some(NodeKind::Element(el)) = doc.node_kind(sib2) {
        assert_eq!(el.name, "sibling2");
        assert_eq!(el.attributes.len(), 1);
        assert_eq!(el.attributes[0].name, "attr");
        assert_eq!(el.attributes[0].value, "val");
    } else {
        panic!("sib2 is not an element");
    }

    assert_eq!(doc.next_sibling_element(sib2, None), None);
    Ok(())
}

#[test]
fn test_comments_and_declarations() -> Result<()> {
    let xml =
        r#"<?xml version="1.0" encoding="UTF-8"?><!-- comment --><root><!DOCTYPE html></root>"#;
    let doc = Document::parse(xml)?;

    let root_node = doc.root();
    let decl = doc.first_child(root_node).unwrap();
    let comment = doc.next_sibling(decl).unwrap();
    let root_el = doc.next_sibling(comment).unwrap();

    if let Some(NodeKind::Declaration(data)) = doc.node_kind(decl) {
        assert_eq!(data.name, "xml version=\"1.0\" encoding=\"UTF-8\"");
    } else {
        panic!("decl is not a declaration");
    }

    if let Some(NodeKind::Comment(text)) = doc.node_kind(comment) {
        assert_eq!(text, " comment ");
    } else {
        panic!("comment is not a comment");
    }

    if let Some(NodeKind::Element(data)) = doc.node_kind(root_el) {
        assert_eq!(data.name, "root");
    } else {
        panic!("root_el is not an element");
    }

    let doctype = doc.first_child(root_el).unwrap();
    if let Some(NodeKind::Unknown(text)) = doc.node_kind(doctype) {
        assert_eq!(text, "DOCTYPE html");
    } else {
        panic!("doctype is not an unknown node");
    }

    Ok(())
}

#[test]
fn test_cdata_section() -> Result<()> {
    let xml = r"<root><![CDATA[some raw <text> &amp; stuff]]></root>";
    let doc = Document::parse(xml)?;

    let root = doc.root_element().unwrap();
    let text_node = doc.first_child(root).unwrap();

    if let Some(NodeKind::Text(data)) = doc.node_kind(text_node) {
        assert!(data.is_cdata);
        assert_eq!(data.content, "some raw <text> &amp; stuff");
    } else {
        panic!("text_node is not CDATA");
    }

    Ok(())
}

#[test]
fn test_entity_resolution() -> Result<()> {
    let xml = r#"<root attr="&amp; &lt; &gt; &quot; &apos; &#65; &#x41;">hello &amp; &lt; &gt; &quot; &apos; &#65; &#x41;</root>"#;
    let doc = Document::parse(xml)?;

    let root = doc.root_element().unwrap();
    assert_eq!(doc.attribute(root, "attr"), Some("& < > \" ' A A"));
    assert_eq!(doc.get_text(root), Some("hello & < > \" ' A A"));

    // process_entities = false
    let opts = ParseOptions::new().with_process_entities(false);
    let mut doc2 = Document::with_options(opts);
    doc2.parse_str(xml)?;
    let root2 = doc2.root_element().unwrap();
    // Predefined entities are NOT resolved, but numeric entities ARE
    assert_eq!(
        doc2.attribute(root2, "attr"),
        Some("&amp; &lt; &gt; &quot; &apos; A A")
    );
    assert_eq!(
        doc2.get_text(root2),
        Some("hello &amp; &lt; &gt; &quot; &apos; A A")
    );

    Ok(())
}

#[test]
fn test_whitespace_modes() -> Result<()> {
    let xml = "<root>  hello \t \n  world  \r\n</root>";

    // Preserve mode
    let opts = ParseOptions::new().with_whitespace(Whitespace::Preserve);
    let mut doc = Document::with_options(opts);
    doc.parse_str(xml)?;
    let root = doc.root_element().unwrap();
    assert_eq!(doc.get_text(root), Some("  hello \t \n  world  \r\n"));

    // Collapse mode
    let opts = ParseOptions::new().with_whitespace(Whitespace::Collapse);
    let mut doc = Document::with_options(opts);
    doc.parse_str(xml)?;
    let root = doc.root_element().unwrap();
    assert_eq!(doc.get_text(root), Some("hello world"));

    // Pedantic mode
    let opts = ParseOptions::new().with_whitespace(Whitespace::Pedantic);
    let mut doc = Document::with_options(opts);
    doc.parse_str(xml)?;
    let root = doc.root_element().unwrap();
    assert_eq!(doc.get_text(root), Some("  hello \t \n  world  \n"));

    Ok(())
}

#[test]
fn test_whitespace_only_text_nodes() -> Result<()> {
    let xml = "<root>   </root>";

    // Preserve mode (discards whitespace-only text nodes)
    let doc = Document::parse(xml)?;
    let root = doc.root_element().unwrap();
    assert_eq!(doc.first_child(root), None);

    // Collapse mode (discards whitespace-only text nodes)
    let opts = ParseOptions::new().with_whitespace(Whitespace::Collapse);
    let mut doc = Document::with_options(opts);
    doc.parse_str(xml)?;
    let root = doc.root_element().unwrap();
    assert_eq!(doc.first_child(root), None);

    // Pedantic mode (keeps whitespace-only text nodes)
    let opts = ParseOptions::new().with_whitespace(Whitespace::Pedantic);
    let mut doc = Document::with_options(opts);
    doc.parse_str(xml)?;
    let root = doc.root_element().unwrap();
    let text = doc.first_child(root).unwrap();
    if let Some(NodeKind::Text(data)) = doc.node_kind(text) {
        assert_eq!(data.content, "   ");
    } else {
        panic!("Expected whitespace text node");
    }

    Ok(())
}

#[test]
fn test_bom_handling() -> Result<()> {
    // UTF-8 BOM is \xEF\xBB\xBF
    let xml_bytes = b"\xEF\xBB\xBF<root/>";
    let doc = Document::parse_bytes(xml_bytes)?;
    assert!(doc.has_bom());

    let doc2 = Document::parse("<root/>")?;
    assert!(!doc2.has_bom());

    Ok(())
}

#[test]
fn test_error_handling() {
    // Empty document
    let res = Document::parse("");
    assert!(matches!(res, Err(XmlError::EmptyDocument)));

    // Stray closing tag
    let res = Document::parse("</root>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Element,
            ..
        })
    ));

    // Multiple root elements
    let res = Document::parse("<root1/><root2/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Element,
            ..
        })
    ));

    // Text content outside root
    let res = Document::parse("hello <root/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Text,
            ..
        })
    ));

    // Mismatched closing tag
    let res = Document::parse("<root><child></root>");
    assert!(matches!(
        res,
        Err(XmlError::MismatchedElement {
            line: 1,
            expected,
            found
        }) if expected == "child" && found == "root"
    ));

    // Duplicate attribute names
    let res = Document::parse("<root a=\"1\" a=\"2\"/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Attribute,
            ..
        })
    ));

    // Unclosed attribute value
    let res = Document::parse("<root a=\"1/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Attribute,
            ..
        })
    ));

    // Unclosed comment
    let res = Document::parse("<root><!-- comment</root>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Comment,
            ..
        })
    ));

    // Unclosed CDATA
    let res = Document::parse("<root><![CDATA[cdata</root>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Cdata,
            ..
        })
    ));

    // Unclosed declaration
    let res = Document::parse("<?xml version=\"1.0\"<root/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Declaration,
            ..
        })
    ));

    // Unclosed unknown
    let res = Document::parse("<!DOCTYPE html <root/>");
    assert!(matches!(
        res,
        Err(XmlError::Parse {
            kind: tinyxml2::ParseErrorKind::Unknown,
            ..
        })
    ));
}

#[test]
fn test_depth_limiting() {
    // Max depth of 3
    let opts = ParseOptions::new().with_max_depth(3);

    // Depth 3 (root -> level2 -> level3) - OK
    let xml_ok = "<a><b><c/></b></a>";
    let mut doc_ok = Document::with_options(opts.clone());
    assert!(doc_ok.parse_str(xml_ok).is_ok());

    // Depth 4 - Fail
    let xml_fail = "<a><b><c><d/></c></b></a>";
    let mut doc_fail = Document::with_options(opts);
    let res = doc_fail.parse_str(xml_fail);
    assert!(matches!(
        res,
        Err(XmlError::ElementDepthExceeded {
            line: 1,
            max_depth: 3
        })
    ));
}

#[test]
fn test_invalid_utf8_rejection() {
    let bytes = b"<root attr=\xFF/>";
    let res = Document::parse_bytes(bytes);
    assert!(res.is_err());
}

#[test]
#[cfg(feature = "std")]
fn test_file_loading() -> Result<()> {
    let xml = "<root><child>file data</child></root>";
    let path = "test_temp_parser.xml";
    fs::write(path, xml).unwrap();

    let doc = Document::load_file(path)?;
    let root = doc.root_element().unwrap();
    let child = doc.first_child_element(root, None).unwrap();
    assert_eq!(doc.get_text(child), Some("file data"));

    fs::remove_file(path).unwrap();
    Ok(())
}
