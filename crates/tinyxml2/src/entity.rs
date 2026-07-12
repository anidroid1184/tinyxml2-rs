//! XML entity encoding and decoding.
//!
//! Handles the five predefined XML entities and numeric character references:
//!
//! | Entity | Character | Code Point |
//! |--------|-----------|------------|
//! | `&amp;` | `&` | U+0026 |
//! | `&lt;` | `<` | U+003C |
//! | `&gt;` | `>` | U+003E |
//! | `&quot;` | `"` | U+0022 |
//! | `&apos;` | `'` | U+0027 |
//!
//! Numeric character references (`&#123;` and `&#x7B;`) are also supported
//! for decoding.
//!
//! # Compatibility with TinyXML2
//!
//! Entity handling matches TinyXML2 exactly:
//! - Only the 5 predefined XML entities are recognized as named entities
//! - Numeric references support both decimal (`&#N;`) and hexadecimal (`&#xN;`)
//! - Invalid numeric references (e.g., `&#0;`, `&#x110000;`) are left as-is
//! - Entity processing can be disabled via `ParseOptions::process_entities`

use alloc::borrow::Cow;
use alloc::string::String;
use core::str;

/// Decodes XML entities in a string, replacing entity references with their
/// corresponding characters.
///
/// Handles:
/// - Named entities: `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`
/// - Decimal numeric references: `&#123;`
/// - Hexadecimal numeric references: `&#x7B;`
///
/// Invalid or unrecognized entities are left unchanged in the output.
///
/// # Examples
///
/// ```
/// use tinyxml2::entity::decode;
///
/// assert_eq!(decode("Hello &amp; World"), "Hello & World");
/// assert_eq!(decode("a &lt; b &gt; c"), "a < b > c");
/// assert_eq!(decode("&#65;"), "A");
/// assert_eq!(decode("&#x41;"), "A");
/// assert_eq!(decode("&unknown;"), "&unknown;");
/// ```
pub fn decode(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars = input.as_bytes();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == b'&' {
            if let Some((decoded_char, consumed)) = try_decode_entity(&chars[i..]) {
                output.push(decoded_char);
                i += consumed;
                continue;
            }
        }
        let remaining = &input[i..];
        let ch = remaining.chars().next().unwrap();
        output.push(ch);
        i += ch.len_utf8();
    }

    output
}

/// Decodes entities in place, returning a `Cow` that avoids allocation when
/// no entities are present.
///
/// This is the preferred decoding function for performance-sensitive paths.
///
/// # Examples
///
/// ```
/// use tinyxml2::entity::decode_cow;
/// use std::borrow::Cow;
///
/// // No entities — returns borrowed reference (no allocation)
/// let result = decode_cow("Hello World");
/// assert!(matches!(result, Cow::Borrowed(_)));
///
/// // With entities — returns owned string
/// let result = decode_cow("a &amp; b");
/// assert_eq!(result, "a & b");
/// ```
pub fn decode_cow(input: &str) -> Cow<'_, str> {
    if !input.contains('&') {
        return Cow::Borrowed(input);
    }
    Cow::Owned(decode(input))
}

/// Decodes ONLY numeric character references in a string, leaving named entities (like `&amp;`) as-is.
///
/// This is used when entity processing is disabled (`ParseOptions::process_entities = false`),
/// in which case TinyXML2 still decodes numeric character references.
#[must_use]
pub fn decode_numeric_only(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let chars = input.as_bytes();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == b'&' {
            if let Some((decoded_char, consumed)) = try_decode_numeric_entity(&chars[i..]) {
                output.push(decoded_char);
                i += consumed;
                continue;
            }
        }
        let remaining = &input[i..];
        let ch = remaining.chars().next().unwrap();
        output.push(ch);
        i += ch.len_utf8();
    }

    output
}

