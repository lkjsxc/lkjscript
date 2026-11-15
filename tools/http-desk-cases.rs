//! Read/write proof for a private desk instance; never points at live desk.
mod http_client;
use http_client::{Connection, io, require};
use std::json::{self, Value};

fn fetch(target: &mut Connector, method: &str, path: &str, extra: &str,
    body: &str, status: u16) -> Result<Vec<u8>, String> {
    let stream = io(target.connect(), "connect isolated desk")?;
    let mut connection = Connection::new(stream);
    let length = body.len();
    let request = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {length}\r\n{extra}\r\n{body}");
    connection.send(request.as_bytes())?;
    let response = connection.header(false)?;
    require(response.status == status, &format!("{method} {path}: expected {status}, got {}", response.status))?;
    require(http_client::find(&response.header, b"Cache-Control: no-store\r\n").is_some(), "desk response must not be cached")?;
    let body = connection.body(response.length)?;
    connection.eof()?;
    Ok(body)
}
fn decode(bytes: &[u8]) -> Result<Value, String> {
    match json::parse(bytes) { Ok(value) => Ok(value), Err(_) => Err("desk returned invalid JSON".to_string()) }
}
fn field<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value.get(key).and_then(|value| value.as_str()).ok_or(format!("desk row missing {key}"))
}
fn check(target: &mut Connector, page: &mut Directory, question: &str) -> Result<(), String> {
    let mut file = io(page.open("index.html"), "open expected page")?;
    let expected = io(file.read_to_end(), "read expected page")?;
    require(fetch(target, "GET", "/", "", "", 200u16)? == expected, "desk page bytes differ")?;
    let value = decode(&fetch(target, "GET", "/proxy/123/state/?query=1", "", "", 200u16)?)?;
    require(value.get("jobs").and_then(|value| value.as_array()).unwrap().is_empty(), "ungranted board must be empty")?;
    require(field(&value, "detail")?.is_empty(), "ungranted handoff must be empty")?;
    let asks = value.get("asks").and_then(|value| value.as_array()).ok_or("missing asks".to_string())?;
    require(asks.len() == 1usize && field(&asks[0usize], "id")? == question, "open question is absent")?;
    fetch(target, "POST", "/say", "", "{\"text\":\"forbidden\"}", 403u16)?;
    fetch(target, "POST", "/say?x=1", "X-Desk: 1\r\n", "{\"text\":\"forbidden\"}", 403u16)?;
    fetch(target, "POST", "/say", "X-Desk: 1\r\n", "{", 409u16)?;
    fetch(target, "POST", "/say", "X-Desk: 1\r\n", "{\"text\":\"　 \"}", 400u16)?;
    let message = "{\"text\":\"　owner 日本語 😀　\",\"request\":\"repeat-owner\"}";
    let first = decode(&fetch(target, "POST", "/proxy/123/say/", "X-Desk: 1\r\n", message, 200u16)?)?;
    require(field(&first, "text")? == "owner 日本語 😀", "owner text trimming differs")?;
    let retried = decode(&fetch(target, "POST", "/say", "X-Desk: 1\r\n",
        "{\"text\":\"different retry\",\"request\":\"repeat-owner\"}", 200u16)?)?;
    require(field(&first, "id")? == field(&retried, "id")?
        && field(&first, "text")? == field(&retried, "text")?, "request retry created or changed a message")?;
    let message = format!("{{\"text\":\"answer 日本語\",\"re\":\"{question}\"}}");
    let answer = decode(&fetch(target, "POST", "/say", "X-Desk: 1\r\n", &message, 200u16)?)?;
    let again = decode(&fetch(target, "POST", "/say", "X-Desk: 1\r\n", &message, 200u16)?)?;
    require(field(&answer, "id")? == field(&again, "id")?, "answer retry duplicated a record")?;
    fetch(target, "POST", "/say", "X-Desk: 1\r\n",
        "{\"text\":\"bad answer\",\"re\":\"missing-question\"}", 409u16)?;
    let after = decode(&fetch(target, "GET", "/state", "", "", 200u16)?)?;
    require(after.get("asks").and_then(|value| value.as_array()).unwrap().is_empty(), "answered question remains open")?;
    let feed = after.get("feed").and_then(|value| value.as_array()).ok_or("missing feed".to_string())?;
    let mut found = 0usize;
    for row in feed {
        if field(row, "kind")? == "owner" && field(row, "text")? == "answer 日本語" { found += 1usize; }
    }
    require(found == 1usize, "answer appears zero or multiple times in feed")?;
    Ok(())
}
fn retry_read(target: &mut Connector) -> Result<(), String> {
    let value = decode(&fetch(target, "POST", "/say", "X-Desk: 1\r\n",
        "{\"text\":\"retry after read\",\"request\":\"repeat-owner\"}", 200u16)?)?;
    require(field(&value, "text")? == "owner 日本語 😀", "read retry changed the original message")?;
    Ok(())
}
pub fn main(mut output: Stdout, mut input: Stdin, mut target: Connector, mut page: Directory) {
    let bytes = input.read_to_end().unwrap();
    let question = String::from_utf8(bytes).unwrap();
    let text = std::str::trim_ascii(question.as_str());
    let result = if text == "retry-read" { retry_read(&mut target) }
        else { check(&mut target, &mut page, text) };
    match result {
        Ok(()) => output.write(b"ok desk HTTP: exact page, proxied state/say, owner messages, first-answer/request retry, closed/missing questions, header/body rejection\n"),
        Err(problem) => { output.write(format!("desk HTTP: {problem}\n").as_bytes()); panic!("desk HTTP failed"); }
    }
}
