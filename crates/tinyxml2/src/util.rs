//! XML utility functions for character classification and string helpers.
//!
//! Provides the low-level character classification routines needed by the parser,
//! matching the XML 1.0 specification's definitions for name characters,
//! whitespace, and UTF-8 handling.
//!
//! # Compatibility with TinyXML2
//!
//! These functions match TinyXML2's `XMLUtil` static methods. TinyXML2 uses a
//! simplified character classification that covers ASCII and common Unicode
//! ranges. Our implementation follows the same practical subset.

use alloc::string::String;

/// Returns `true` if the character is XML whitespace.
///
/// XML 1.0 defines whitespace as: space (0x20), tab (0x09), carriage return
/// (0x0D), and line feed (0x0A).
///
/// # Examples
///
/// ```
/// use tinyxml2::util::is_whitespace;
///
/// assert!(is_whitespace(' '));
/// assert!(is_whitespace('\t'));
/// assert!(is_whitespace('\n'));
/// assert!(is_whitespace('\r'));
/// assert!(!is_whitespace('a'));
/// ```
#[inline]
#[must_use]
pub const fn is_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r')
}

/// Returns `true` if the byte is XML whitespace.
///
/// Byte-oriented version of [`is_whitespace`] for use during scanning.
#[inline]
#[must_use]
pub const fn is_whitespace_byte(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

/// Returns `true` if the character can start an XML name.
///
/// Per XML 1.0, a name start character is: letter, underscore, or colon.
/// We follow TinyXML2's practical definition which covers ASCII letters,
/// underscore, colon, and the common Unicode letter ranges.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::is_name_start_char;
///
/// assert!(is_name_start_char('a'));
/// assert!(is_name_start_char('Z'));
/// assert!(is_name_start_char('_'));
/// assert!(is_name_start_char(':'));
/// assert!(!is_name_start_char('0'));
/// assert!(!is_name_start_char('-'));
/// assert!(!is_name_start_char('.'));
/// ```
#[inline]
#[must_use]
pub const fn is_name_start_char(ch: char) -> bool {
    matches!(ch,
        'A'..='Z'
        | 'a'..='z'
        | '_'
        | ':'
        | '\u{C0}'..='\u{D6}'
        | '\u{D8}'..='\u{F6}'
        | '\u{F8}'..='\u{2FF}'
        | '\u{370}'..='\u{37D}'
        | '\u{37F}'..='\u{1FFF}'
        | '\u{200C}'..='\u{200D}'
        | '\u{2070}'..='\u{218F}'
        | '\u{2C00}'..='\u{2FEF}'
        | '\u{3001}'..='\u{D7FF}'
        | '\u{F900}'..='\u{FDCF}'
        | '\u{FDF0}'..='\u{FFFD}'
        | '\u{10000}'..='\u{EFFFF}'
    )
}

/// Returns `true` if the character can appear in an XML name (after the first
/// character).
///
/// Name characters include name start characters plus digits, hyphen, period,
/// middle dot, and combining/extender characters.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::is_name_char;
///
/// assert!(is_name_char('a'));
/// assert!(is_name_char('0'));
/// assert!(is_name_char('-'));
/// assert!(is_name_char('.'));
/// assert!(!is_name_char(' '));
/// assert!(!is_name_char('<'));
/// ```
#[inline]
#[must_use]
pub const fn is_name_char(ch: char) -> bool {
    is_name_start_char(ch)
        || matches!(ch,
            '0'..='9'
            | '-'
            | '.'
            | '\u{B7}'
            | '\u{0300}'..='\u{036F}'
            | '\u{203F}'..='\u{2040}'
        )
}

/// Skips whitespace characters at the beginning of a string slice.
///
/// Returns the remaining slice after leading whitespace, and the number of
/// newline characters encountered (for line tracking).
///
/// # Examples
///
/// ```
/// use tinyxml2::util::skip_whitespace;
///
/// let (rest, newlines) = skip_whitespace("  \n  hello");
/// assert_eq!(rest, "hello");
/// assert_eq!(newlines, 1);
///
/// let (rest, newlines) = skip_whitespace("no leading space");
/// assert_eq!(rest, "no leading space");
/// assert_eq!(newlines, 0);
/// ```
#[must_use]
pub fn skip_whitespace(input: &str) -> (&str, u32) {
    let mut newlines = 0u32;
    let mut chars = input.char_indices();

    for (i, ch) in &mut chars {
        if !is_whitespace(ch) {
            return (&input[i..], newlines);
        }
        if ch == '\n' {
            newlines += 1;
        }
    }

    // All whitespace
    ("", newlines)
}

/// Collapses whitespace in a string according to TinyXML2's `COLLAPSE_WHITESPACE`
/// mode.
///
/// - Leading and trailing whitespace is removed.
/// - Internal runs of whitespace are collapsed to a single space.
/// - All whitespace characters (including `\n`, `\r`, `\t`) become spaces.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::collapse_whitespace;
///
/// assert_eq!(collapse_whitespace("  hello   world  "), "hello world");
/// assert_eq!(collapse_whitespace("\n\thello\n\tworld\n"), "hello world");
/// assert_eq!(collapse_whitespace("already clean"), "already clean");
/// assert_eq!(collapse_whitespace("   "), "");
/// ```
#[must_use]
pub fn collapse_whitespace(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut in_whitespace = true; // Start true to trim leading whitespace

    for ch in input.chars() {
        if is_whitespace(ch) {
            if !in_whitespace && !result.is_empty() {
                // Only add space if we've already started collecting non-ws chars
                in_whitespace = true;
            } else {
                in_whitespace = true;
            }
        } else {
            if in_whitespace && !result.is_empty() {
                result.push(' ');
            }
            result.push(ch);
            in_whitespace = false;
        }
    }

    result
}

/// Reads an XML name from the beginning of a string slice.
///
/// Returns `Some((name, rest))` if a valid XML name was found at the start,
/// or `None` if the input does not start with a name start character.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::read_name;
///
/// assert_eq!(read_name("element>rest"), Some(("element", ">rest")));
/// assert_eq!(read_name("ns:tag attr"), Some(("ns:tag", " attr")));
/// assert_eq!(read_name("123invalid"), None);
/// assert_eq!(read_name(""), None);
/// ```
#[must_use]
pub fn read_name(input: &str) -> Option<(&str, &str)> {
    let mut chars = input.chars();

    // First character must be a name start character
    let first = chars.next()?;
    if !is_name_start_char(first) {
        return None;
    }

    // Find the end of the name
    let mut end = first.len_utf8();
    for ch in chars {
        if !is_name_char(ch) {
            break;
        }
        end += ch.len_utf8();
    }

    Some((&input[..end], &input[end..]))
}

/// Checks if a string starts with the UTF-8 BOM (byte order mark).
///
/// The UTF-8 BOM is the byte sequence `EF BB BF`.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::starts_with_bom;
///
/// assert!(starts_with_bom("\u{FEFF}hello"));
/// assert!(!starts_with_bom("hello"));
/// assert!(!starts_with_bom(""));
/// ```
#[inline]
#[must_use]
pub fn starts_with_bom(input: &str) -> bool {
    input.as_bytes().starts_with(&[0xEF, 0xBB, 0xBF])
}

/// Strips the UTF-8 BOM from the beginning of a string if present.
///
/// Returns the remaining string and whether a BOM was found.
///
/// # Examples
///
/// ```
/// use tinyxml2::util::strip_bom;
///
/// let (rest, had_bom) = strip_bom("\u{FEFF}hello");
/// assert_eq!(rest, "hello");
/// assert!(had_bom);
///
/// let (rest, had_bom) = strip_bom("hello");
/// assert_eq!(rest, "hello");
/// assert!(!had_bom);
/// ```
#[must_use]
pub fn strip_bom(input: &str) -> (&str, bool) {
    if starts_with_bom(input) {
        (&input[3..], true)
    } else {
        (input, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Whitespace tests ----

    #[test]
    fn whitespace_chars() {
        assert!(is_whitespace(' '));
        assert!(is_whitespace('\t'));
        assert!(is_whitespace('\n'));
        assert!(is_whitespace('\r'));
        assert!(!is_whitespace('a'));
        assert!(!is_whitespace('0'));
        assert!(!is_whitespace('\u{00A0}')); // Non-breaking space is NOT XML whitespace
    }

    #[test]
    fn whitespace_bytes() {
        assert!(is_whitespace_byte(b' '));
        assert!(is_whitespace_byte(b'\t'));
        assert!(is_whitespace_byte(b'\n'));
        assert!(is_whitespace_byte(b'\r'));
        assert!(!is_whitespace_byte(b'a'));
    }

    // ---- Name character tests ----

    #[test]
    fn name_start_ascii() {
        for ch in 'A'..='Z' {
            assert!(is_name_start_char(ch), "Expected '{ch}' to be name start");
        }
        for ch in 'a'..='z' {
            assert!(is_name_start_char(ch), "Expected '{ch}' to be name start");
        }
        assert!(is_name_start_char('_'));
        assert!(is_name_start_char(':'));
    }

    #[test]
    fn name_start_rejects_digits() {
        for ch in '0'..='9' {
            assert!(
                !is_name_start_char(ch),
                "Digit '{ch}' should NOT be name start"
            );
        }
    }

    #[test]
    fn name_start_rejects_special() {
        assert!(!is_name_start_char('-'));
        assert!(!is_name_start_char('.'));
        assert!(!is_name_start_char(' '));
        assert!(!is_name_start_char('<'));
        assert!(!is_name_start_char('>'));
    }

    #[test]
    fn name_start_unicode() {
        // Latin supplement
        assert!(is_name_start_char('\u{C0}')); // À
        assert!(is_name_start_char('\u{D6}')); // Ö
        // CJK
        assert!(is_name_start_char('\u{4E00}')); // 一
    }

    #[test]
    fn name_char_includes_digits_and_hyphen() {
        assert!(is_name_char('0'));
        assert!(is_name_char('9'));
        assert!(is_name_char('-'));
        assert!(is_name_char('.'));
        assert!(is_name_char('\u{B7}')); // Middle dot
    }

    #[test]
    fn name_char_rejects_special() {
        assert!(!is_name_char(' '));
        assert!(!is_name_char('<'));
        assert!(!is_name_char('>'));
        assert!(!is_name_char('='));
        assert!(!is_name_char('"'));
    }

    // ---- Skip whitespace tests ----

    #[test]
    fn skip_whitespace_basic() {
        let (rest, nl) = skip_whitespace("   hello");
        assert_eq!(rest, "hello");
        assert_eq!(nl, 0);
    }

    #[test]
    fn skip_whitespace_with_newlines() {
        let (rest, nl) = skip_whitespace("\n\n  hello");
        assert_eq!(rest, "hello");
        assert_eq!(nl, 2);
    }

    #[test]
    fn skip_whitespace_no_whitespace() {
        let (rest, nl) = skip_whitespace("hello");
        assert_eq!(rest, "hello");
        assert_eq!(nl, 0);
    }

    #[test]
    fn skip_whitespace_all_whitespace() {
        let (rest, nl) = skip_whitespace("  \n\t ");
        assert_eq!(rest, "");
        assert_eq!(nl, 1);
    }

    #[test]
    fn skip_whitespace_empty() {
        let (rest, nl) = skip_whitespace("");
        assert_eq!(rest, "");
        assert_eq!(nl, 0);
    }

    // ---- Collapse whitespace tests ----

    #[test]
    fn collapse_basic() {
        assert_eq!(collapse_whitespace("hello   world"), "hello world");
    }

    #[test]
    fn collapse_leading_trailing() {
        assert_eq!(collapse_whitespace("  hello  "), "hello");
    }

    #[test]
    fn collapse_mixed_whitespace() {
        assert_eq!(collapse_whitespace("\n\thello\n\tworld\n"), "hello world");
    }

    #[test]
    fn collapse_all_whitespace() {
        assert_eq!(collapse_whitespace("   "), "");
    }

    #[test]
    fn collapse_empty() {
        assert_eq!(collapse_whitespace(""), "");
    }

    #[test]
    fn collapse_single_word() {
        assert_eq!(collapse_whitespace("hello"), "hello");
    }

    #[test]
    fn collapse_already_clean() {
        assert_eq!(collapse_whitespace("a b c"), "a b c");
    }

    // ---- Read name tests ----

    #[test]
    fn read_name_simple() {
        assert_eq!(read_name("element>"), Some(("element", ">")));
    }

    #[test]
    fn read_name_with_namespace() {
        assert_eq!(read_name("ns:tag attr"), Some(("ns:tag", " attr")));
    }

    #[test]
    fn read_name_with_digits() {
        assert_eq!(read_name("item123 "), Some(("item123", " ")));
    }

    #[test]
    fn read_name_with_hyphen() {
        assert_eq!(read_name("my-elem/>"), Some(("my-elem", "/>")));
    }

    #[test]
    fn read_name_starts_with_digit() {
        assert_eq!(read_name("123invalid"), None);
    }

    #[test]
    fn read_name_empty() {
        assert_eq!(read_name(""), None);
    }

    #[test]
    fn read_name_underscore_start() {
        assert_eq!(read_name("_private "), Some(("_private", " ")));
    }

    #[test]
    fn read_name_unicode() {
        assert_eq!(read_name("Ölpreis>"), Some(("Ölpreis", ">")));
    }

    // ---- BOM tests ----

    #[test]
    fn bom_detection() {
        assert!(starts_with_bom("\u{FEFF}hello"));
        assert!(!starts_with_bom("hello"));
        assert!(!starts_with_bom(""));
        assert!(!starts_with_bom("ab")); // too short for BOM
    }

    #[test]
    fn bom_stripping() {
        let (rest, had) = strip_bom("\u{FEFF}hello");
        assert_eq!(rest, "hello");
        assert!(had);

        let (rest, had) = strip_bom("hello");
        assert_eq!(rest, "hello");
        assert!(!had);
    }
}
