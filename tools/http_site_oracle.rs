//! Operational test oracle: live response observations, never server input.
use std::json::{self, Value};
use std::sha256;
use crate::http_client::{io, require};
use crate::http_site::{self, Page};
use crate::http_site_probes::{self, Probe};

pub const PATH: &str = "tests/fixtures/site/origin.json";
pub const MANIFEST: &str = "examples/site-content/manifest.json";
pub fn bytes(root: &mut Directory, path: &str) -> Result<Vec<u8>, String> {
    let mut file = io(root.open(path), &format!("open {path}"))?;
    io(file.read_to_end(), &format!("read {path}"))
}
pub fn read(root: &mut Directory, path: &str) -> Result<Value, String> {
    let input = bytes(root, path)?;
    match json::parse(&input) {
        Ok(value) => Ok(value),
        Err(problem) => Err(format!("{path}: {problem}")),
    }
}
pub fn version(value: &Value) -> Result<(), String> {
    require(value.get("version").and_then(|field| field.as_usize()) == Some(1usize), "site fixture version must be 1")
}
pub fn string(value: &Value, name: &str) -> Result<String, String> {
    match value.get(name).and_then(|field| field.as_str()) {
        Some(text) => Ok(text.to_string()),
        None => Err(format!("site fixture {name} must be a string")),
    }
}
pub fn requests(root: &mut Directory) -> Result<Vec<Probe>, String> {
    let paths = http_site::extra_paths_at(root, "examples/site-content/paths.txt")?;
    require(paths.len() == 51usize, "site query fixture must contain the 51 recorded paths; refresh the oracle when changing the list")?;
    let mut requests: Vec<Probe> = Vec::new();
    for path in paths {
        require(path.starts_with("/search?"), "site query fixture contains a non-search target")?;
        requests.push(Probe::new("search query", "GET", &path, "none"));
    }
    let protocol = http_site_probes::protocol();
    require(protocol.len() == 13usize, "site protocol fixture must contain thirteen probes")?;
    for request in protocol { requests.push(request); }
    Ok(requests)
}
pub fn metadata(value: &Value, path: &str, declared_length: Option<usize>) -> Result<Page, String> {
    let status = value.get("status").and_then(|field| field.as_usize()).ok_or("site fixture status must be an integer".to_string())?;
    require(status >= 100usize && status <= 599usize, "site fixture status is outside HTTP's status range")?;
    let fields = value.get("headers").and_then(|field| field.as_array()).ok_or("site fixture headers must be an array".to_string())?;
    require(fields.len() == 6usize, "site fixture must retain all six stable response headers")?;
    let mut headers: Vec<(String, String)> = Vec::new();
    for field in fields {
        let pair = field.as_array().ok_or("site fixture header must be a pair".to_string())?;
        require(pair.len() == 2usize, "site fixture header must have a name and value")?;
        let name = pair[0usize].as_str().ok_or("site fixture header name must be text".to_string())?;
        let value = pair[1usize].as_str().ok_or("site fixture header value must be text".to_string())?;
        require(name == name.to_ascii_lowercase().as_str(), "site fixture header names must be lowercase")?;
        require(name != "date" && name != "connection" && name != "content-length", "site fixture stable headers contain a volatile or derived field")?;
        headers.push((name.to_string(), value.to_string()));
    }
    Ok(Page { path: path.to_string(), status: status as u16, headers, declared_length, body: Vec::new() })
}
pub fn recorded(request: &Probe, page: &Page) -> Value {
    let mut headers: Vec<Value> = Vec::new();
    for field in &page.headers {
        headers.push(Value::Array(vec![Value::String(field.0.clone()), Value::String(field.1.clone())]));
    }
    let length = match page.declared_length {
        Some(length) => Value::from_usize(length),
        None => Value::Null,
    };
    Value::Object(vec![
        ("name".to_string(), Value::String(request.name.clone())),
        ("method".to_string(), Value::String(request.method.clone())),
        ("path".to_string(), Value::String(request.path.clone())),
        ("extra_headers".to_string(), Value::String(request.extra_headers.clone())),
        ("status".to_string(), Value::from_usize(page.status as usize)),
        ("declared_length".to_string(), length),
        ("headers".to_string(), Value::Array(headers)),
        ("body_sha256".to_string(), Value::String(sha256::hex_digest(&page.body))),
    ])
}
pub fn verify_record(value: &Value, request: &Probe, actual: &Page) -> Result<(), String> {
    require(string(value, "name")? == request.name, "site oracle probe name differs from its request list")?;
    require(string(value, "method")? == request.method, "site oracle method differs from its request list")?;
    require(string(value, "path")? == request.path, "site oracle path differs from its request list")?;
    require(string(value, "extra_headers")? == request.extra_headers, "site oracle headers differ from their request descriptor")?;
    let length = match value.get("declared_length") {
        Some(Value::Null) => None,
        Some(field) => Some(field.as_usize().ok_or("site oracle Content-Length must be null or an integer".to_string())?),
        None => return Err("site oracle has no Content-Length observation".to_string()),
    };
    let expected = metadata(value, &request.path, length)?;
    http_site::compare_metadata(&expected, actual)?;
    let digest = string(value, "body_sha256")?;
    require(digest.len() == 64usize, "site oracle SHA-256 must have 64 lowercase hexadecimal digits")?;
    for byte in digest.as_bytes() {
        require((*byte >= b'0' && *byte <= b'9') || (*byte >= b'a' && *byte <= b'f'), "site oracle SHA-256 is not lowercase hexadecimal")?;
    }
    require(sha256::hex_digest(&actual.body) == digest, "body SHA-256 differs from the complete live-origin observation")
}
pub fn archived(root: &mut Directory, value: &Value) -> Result<Page, String> {
    let path = string(value, "path")?;
    let filename = string(value, "body")?;
    require(filename.len() > 5usize && filename.ends_with(".body"), "site archive body filename must be decimal.body")?;
    for byte in filename[..filename.len() - 5usize].as_bytes() {
        require(*byte >= b'0' && *byte <= b'9', "site archive body filename must be decimal.body")?;
    }
    let body = bytes(root, &format!("examples/site-content/{filename}"))?;
    let mut expected = metadata(value, &path, Some(body.len()))?;
    expected.body = body;
    Ok(expected)
}