/// Attempts to decode a single numeric entity reference starting at the given byte slice.
fn try_decode_numeric_entity(bytes: &[u8]) -> Option<(char, usize)> {
    debug_assert_eq!(bytes[0], b'&');

    // Find the semicolon
    let semi_pos = bytes.iter().position(|&b| b == b';')?;

    // Must have at least `&#N;` (4 bytes)
    if semi_pos < 3 {
        return None;
    }

    let entity_body = &bytes[1..semi_pos];
    let consumed = semi_pos + 1;

    // Check numeric references only
    if entity_body.first() == Some(&b'#') {
        let num_body = &entity_body[1..];
        let code_point = if num_body.first() == Some(&b'x') || num_body.first() == Some(&b'X') {
            // Hexadecimal: &#xNN;
            let hex_str = str::from_utf8(&num_body[1..]).ok()?;
            u32::from_str_radix(hex_str, 16).ok()?
        } else {
            // Decimal: &#NN;
            let dec_str = str::from_utf8(num_body).ok()?;
            dec_str.parse::<u32>().ok()?
        };

        // Validate: must be a valid Unicode scalar value and not U+0000
        if code_point == 0 {
            return None;
        }
        let c = char::from_u32(code_point)?;
        return Some((c, consumed));
    }

    None
}

/// Attempts to decode a single entity reference starting at the given byte slice.
///
/// Returns `Some((char, bytes_consumed))` if a valid entity was decoded,
/// or `None` if the input does not start with a valid entity reference.
fn try_decode_entity(bytes: &[u8]) -> Option<(char, usize)> {
    debug_assert_eq!(bytes[0], b'&');

    // Find the semicolon
    let semi_pos = bytes.iter().position(|&b| b == b';')?;

    // Must have at least `&X;` (3 bytes)
    if semi_pos < 2 {
        return None;
    }

    let entity_body = &bytes[1..semi_pos];
    let consumed = semi_pos + 1;

    // Check named entities first
    match entity_body {
        b"amp" => return Some(('&', consumed)),
        b"lt" => return Some(('<', consumed)),
        b"gt" => return Some(('>', consumed)),
        b"quot" => return Some(('"', consumed)),
        b"apos" => return Some(('\'', consumed)),
        _ => {}
    }

    // Check numeric references
    try_decode_numeric_entity(bytes)
}

/// Encodes special XML characters in text content.
///
/// Replaces `&`, `<`, `>` with their entity equivalents. This is the encoding
/// used for element text content (not attribute values).
///
/// # Examples
///
/// ```
/// use tinyxml2::entity::encode_text;
///
/// assert_eq!(encode_text("a < b & c > d"), "a &lt; b &amp; c &gt; d");
/// assert_eq!(encode_text("no special chars"), "no special chars");
/// ```
pub fn encode_text(input: &str) -> Cow<'_, str> {
    if !input.contains(['&', '<', '>']) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len() + input.len() / 8);
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            other => output.push(other),
        }
    }
    Cow::Owned(output)
}

/// Encodes special XML characters in attribute values.
///
/// Replaces `&`, `<`, `>`, `"`, and `'` with their entity equivalents.
/// Attribute values require more escaping than text content because they
/// appear inside quoted strings.
///
/// # Examples
///
/// ```
/// use tinyxml2::entity::encode_attribute;
///
/// assert_eq!(
///     encode_attribute("value with \"quotes\" & 'apostrophes'"),
///     "value with &quot;quotes&quot; &amp; &apos;apostrophes&apos;"
/// );
/// ```
pub fn encode_attribute(input: &str) -> Cow<'_, str> {
    if !input.contains(['&', '<', '>', '"', '\'']) {
        return Cow::Borrowed(input);
    }

    let mut output = String::with_capacity(input.len() + input.len() / 4);
    for ch in input.chars() {
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            other => output.push(other),
        }
    }
    Cow::Owned(output)
}

#[cfg(test)]
mod tests {
    use alloc::borrow::Cow;

    use super::*;

    // ---- Decode tests ----

    #[test]
    fn decode_named_entities() {
        assert_eq!(decode("&amp;"), "&");
        assert_eq!(decode("&lt;"), "<");
        assert_eq!(decode("&gt;"), ">");
        assert_eq!(decode("&quot;"), "\"");
        assert_eq!(decode("&apos;"), "'");
    }

    #[test]
    fn decode_multiple_entities() {
        assert_eq!(decode("a &amp; b &lt; c"), "a & b < c");
        assert_eq!(decode("&lt;&gt;&amp;"), "<>&");
    }

    #[test]
    fn decode_no_entities() {
        assert_eq!(decode("hello world"), "hello world");
        assert_eq!(decode(""), "");
    }

    #[test]
    fn decode_decimal_numeric() {
        assert_eq!(decode("&#65;"), "A");
        assert_eq!(decode("&#97;"), "a");
        assert_eq!(decode("&#8364;"), "€");
        assert_eq!(decode("&#128512;"), "😀");
    }

