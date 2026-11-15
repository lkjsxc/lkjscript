//! Desk's three routes. External board, handoff and wake authority are absent.
use super::store;
use crate::http_protocol::Request;
use crate::http_server::{Body, Handler, Reply};
use std::json::{self, Value};
const PAGE: &str = include_str!("index.html");
pub struct Desk { pub state: Directory, pub clock: Clock, pub entropy: Entropy }
fn reply(status: u16, kind: &str, body: String) -> Reply {
    Reply::new(status, kind, Body::Message(body)).header("Cache-Control", "no-store")
}
fn empty(status: u16) -> Reply { reply(status, "text/plain", String::new()) }
fn json_reply(value: &Value) -> Result<Reply, String> {
    Ok(reply(200u16, "application/json", store::json_text(value)?))
}
fn last_path(path: &str) -> &str {
    let mut end = path.len();
    while end > 0usize && path.as_bytes()[end - 1usize] == b'/' { end -= 1usize; }
    let start = match path[..end].rfind('/') { Some(index) => index + 1usize, None => 0usize };
    &path[start..end]
}
fn limit(text: &str, maximum: usize) -> String {
    let mut result = String::new();
    let mut count = 0usize;
    for character in text.chars() {
        if count == maximum { break; }
        result.push(character);
        count += 1usize;
    }
    result
}
// The page sends strings. Preserve Python's useful scalar conversions without
// silently accepting a structure where the JSON object contract requires text.
fn scalar(value: Option<&Value>, null_empty: bool) -> Result<String, String> {
    match value {
        None => Ok(String::new()),
        Some(value) => match value {
            Value::String(text) => Ok(text.clone()),
            Value::Number(text) => if null_empty && text == "0" { Ok(String::new()) } else { Ok(text.clone()) },
            Value::Bool(true) => Ok("True".to_string()),
            Value::Bool(false) => Ok(if null_empty { String::new() } else { "False".to_string() }),
            Value::Null => Ok(if null_empty { String::new() } else { "None".to_string() }),
            _ => Err("desk text, re and request must be scalar values".to_string()),
        },
    }
}
fn message(body: &[u8]) -> Result<(String, String, String), String> {
    let length = if body.len() > 65_536usize { 65_536usize } else { body.len() };
    let value = match json::parse(&body[..length]) {
        Ok(value) => value, Err(_) => return Err("invalid desk JSON".to_string()),
    };
    if value.as_object().is_none() { return Err("desk message must be an object".to_string()); }
    let text = scalar(value.get("text"), false)?;
    let text = limit(std::text::trim(&text), 8000usize);
    let re = limit(&scalar(value.get("re"), true)?, 32usize);
    let request = limit(&scalar(value.get("request"), true)?, 128usize);
    Ok((text, re, request))
}
impl Handler for Desk {
    fn methods(&self) -> &'static str { "GET, HEAD, POST" }
    fn preflight(&mut self, request: &Request, _now: i64) -> Option<Reply> {
        if request.method == "POST" {
            let header = match &request.x_desk { Some(value) => value.as_str(), None => "" };
            let mut path = request.raw_path.clone();
            if !request.query.is_empty() { path.push('?'); path.push_str(&request.query); }
            let mut end = path.len();
            while end > 0usize && path.as_bytes()[end - 1usize] == b'/' { end -= 1usize; }
            if header != "1" || !path[..end].ends_with("say") { return Some(empty(403u16)); }
        }
        None
    }
    fn retain_body(&self, request: &Request) -> bool { request.method == "POST" }
    fn respond(&mut self, request: &Request, body: Vec<u8>, now: i64) -> Result<Reply, String> {
        if request.method != "POST" {
            if last_path(&request.raw_path) == "state" {
                return json_reply(&store::state(&mut self.state, now)?);
            }
            return Ok(reply(200u16, "text/html; charset=utf-8", PAGE.to_string()));
        }
        let (text, re, request) = match message(&body) {
            Ok(message) => message, Err(_) => return Ok(empty(409u16)),
        };
        if text.is_empty() { return Ok(empty(400u16)); }
        let row = match store::record(&mut self.state, &text, &re, &request, &mut self.clock, &mut self.entropy) {
            Ok(row) => row, Err(problem) => {
                if problem == "Question is closed or missing" { return Ok(empty(409u16)); }
                return Err(problem);
            }
        };
        json_reply(&row.value(None, None))
    }
    fn finish(&mut self) -> Result<(), String> { store::io(self.state.sync(), "sync desk shutdown") }
}
#[test]
fn routes_and_unicode_lengths() {
    assert!(last_path("/proxy/123/state///") == "state");
    assert!(last_path("/").is_empty());
    assert!(limit("a日本😀", 3usize) == "a日本");
    let (text, re, request) = message("{\"text\":\"　日本語 😀　\",\"re\":null,\"request\":\"r\"}".as_bytes()).unwrap();
    assert!(text == "日本語 😀" && re.is_empty() && request == "r");
}
