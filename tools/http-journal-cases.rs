//! Focused journal HTTP requests used by http-journal-cases.sh.
mod http_client;
use http_client::Connection;
use std::json::{self, Value};
fn read(connection: &mut Connection, status: u16) -> Value {
    let response = connection.header(false).unwrap();
    assert!(response.status == status);
    let bytes = connection.body(response.length).unwrap();
    json::parse(&bytes).unwrap()
}
fn encoded(method: &str, path: &str, text: &str) -> Vec<u8> {
    format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{text}", text.len()).as_bytes().to_vec()
}
fn request(connection: &mut Connection, method: &str, path: &str, text: &str, status: u16) -> Value {
    connection.send(&encoded(method, path, text)).unwrap();
    read(connection, status)
}
fn entry(value: &Value, id: usize, revision: usize, text: &str) {
    assert!(value.get("id").unwrap().as_usize() == Some(id));
    assert!(value.get("revision").unwrap().as_usize() == Some(revision));
    assert!(value.get("text").unwrap().as_str() == Some(text));
}
fn list(value: &Value, count: usize) {
    match value.get("entries").unwrap() { Value::Array(entries) => assert!(entries.len() == count), _ => panic!("not entries") }
}
pub fn main(mut world: World) {
    let args = world.args();
    let port = String::from_utf8(args[0].clone()).unwrap().as_str().parse::<u16>().unwrap();
    let mode = String::from_utf8(args[1].clone()).unwrap();
    let mut target = world.connector("127.0.0.1", port).unwrap();
    let mut connection = Connection::new(target.connect().unwrap());
    if mode == "init" {
        let value = request(&mut connection, "POST", "/entries", "{\"text\":\"alpha\"}", 201u16);
        entry(&value, 1usize, 1usize, "alpha");
        let mut pipeline = encoded("POST", "/entries", "{\"text\":\"snow \\u2603\\n\\u0000\"}");
        pipeline.extend_from_slice(&encoded("GET", "/entries/2", ""));
        connection.send(&pipeline).unwrap();
        entry(&read(&mut connection, 201u16), 2usize, 2usize, "snow ☃\n\0");
        entry(&read(&mut connection, 200u16), 2usize, 2usize, "snow ☃\n\0");
        let value = request(&mut connection, "PUT", "/entries/1", "{\"text\":\"omega\"}", 200u16);
        entry(&value, 1usize, 3usize, "omega");
        let _ = request(&mut connection, "POST", "/entries", "{\"text\":\"x\",\"t\\u0065xt\":\"y\"}", 400u16);
        let _ = request(&mut connection, "PUT", "/entries/99", "{\"text\":\"absent\"}", 404u16);
    }
    if mode == "add" {
        let mut text = String::new();
        for _index in 0usize..20000usize { text.push('x'); }
        let body = json::stringify(&Value::Object(vec![("text".to_string(), Value::String(text.clone()))])).unwrap();
        let value = request(&mut connection, "POST", "/entries", &body, 201u16);
        entry(&value, 3usize, 4usize, &text);
    } else {
        entry(&request(&mut connection, "GET", "/entries/1", "", 200u16), 1usize, 3usize, "omega");
        entry(&request(&mut connection, "GET", "/entries/2", "", 200u16), 2usize, 2usize, "snow ☃\n\0");
        list(&request(&mut connection, "GET", "/entries/1/3", "", 200u16), 2usize);
        list(&request(&mut connection, "GET", "/entries/2/3", "", 200u16), 1usize);
        connection.send(&encoded("HEAD", "/entries/1", "")).unwrap();
        let response = connection.header(true).unwrap();
        assert!(response.status == 200u16 && response.length > 0usize);
        let status = request(&mut connection, "GET", "/status", "", 200u16);
        let extra = mode == "extra";
        if extra {
            let value = request(&mut connection, "GET", "/entries/3", "", 200u16);
            let text = value.get("text").unwrap().as_str().unwrap();
            assert!(text.len() == 20000usize);
            for byte in text.as_bytes() { assert!(*byte == b'x'); }
        }
        assert!(status.get("revision").unwrap().as_usize() == Some(if extra { 4usize } else { 3usize }));
        assert!(status.get("entries").unwrap().as_usize() == Some(if extra { 3usize } else { 2usize }));
    }
    world.stdout().write(format!("ok journal {mode}\n").as_bytes());
}
