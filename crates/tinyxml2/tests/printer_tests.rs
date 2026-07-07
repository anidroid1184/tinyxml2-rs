use tinyxml2::{Document, XmlPrinter};

#[test]
fn test_pretty_print_basic() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let decl = doc.new_declaration("xml version=\"1.0\" encoding=\"UTF-8\"");
    doc.insert_first_child(doc.root(), decl).unwrap();

    let child1 = doc.new_element("child1");
    doc.set_attribute(child1, "attr", "value").unwrap();
    doc.insert_end_child(root, child1).unwrap();

    let gc = doc.new_element("grandchild");
    doc.insert_end_child(child1, gc).unwrap();

    let txt = doc.new_text("Hello");
    doc.insert_end_child(child1, txt).unwrap();

    let comment = doc.new_comment("A comment");
    doc.insert_end_child(root, comment).unwrap();

    let unknown = doc.new_unknown("DOCTYPE html");
    doc.insert_end_child(doc.root(), unknown).unwrap();

    let expected = "\
<?xml version=\"1.0\" encoding=\"UTF-8\"?>
<root>
    <child1 attr=\"value\">
        <grandchild/>
        Hello
    </child1>
    <!--A comment-->
</root>
<!DOCTYPE html>
";

    assert_eq!(doc.to_string(), expected);
}

#[test]
fn test_compact_print_basic() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let child = doc.new_element("child");
    doc.set_attribute(child, "a", "1").unwrap();
    doc.insert_end_child(root, child).unwrap();

    let text = doc.new_text("text");
    doc.insert_end_child(child, text).unwrap();

    let expected = "<root><child a=\"1\">text</child></root>";
    assert_eq!(doc.to_string_compact(), expected);
}

#[test]
fn test_entity_escaping() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let child = doc.new_element("child");
    doc.set_attribute(child, "special", "a < b & c > d \"quotes\" 'apos'")
        .unwrap();
    doc.insert_end_child(root, child).unwrap();

    let text = doc.new_text("escaped: & < > \"quotes\" 'apos'");
    doc.insert_end_child(child, text).unwrap();

    let cdata = doc.new_cdata("raw: & < > \"quotes\" 'apos'");
    doc.insert_end_child(root, cdata).unwrap();

    let expected = "<root><child special=\"a &lt; b &amp; c &gt; d &quot;quotes&quot; &apos;apos&apos;\">escaped: &amp; &lt; &gt; \"quotes\" 'apos'</child><![CDATA[raw: & < > \"quotes\" 'apos']]></root>";
    assert_eq!(doc.to_string_compact(), expected);
}

#[test]
fn test_custom_indentation() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let child = doc.new_element("child");
    doc.insert_end_child(root, child).unwrap();

    let gc = doc.new_element("grandchild");
    doc.insert_end_child(child, gc).unwrap();

    let mut printer = XmlPrinter::new();
    printer.set_indent_str("\t");
    doc.accept(&mut printer);

    let expected = "\
<root>
\t<child>
\t\t<grandchild/>
\t</child>
</root>
";
    assert_eq!(printer.as_str(), expected);
}

#[test]
fn test_bom_output() {
    let mut doc = Document::new();
    doc.set_bom(true);

    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let mut printer = XmlPrinter::new_compact();
    printer.set_bom(true);
    doc.accept(&mut printer);

    let res = printer.into_string();
    assert!(res.starts_with("\u{FEFF}"));
    assert_eq!(res, "\u{FEFF}<root/>");
}

#[test]
fn test_streaming_api() {
    let mut printer = XmlPrinter::new();
    printer.push_header("1.0", Some("UTF-8"), Some(true));
    printer.open_element("root");
    printer.push_attribute("version", "2");
    printer.open_element("child");
    printer.push_text("Hello");
    printer.close_element(); // child
    printer.push_comment("A comment");
    printer.push_cdata("raw & text");
    printer.push_unknown("UNKNOWN");
    printer.push_declaration("xml-stylesheet href=\"style.css\"");
    printer.close_element(); // root

    let expected = "\
<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>
<root version=\"2\">
    <child>Hello</child>
    <!--A comment-->
    <![CDATA[raw & text]]>
    <!UNKNOWN>
    <?xml-stylesheet href=\"style.css\"?>
</root>
";

    assert_eq!(printer.as_str(), expected);
}

#[test]
#[cfg(feature = "std")]
fn test_save_writer() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let mut buf = Vec::new();
    doc.save_writer(&mut buf).unwrap();
    assert_eq!(String::from_utf8(buf).unwrap(), "<root/>\n");

    let mut buf_compact = Vec::new();
    doc.save_writer_compact(&mut buf_compact).unwrap();
    assert_eq!(String::from_utf8(buf_compact).unwrap(), "<root/>");
}

#[test]
fn test_display_trait() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    let formatted = format!("{doc}");
    assert_eq!(formatted, "<root/>\n");
}

#[test]
fn test_complex_and_simple_elements() {
    let mut doc = Document::new();
    let root = doc.new_element("root");
    doc.insert_end_child(doc.root(), root).unwrap();

    // Text-only: should be single line
    let el1 = doc.new_element("text-only");
    let txt = doc.new_text("Hello World");
    doc.insert_end_child(el1, txt).unwrap();
    doc.insert_end_child(root, el1).unwrap();

    // Nested: should be multiline pretty-printed
    let el2 = doc.new_element("nested");
    let inner = doc.new_element("inner");
    doc.insert_end_child(el2, inner).unwrap();
    doc.insert_end_child(root, el2).unwrap();

    // Mixed content
    let el3 = doc.new_element("mixed");
    let txt1 = doc.new_text("Before ");
    let inner3 = doc.new_element("inner");
    let txt2 = doc.new_text(" After");
    doc.insert_end_child(el3, txt1).unwrap();
    doc.insert_end_child(el3, inner3).unwrap();
    doc.insert_end_child(el3, txt2).unwrap();
    doc.insert_end_child(root, el3).unwrap();

    let expected = "\
<root>
    <text-only>Hello World</text-only>
    <nested>
        <inner/>
    </nested>
    <mixed>
        Before 
        <inner/>
         After
    </mixed>
</root>
";

    assert_eq!(doc.to_string(), expected);
}
