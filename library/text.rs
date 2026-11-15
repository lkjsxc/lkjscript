//! Text transformations expressed through ordinary owned strings and Unicode scalars.
//! Lossy UTF-8 replaces each maximal ill-formed subpart with U+FFFD.
use crate::{IntoIterator, Iterator, String};

/// Preserve valid UTF-8 and replace each maximal invalid subpart once.
/// A bad continuation is reconsidered as a new leading byte; a truncated valid
/// prefix is one replacement, while overlong/surrogate/out-of-range prefixes
/// stop before the first forbidden continuation.
pub fn utf8_lossy(bytes: &[u8]) -> String {
    let mut result = String::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let first = bytes[at];
        if first < 0x80u8 {
            result.push(first as char);
            at += 1usize;
            continue;
        }
        let width = if first >= 0xc2u8 && first <= 0xdfu8 { 2usize }
            else if first >= 0xe0u8 && first <= 0xefu8 { 3usize }
            else if first >= 0xf0u8 && first <= 0xf4u8 { 4usize }
            else {
                result.push('\u{fffd}');
                at += 1usize;
                continue;
            };
        let mut code = if width == 2usize { (first & 0x1fu8) as u32 }
            else if width == 3usize { (first & 0x0fu8) as u32 }
            else { (first & 0x07u8) as u32 };
        let mut low = if first == 0xe0u8 { 0xa0u8 }
            else if first == 0xf0u8 { 0x90u8 } else { 0x80u8 };
        let mut high = if first == 0xedu8 { 0x9fu8 }
            else if first == 0xf4u8 { 0x8fu8 } else { 0xbfu8 };
        at += 1usize;
        let mut consumed = 1usize;
        while consumed < width && at < bytes.len() {
            let byte = bytes[at];
            if byte < low || byte > high { break; }
            code = code * 64u32 + (byte & 0x3fu8) as u32;
            at += 1usize;
            consumed += 1usize;
            low = 0x80u8;
            high = 0xbfu8;
        }
        if consumed == width { result.push(char::from_u32(code).unwrap()); }
        else { result.push('\u{fffd}'); }
    }
    result
}
fn hex(byte: u8) -> u8 {
    if byte >= b'0' && byte <= b'9' { byte - b'0' }
    else if byte >= b'a' && byte <= b'f' { byte - b'a' + 10u8 }
    else if byte >= b'A' && byte <= b'F' { byte - b'A' + 10u8 }
    else { 255u8 }
}
/// Decode one application/x-www-form-urlencoded component.
/// Plus becomes space, valid percent pairs become bytes, malformed percent text
/// stays literal, and the resulting bytes use utf8_lossy's replacement rules.
pub fn form_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut decoded: Vec<u8> = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let byte = bytes[at];
        if byte == b'+' {
            decoded.push(b' ');
        } else if byte == b'%' && bytes.len() - at >= 3usize {
            let first = hex(bytes[at + 1usize]);
            let second = hex(bytes[at + 2usize]);
            if first < 16u8 && second < 16u8 {
                decoded.push(first * 16u8 + second);
                at += 3usize;
                continue;
            }
            decoded.push(byte);
        } else {
            decoded.push(byte);
        }
        at += 1usize;
    }
    utf8_lossy(&decoded)
}
/// Escape the five HTML text/quoted-attribute characters; other scalars are kept.
pub fn escape_html(value: &str) -> String {
    let mut result = String::new();
    for scalar in value.chars() {
        match scalar {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(scalar),
        }
    }
    result
}
/// Borrow the text between its first and last non-White_Space Unicode scalars.
pub fn trim(value: &str) -> &str {
    let mut start = 0usize;
    let mut end = 0usize;
    let mut at = 0usize;
    let mut found = false;
    for scalar in value.chars() {
        let next = at + scalar.len_utf8();
        if !scalar.is_whitespace() {
            if !found { start = at; found = true; }
            end = next;
        }
        at = next;
    }
    &value[start..end]
}
/// Unicode 17.0.0 lowercase mapping applied to each scalar independently.
/// This includes expansions such as I-with-dot -> i + combining dot, but no
/// surrounding-text rules: Greek capital sigma always becomes ordinary sigma,
/// even at a word end. This is not locale-sensitive or contextual str lowercase.
pub fn lowercase_chars(value: &str) -> String {
    let mut result = String::new();
    for scalar in value.chars() { crate::text_lower::append(scalar, &mut result); }
    result
}
