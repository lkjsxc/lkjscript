//! Shared wire observations and crawl state; every connection uses a grant.
use std::collections::BTreeSet;
use std::json::{self, Value};
use crate::http_client::{Connection, Response, find, io, require};
use crate::http_site_links;

pub const MISSING: &str = "/__lkjscript_site_missing__";
pub struct Page {
    pub path: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub declared_length: Option<usize>,
    pub body: Vec<u8>,
}
fn utf8(bytes: Vec<u8>, label: &str) -> Result<String, String> {
    match String::from_utf8(bytes) {
        Ok(text) => Ok(text),
        Err(_) => Err(format!("{label} is not UTF-8")),
    }
}
fn trim(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && (bytes[start] == b' ' || bytes[start] == b'\t') { start += 1usize; }
    while end > start && (bytes[end - 1usize] == b' ' || bytes[end - 1usize] == b'\t') { end -= 1usize; }
    &value[start..end]
}
fn headers(response: &Response) -> Result<(Vec<(String, String)>, Option<usize>), String> {
    let text = utf8(response.header.clone(), "response headers")?;
    let mut fields: Vec<(String, String)> = Vec::new();
    let mut first = true;
    let mut length = None;
    for line in text.as_str().split("\r\n") {
        if first { first = false; continue; }
        if line.is_empty() { break; }
        let colon = find(line.as_bytes(), b":").ok_or("response header missing colon".to_string())?;
        let name = line[..colon].to_ascii_lowercase();
        let value = trim(&line[colon + 1usize..]).to_string();
        if name == "content-length" { length = Some(response.length); }
        else if name != "date" && name != "connection" { fields.push((name, value)); }
    }
    Ok((fields, length))
}
pub fn fetch(target: &mut Connector, method: &str, path: &str, extra: &str) -> Result<Page, String> {
    require(method == "GET" || method == "HEAD", "site observations permit only GET and HEAD")?;
    for byte in path.as_bytes() {
        require(*byte > 32u8 && *byte != 127u8, "request target contains whitespace or a control byte")?;
    }
    let mut connection = Connection::new(io(target.connect(), "connect to granted site endpoint")?);
    // Both endpoints receive the origin's logical authority, independent of the
    // candidate's fresh transport port. No redirects select a new Connector.
    let request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:8798\r\nConnection: close\r\n{extra}\r\n");
    connection.send(request.as_bytes())?;
    let response = connection.header(method == "HEAD")?;
    let no_body = method == "HEAD" || response.status < 200u16 || response.status == 204u16 || response.status == 304u16;
    let body = connection.body(if no_body { 0usize } else { response.length })?;
    connection.eof()?;
    let (fields, declared_length) = headers(&response)?;
    Ok(Page { path: path.to_string(), status: response.status, headers: fields, declared_length, body })
}
pub fn get(target: &mut Connector, path: &str) -> Result<Page, String> {
    match fetch(target, "GET", path, "") {
        Ok(page) => Ok(page),
        Err(problem) => Err(format!("GET {path}: {problem}")),
    }
}
pub fn metadata(page: &Page, body: &str) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for field in &page.headers {
        headers.push(Value::Array(vec![Value::String(field.0.clone()), Value::String(field.1.clone())]));
    }
    Value::Object(vec![
        ("path".to_string(), Value::String(page.path.clone())),
        ("status".to_string(), Value::from_usize(page.status as usize)),
        ("headers".to_string(), Value::Array(headers)),
        ("body".to_string(), Value::String(body.to_string())),
    ])
}
pub struct Crawl {
    paths: Vec<String>,
    seen: BTreeSet<String>,
    at: usize,
    queries: bool,
}
impl Crawl {
    pub fn new(queries: bool) -> Crawl {
        let mut crawl = Crawl { paths: Vec::new(), seen: BTreeSet::new(), at: 0usize, queries };
        crawl.add("/".to_string());
        crawl.add("/robots.txt".to_string());
        crawl
    }
    pub fn add(&mut self, path: String) {
        let end = if self.queries { path.len() } else { find(path.as_bytes(), b"?").unwrap_or(path.len()) };
        let path = path[..end].to_string();
        if self.seen.insert(path.clone()) { self.paths.push(path); }
    }
    pub fn next(&mut self) -> Option<String> {
        if self.at == self.paths.len() { return None; }
        let path = self.paths[self.at].clone();
        self.at += 1usize;
        Some(path)
    }
    pub fn contains(&self, path: &str) -> bool {
        for known in &self.paths { if known.as_str() == path { return true; } }
        false
    }
    pub fn discover(&mut self, page: &Page) -> Result<(), String> {
        let mut html = false;
        for field in &page.headers {
            if field.0 == "content-type" && field.1.as_str().starts_with("text/html") { html = true; }
        }
        if html {
            let links = http_site_links::html(&page.path, &page.body)?;
            for path in links { self.add(path); }
        }
        if page.path == "/robots.txt" {
            let text = utf8(page.body.clone(), "robots.txt")?;
            for line in text.as_str().lines() {
                if let Some((name, value)) = line.split_once(':') {
                    let name = std::str::trim_ascii(name);
                    if name.eq_ignore_ascii_case("allow") || name.eq_ignore_ascii_case("disallow") || name.eq_ignore_ascii_case("sitemap") {
                        let value = std::str::trim_ascii(value);
                        if !value.contains('*') {
                            if let Some(path) = http_site_links::resolve("/", value) { self.add(path); }
                        }
                    }
                }
            }
        }
        Ok(())
    }
}
pub fn extra_paths(root: &mut Directory) -> Result<Vec<String>, String> {
    extra_paths_at(root, "paths.txt")
}
pub fn extra_paths_at(root: &mut Directory, name: &str) -> Result<Vec<String>, String> {
    let mut input = match root.open(name) {
        Ok(file) => file,
        Err(2i64) => return Ok(Vec::new()),
        Err(errno) => return Err(format!("open parity paths.txt: errno {errno}")),
    };
    let text = utf8(io(input.read_to_end(), "read parity paths.txt")?, "paths.txt")?;
    let mut paths = Vec::new();
    for line in text.as_str().lines() {
        let line = std::str::trim_ascii(line);
        if line.is_empty() || line.starts_with("#") { continue; }
        let path = http_site_links::resolve("/", line).ok_or(format!("parity path is not same-origin: {line}"))?;
        paths.push(path);
    }
    Ok(paths)
}
pub fn manifest_paths(root: &mut Directory) -> Result<Vec<String>, String> {
    let mut input = io(root.open("manifest.json"), "open snapshot manifest")?;
    let bytes = io(input.read_to_end(), "read snapshot manifest")?;
    let value = match json::parse(&bytes) { Ok(value) => value, Err(error) => return Err(format!("manifest {error}")) };
    require(value.get("version").and_then(|value| value.as_usize()) == Some(1usize), "manifest version must be 1")?;
    let pages = value.get("pages").and_then(|value| value.as_array()).ok_or("manifest pages must be an array".to_string())?;
    let mut paths = Vec::new();
    for page in pages {
        let path = page.get("path").and_then(|value| value.as_str()).ok_or("manifest page path must be text".to_string())?;
        paths.push(path.to_string());
    }
    Ok(paths)
}
pub fn compare_metadata(expected: &Page, actual: &Page) -> Result<(), String> {
    require(expected.status == actual.status, &format!("status differs: origin {}, candidate {}", expected.status, actual.status))?;
    require(expected.declared_length == actual.declared_length, "Content-Length presence/value differs")?;
    let mut used = vec![false; actual.headers.len()];
    for wanted in &expected.headers {
        let mut found = false;
        for index in 0usize..actual.headers.len() {
            if !used[index] && actual.headers[index].0 == wanted.0 && actual.headers[index].1 == wanted.1 {
                used[index] = true;
                found = true;
                break;
            }
        }
        require(found, &format!("missing or different candidate header {}: {}", wanted.0, wanted.1))?;
    }
    for index in 0usize..actual.headers.len() {
        require(used[index], &format!("unexpected candidate header {}: {}", actual.headers[index].0, actual.headers[index].1))?;
    }
    Ok(())
}
pub fn compare(expected: &Page, actual: &Page) -> Result<(), String> {
    compare_metadata(expected, actual)?;
    let common = if expected.body.len() < actual.body.len() { expected.body.len() } else { actual.body.len() };
    let mut at = 0usize;
    while at < common && expected.body[at] == actual.body[at] { at += 1usize; }
    require(at == common && expected.body.len() == actual.body.len(),
        &format!("body differs at byte {at}: origin {} bytes, candidate {} bytes", expected.body.len(), actual.body.len()))
}
#[test]
fn stable_headers_and_head_length() {
    let raw = b"HTTP/1.1 200 OK\r\nDate: yesterday\r\nContent-Length: 0\r\nConnection: close\r\nContent-Type: text/html\r\nCache-Control: no-cache\r\nX-Content-Type-Options: nosniff\r\n\r\n";
    let response = crate::http_client::parse_header(raw[..].to_vec(), true).unwrap();
    let (fields, length) = headers(&response).unwrap();
    assert_eq!(length, Some(0usize));
    assert_eq!(fields.len(), 3usize);
    assert_eq!(fields[0usize].0, "content-type");
    assert_eq!(fields[2usize].1, "nosniff");
}
#[test]
fn header_order_and_body_comparison() {
    let first = Page { path: "/".to_string(), status: 200u16,
        headers: vec![("content-type".to_string(), "text/plain".to_string()), ("cache-control".to_string(), "no-cache".to_string())],
        declared_length: Some(3usize), body: b"abc"[..].to_vec() };
    let mut second = Page { path: "/".to_string(), status: 200u16,
        headers: vec![("cache-control".to_string(), "no-cache".to_string()), ("content-type".to_string(), "text/plain".to_string())],
        declared_length: Some(3usize), body: b"abc"[..].to_vec() };
    assert!(compare(&first, &second).is_ok());
    second.body[2usize] = b'd';
    assert!(compare(&first, &second).is_err());
    second.body[2usize] = b'c';
    second.headers.push(("vary".to_string(), "accept-encoding".to_string()));
    assert!(compare(&first, &second).is_err());
}