    #[test]
    fn decode_hex_numeric() {
        assert_eq!(decode("&#x41;"), "A");
        assert_eq!(decode("&#x61;"), "a");
        assert_eq!(decode("&#x20AC;"), "€");
        assert_eq!(decode("&#x1F600;"), "😀");
        // Uppercase X
        assert_eq!(decode("&#X41;"), "A");
    }

    #[test]
    fn decode_invalid_numeric_left_unchanged() {
        // U+0000 is not a valid XML character
        assert_eq!(decode("&#0;"), "&#0;");
        // Beyond Unicode range
        assert_eq!(decode("&#x110000;"), "&#x110000;");
        // Empty numeric
        assert_eq!(decode("&#;"), "&#;");
    }

    #[test]
    fn decode_unknown_entity_left_unchanged() {
        assert_eq!(decode("&unknown;"), "&unknown;");
        assert_eq!(decode("&nbsp;"), "&nbsp;");
    }

    #[test]
    fn decode_ampersand_without_semicolon() {
        assert_eq!(decode("a & b"), "a & b");
        assert_eq!(decode("&"), "&");
        assert_eq!(decode("&&"), "&&");
    }

    #[test]
    fn decode_nested_entity_reference() {
        // &amp;amp; should decode to &amp; (one level of decoding)
        assert_eq!(decode("&amp;amp;"), "&amp;");
    }

    #[test]
    fn decode_entity_at_end() {
        assert_eq!(decode("text&amp;"), "text&");
    }

    #[test]
    fn decode_cow_borrows_when_no_entities() {
        let result = decode_cow("no entities here");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn decode_cow_owns_when_entities_present() {
        let result = decode_cow("has &amp; entity");
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(result, "has & entity");
    }

    // ---- Encode tests ----

    #[test]
    fn encode_text_special_chars() {
        assert_eq!(encode_text("a & b"), "a &amp; b");
        assert_eq!(encode_text("a < b"), "a &lt; b");
        assert_eq!(encode_text("a > b"), "a &gt; b");
        assert_eq!(encode_text("a < b & c > d"), "a &lt; b &amp; c &gt; d");
    }

    #[test]
    fn encode_text_no_special_chars() {
        let result = encode_text("hello world");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn encode_text_preserves_quotes() {
        // Text content does NOT need to escape quotes
        assert_eq!(encode_text("he said \"hello\""), "he said \"hello\"");
    }

    #[test]
    fn encode_attribute_all_special_chars() {
        assert_eq!(encode_attribute("&"), "&amp;");
        assert_eq!(encode_attribute("<"), "&lt;");
        assert_eq!(encode_attribute(">"), "&gt;");
        assert_eq!(encode_attribute("\""), "&quot;");
        assert_eq!(encode_attribute("'"), "&apos;");
    }

    #[test]
    fn encode_attribute_no_special_chars() {
        let result = encode_attribute("hello world");
        assert!(matches!(result, Cow::Borrowed(_)));
    }

    #[test]
    fn encode_attribute_mixed() {
        assert_eq!(
            encode_attribute("val=\"a & b\""),
            "val=&quot;a &amp; b&quot;"
        );
    }

    // ---- Round-trip tests ----

    #[test]
    fn roundtrip_text() {
        let original = "a < b & c > d";
        let encoded = encode_text(original);
        let decoded = decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_attribute() {
        let original = "he said \"it's < 5 & > 3\"";
        let encoded = encode_attribute(original);
        let decoded = decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn roundtrip_empty() {
        assert_eq!(decode(&encode_text("")), "");
        assert_eq!(decode(&encode_attribute("")), "");
    }

    #[test]
    fn roundtrip_no_special_chars() {
        let s = "hello world 12345";
        assert_eq!(decode(&encode_text(s)), s);
        assert_eq!(decode(&encode_attribute(s)), s);
    }

    #[test]
    fn roundtrip_all_entities() {
        let original = "&<>\"'";
        let encoded = encode_attribute(original);
        assert_eq!(encoded, "&amp;&lt;&gt;&quot;&apos;");
        let decoded = decode(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn decode_numeric_only_behavior() {
        assert_eq!(decode_numeric_only("a &amp; b"), "a &amp; b");
        assert_eq!(decode_numeric_only("a &#65; b"), "a A b");
        assert_eq!(decode_numeric_only("a &#x41; b"), "a A b");
        assert_eq!(decode_numeric_only("a &#x1F600; &amp;"), "a 😀 &amp;");
    }
}
