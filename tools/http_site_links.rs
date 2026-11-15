//! Same-origin URL and HTML-attribute discovery for the documentation clients.
//! This is an HTTP crawler, not an HTML renderer. It follows href/src/action,
//! skips comments and script/style raw text, and never selects another endpoint.
use crate::http_client::{find, require};

fn whitespace(byte: u8) -> bool { byte == 32u8 || (byte >= 9u8 && byte <= 13u8) }
fn lower(byte: u8) -> u8 { if byte >= b'A' && byte <= b'Z' { byte + 32u8 } else { byte } }
fn same(bytes: &[u8], expected: &[u8]) -> bool {
    if bytes.len() != expected.len() { return false; }
    for index in 0usize..bytes.len() {
        if lower(bytes[index]) != lower(expected[index]) { return false; }
    }
    true
}
fn text(bytes: Vec<u8>) -> Result<String, String> {
    match String::from_utf8(bytes) {
        Ok(value) => Ok(value),
        Err(_) => Err("HTML link is not UTF-8".to_string()),
    }
}
fn entities(bytes: &[u8]) -> Result<String, String> {
    let mut output: Vec<u8> = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        if bytes[at] != b'&' { output.push(bytes[at]); at += 1usize; continue; }
        let mut end = at + 1usize;
        while end < bytes.len() && bytes[end] != b';' && bytes[end] != b'&'
            && bytes[end] != b'<' && !whitespace(bytes[end]) { end += 1usize; }
        if end == bytes.len() || bytes[end] != b';' {
            output.push(b'&'); at += 1usize; continue;
        }
        let name = &bytes[at + 1usize..end];
        let named = if name == b"amp" { Some('&') }
            else if name == b"quot" { Some('"') }
            else if name == b"apos" { Some('\'') }
            else if name == b"lt" { Some('<') }
            else if name == b"gt" { Some('>') }
            else { None };
        let scalar = if let Some(value) = named { value } else {
            require(!name.is_empty() && name[0usize] == b'#', "unsupported named HTML link entity")?;
            let mut index = 1usize;
            let hexadecimal = index < name.len() && (name[index] == b'x' || name[index] == b'X');
            if hexadecimal { index += 1usize; }
            require(index < name.len(), "empty numeric HTML link entity")?;
            let radix = if hexadecimal { 16u32 } else { 10u32 };
            let mut value = 0u32;
            while index < name.len() {
                let byte = name[index];
                let digit = if byte >= b'0' && byte <= b'9' { (byte - b'0') as u32 }
                    else if hexadecimal && byte >= b'a' && byte <= b'f' { (byte - b'a' + 10u8) as u32 }
                    else if hexadecimal && byte >= b'A' && byte <= b'F' { (byte - b'A' + 10u8) as u32 }
                    else { return Err("invalid numeric HTML link entity".to_string()); };
                value = value.checked_mul(radix).and_then(|number| number.checked_add(digit))
                    .ok_or("HTML link entity overflow".to_string())?;
                index += 1usize;
            }
            char::from_u32(value).ok_or("invalid Unicode scalar in HTML link".to_string())?
        };
        let encoded = format!("{scalar}");
        output.extend_from_slice(encoded.as_bytes());
        at = end + 1usize;
    }
    text(output)
}
fn quoted_target(value: &str) -> String {
    let digits = b"0123456789ABCDEF";
    let mut output = String::new();
    for byte in value.as_bytes() {
        if *byte <= 32u8 || *byte >= 127u8 || *byte == b'"' || *byte == b'<' || *byte == b'>' {
            output.push('%');
            output.push(digits[(*byte / 16u8) as usize] as char);
            output.push(digits[(*byte % 16u8) as usize] as char);
        } else { output.push(*byte as char); }
    }
    output
}
fn authority_path(value: &str) -> Option<String> {
    let mut end = 0usize;
    let bytes = value.as_bytes();
    while end < bytes.len() && bytes[end] != b'/' && bytes[end] != b'?' { end += 1usize; }
    if &value[..end] != "127.0.0.1:8798" { return None; }
    if end == bytes.len() { return Some("/".to_string()); }
    if bytes[end] == b'?' { return Some(format!("/{}", &value[end..])); }
    Some(value[end..].to_string())
}
/// Resolve a link against a request target. A rejected external scheme/authority
/// returns None; all returned strings are origin-form paths for the one grant.
pub fn resolve(base: &str, reference: &str) -> Option<String> {
    let reference = std::str::trim_ascii(reference);
    let end = find(reference.as_bytes(), b"#").unwrap_or(reference.len());
    let reference = &reference[..end];
    if reference.is_empty() { return None; }
    let relative = if reference.len() >= 7usize && same(&reference.as_bytes()[..7usize], b"http://") {
        authority_path(&reference[7usize..])?
    } else if reference.starts_with("//") {
        authority_path(&reference[2usize..])?
    } else {
        let bytes = reference.as_bytes();
        let mut at = 0usize;
        while at < bytes.len() && bytes[at] != b'/' && bytes[at] != b'?' {
            if bytes[at] == b':' { return None; }
            at += 1usize;
        }
        reference.to_string()
    };
    let base_end = find(base.as_bytes(), b"?").unwrap_or(base.len());
    let base_path = &base[..base_end];
    let target = if relative.starts_with("/") { relative }
        else if relative.starts_with("?") { format!("{base_path}{relative}") }
        else {
            let mut slash = 0usize;
            for index in 0usize..base_path.len() {
                if base_path.as_bytes()[index] == b'/' { slash = index + 1usize; }
            }
            format!("{}{relative}", &base_path[..slash])
        };
    let query_at = find(target.as_bytes(), b"?").unwrap_or(target.len());
    let path = &target[..query_at];
    let query = &target[query_at..];
    let trailing_dot = path.ends_with("/.") || path.ends_with("/..");
    let mut segments: Vec<String> = Vec::new();
    let mut first = true;
    for part in path.split('/') {
        if first { first = false; if part.is_empty() { continue; } }
        if part == "." { continue; }
        if part == ".." { let _ = segments.pop(); }
        else { segments.push(part.to_string()); }
    }
    if trailing_dot && !segments.is_empty() { segments.push(String::new()); }
    let mut normalized = "/".to_string();
    for index in 0usize..segments.len() {
        if index != 0usize { normalized.push('/'); }
        normalized.push_str(&segments[index]);
    }
    normalized.push_str(query);
    Some(quoted_target(&normalized))
}
fn case_find(bytes: &[u8], pattern: &[u8]) -> Option<usize> {
    if bytes.len() < pattern.len() { return None; }
    let mut at = 0usize;
    while at <= bytes.len() - pattern.len() {
        if same(&bytes[at..at + pattern.len()], pattern) { return Some(at); }
        at += 1usize;
    }
    None
}
fn tag_letter(byte: u8) -> bool {
    (byte >= b'a' && byte <= b'z') || (byte >= b'A' && byte <= b'Z')
}
pub fn html(base: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
    let mut output: Vec<String> = Vec::new();
    let mut base = base.to_string();
    let mut at = 0usize;
    while at < bytes.len() {
        let start = match find(&bytes[at..], b"<") { Some(index) => at + index, None => break };
        at = start + 1usize;
        if bytes.len() - at >= 3usize && &bytes[at..at + 3usize] == b"!--" {
            at = match find(&bytes[at + 3usize..], b"-->") {
                Some(index) => at + 3usize + index + 3usize,
                None => bytes.len(),
            };
            continue;
        }
        if at == bytes.len() || !tag_letter(bytes[at]) { continue; }
        let tag_start = at;
        while at < bytes.len() && !whitespace(bytes[at]) && bytes[at] != b'>' && bytes[at] != b'/' { at += 1usize; }
        let tag = &bytes[tag_start..at];
        while at < bytes.len() && bytes[at] != b'>' {
            while at < bytes.len() && (whitespace(bytes[at]) || bytes[at] == b'/') { at += 1usize; }
            if at == bytes.len() || bytes[at] == b'>' { break; }
            let name_start = at;
            while at < bytes.len() && !whitespace(bytes[at]) && bytes[at] != b'=' && bytes[at] != b'>' { at += 1usize; }
            let name = &bytes[name_start..at];
            while at < bytes.len() && whitespace(bytes[at]) { at += 1usize; }
            if at == bytes.len() || bytes[at] != b'=' { continue; }
            at += 1usize;
            while at < bytes.len() && whitespace(bytes[at]) { at += 1usize; }
            require(at < bytes.len(), "unfinished HTML link attribute")?;
            let quote = if bytes[at] == b'\'' || bytes[at] == b'"' { let quote = bytes[at]; at += 1usize; quote } else { 0u8 };
            let value_start = at;
            if quote == 0u8 {
                while at < bytes.len() && !whitespace(bytes[at]) && bytes[at] != b'>' { at += 1usize; }
            } else {
                while at < bytes.len() && bytes[at] != quote { at += 1usize; }
                require(at < bytes.len(), "unterminated HTML link attribute")?;
            }
            let value = &bytes[value_start..at];
            if quote != 0u8 { at += 1usize; }
            if same(name, b"href") || same(name, b"src") || same(name, b"action") {
                let decoded = entities(value)?;
                if same(tag, b"base") && same(name, b"href") {
                    base = resolve(&base, &decoded).ok_or("external HTML base is not supported by the same-origin crawler".to_string())?;
                } else if let Some(path) = resolve(&base, &decoded) { output.push(path); }
            }
        }
        if at < bytes.len() { at += 1usize; }
        if same(tag, b"script") || same(tag, b"style") {
            let close = if same(tag, b"script") { &b"</script"[..] } else { &b"</style"[..] };
            at = match case_find(&bytes[at..], close) { Some(index) => at + index, None => bytes.len() };
        }
    }
    Ok(output)
}
#[test]
fn url_resolution() {
    assert!(resolve("/docs/http", "../start#one") == Some("/start".to_string()));
    assert!(resolve("/docs/http", "./start") == Some("/docs/start".to_string()));
    assert!(resolve("/docs/http?q=old", "?q=new#x") == Some("/docs/http?q=new".to_string()));
    assert!(resolve("/", "http://127.0.0.1:8798/docs/start") == Some("/docs/start".to_string()));
    assert!(resolve("/", "//example.invalid/docs/start").is_none());
    assert!(resolve("/", "https://127.0.0.1:8798/").is_none());
    assert!(resolve("/", "http://127.0.0.1:8798@elsewhere/").is_none());
    assert!(resolve("/", "mailto:test@example.invalid").is_none());
    assert!(resolve("/a/b", "#fragment").is_none());
    assert!(resolve("/", "/a//b/../") == Some("/a//".to_string()));
    assert!(resolve("/", "/docs/%73tart") == Some("/docs/%73tart".to_string()));
    assert!(resolve("/", "/search?q=雪 x") == Some("/search?q=%E9%9B%AA%20x".to_string()));
}
#[test]
fn html_attribute_discovery() {
    let input = b"<!-- <a href='/ignored'> --><a HREF='../start#top'>x</a><img src=/assets/site.css><form action='/search?q=a&amp;b=2'><script>var x=\"<a href='/not-a-link'>\";</script><style>x{content:\"<a href='/also-not'>\"}</style><a href='https://elsewhere.invalid/'>outside</a>";
    let links = html("/docs/http", input).unwrap();
    assert_eq!(links, vec!["/start".to_string(), "/assets/site.css".to_string(), "/search?q=a&b=2".to_string()]);
    let numeric = html("/", b"<a href='/&#x64;ocs/&#115;tart'>").unwrap();
    assert_eq!(numeric, vec!["/docs/start".to_string()]);
    assert!(html("/", b"<a href='/unfinished>").is_err());
}
