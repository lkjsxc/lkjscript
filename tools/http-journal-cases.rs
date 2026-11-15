//! Focused journal HTTP requests used by http-journal-cases.sh.
mod http_client;
mod http_journal_auth;
use http_client::Connection;
use std::json::{self, Value};
fn read(connection: &mut Connection, status: u16) -> Value {
    let response = connection.header(false).unwrap();
    assert!(response.status == status);
    assert!(http_client::find(&response.header, b"\r\nDate: ").is_some());
    if status == 401u16 {
        assert!(response.close);
        assert!(http_client::find(&response.header, b"WWW-Authenticate: Bearer realm=\"journal\"").is_some());
    }
    let bytes = connection.body(response.length).unwrap();
    json::parse(&bytes).unwrap()
}
fn encoded(token: &str, method: &str, path: &str, text: &str) -> Vec<u8> {
    let authorization = http_journal_auth::header(token);
    format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{authorization}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{text}", text.len()).as_bytes().to_vec()
}
fn request(connection: &mut Connection, token: &str, method: &str, path: &str, text: &str, status: u16) -> Value {
    connection.send(&encoded(token, method, path, text)).unwrap();
    read(connection, status)
}
fn rejected(target: &mut Connector, token: &str, method: &str, path: &str, text: &str) {
    let mut connection = Connection::new(target.connect().unwrap());
    let _ = request(&mut connection, token, method, path, text, 401u16);
    match connection.eof() {
        Ok(()) => {},
        Err(problem) => assert!(problem == "read response: errno 104"),
    }
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
    let path = String::from_utf8(args[2].clone()).unwrap();
    let mut root = world.directory(&path).unwrap();
    let mut entropy = world.entropy();
    let mut clock = world.clock();
    let now = clock.unix_seconds().unwrap();
    let token = if args.len() > 3usize {
        let name = String::from_utf8(args[3].clone()).unwrap();
        let mut file = root.open(&name).unwrap();
        let text = String::from_utf8(file.read_to_end().unwrap()).unwrap();
        std::str::trim_ascii(text.as_str()).to_string()
    } else { http_journal_auth::issue(&mut root, &mut entropy, now, 3600i64).unwrap() };
    let mut target = world.connector("127.0.0.1", port).unwrap();
    let mut connection = Connection::new(target.connect().unwrap());
    if mode == "init" {
        rejected(&mut target, "", "POST", "/entries", "{\"text\":\"unauthorized\"}");
        rejected(&mut target, "", "PUT", "/entries/99", "not JSON");
        let expired = http_journal_auth::issue(&mut root, &mut entropy, now, -1i64).unwrap();
        rejected(&mut target, &expired, "POST", "/entries", "{\"text\":\"expired\"}");
        let mut altered = token.as_bytes().to_vec();
        let last = altered.len() - 1usize;
        altered[last] = if altered[last] == b'0' { b'1' } else { b'0' };
        let bad_token = String::from_utf8(altered).unwrap();
        rejected(&mut target, &bad_token, "POST", "/entries", "{\"text\":\"bad signature\"}");
        let mut duplicate = Connection::new(target.connect().unwrap());
        duplicate.send(format!("POST /entries HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer {token}\r\nAuthorization: Bearer {token}\r\nContent-Length: 0\r\n\r\n").as_bytes()).unwrap();
        let response = duplicate.header(false).unwrap();
        assert!(response.status == 400u16);
        duplicate.discard_body(response.length).unwrap();
        duplicate.eof().unwrap();
        let mut early = Connection::new(target.connect().unwrap());
        early.send(b"POST /entries HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1073741824\r\n\r\n").unwrap();
        let _ = read(&mut early, 401u16);
        early.eof().unwrap();
        let value = request(&mut connection, &token, "POST", "/entries", "{\"text\":\"alpha\"}", 201u16);
        entry(&value, 1usize, 1usize, "alpha");
        let mut pipeline = encoded(&token, "POST", "/entries", "{\"text\":\"snow \\u2603\\n\\u0000\"}");
        pipeline.extend_from_slice(&encoded(&token, "GET", "/entries/2", ""));
        connection.send(&pipeline).unwrap();
        entry(&read(&mut connection, 201u16), 2usize, 2usize, "snow ☃\n\0");
        entry(&read(&mut connection, 200u16), 2usize, 2usize, "snow ☃\n\0");
        let value = request(&mut connection, &token, "PUT", "/entries/1", "{\"text\":\"omega\"}", 200u16);
        entry(&value, 1usize, 3usize, "omega");
        let _ = request(&mut connection, &token, "POST", "/entries", "{\"text\":\"x\",\"t\\u0065xt\":\"y\"}", 400u16);
        let _ = request(&mut connection, &token, "PUT", "/entries/99", "{\"text\":\"absent\"}", 404u16);
    }
    // A valid token still authorizes a write after restart; the missing ID keeps
    // the durable fixture unchanged for the framing/recovery cases.
    let _ = request(&mut connection, &token, "PUT", "/entries/99", "{\"text\":\"absent\"}", 404u16);
    if mode == "add" {
        let mut text = String::new();
        for _index in 0usize..20000usize { text.push('x'); }
        let body = json::stringify(&Value::Object(vec![("text".to_string(), Value::String(text.clone()))])).unwrap();
        let value = request(&mut connection, &token, "POST", "/entries", &body, 201u16);
        entry(&value, 3usize, 4usize, &text);
    } else {
        entry(&request(&mut connection, &token, "GET", "/entries/1", "", 200u16), 1usize, 3usize, "omega");
        entry(&request(&mut connection, &token, "GET", "/entries/2", "", 200u16), 2usize, 2usize, "snow ☃\n\0");
        list(&request(&mut connection, &token, "GET", "/entries/1/3", "", 200u16), 2usize);
        list(&request(&mut connection, &token, "GET", "/entries/2/3", "", 200u16), 1usize);
        connection.send(&encoded(&token, "HEAD", "/entries/1", "")).unwrap();
        let response = connection.header(true).unwrap();
        assert!(response.status == 200u16 && response.length > 0usize);
        let status = request(&mut connection, &token, "GET", "/status", "", 200u16);
        let extra = mode == "extra";
        if extra {
            let value = request(&mut connection, &token, "GET", "/entries/3", "", 200u16);
            let text = value.get("text").unwrap().as_str().unwrap();
            assert!(text.len() == 20000usize);
            for byte in text.as_bytes() { assert!(*byte == b'x'); }
        }
        assert!(status.get("revision").unwrap().as_usize() == Some(if extra { 4usize } else { 3usize }));
        assert!(status.get("entries").unwrap().as_usize() == Some(if extra { 3usize } else { 2usize }));
    }
    world.stdout().write(format!("ok journal {mode}\n").as_bytes());
}
