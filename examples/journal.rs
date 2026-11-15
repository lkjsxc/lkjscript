//! Crash-durable journal service using the shared readiness-based HTTP engine.
mod http_protocol;
mod http_policy;
mod http_server;
mod journal_store;
mod journal_sessions;
use http_protocol::Request;
use http_server::{Body, Handler, Reply};
use journal_store::{Entry, Store};
use journal_sessions::Sessions;
use std::json::{self, Value};

fn number(value: usize) -> Value { Value::Number(format!("{value}")) }
fn entry_value(entry: &Entry) -> Value {
    Value::Object(vec![
        ("id".to_string(), number(entry.id)),
        ("revision".to_string(), number(entry.revision)),
        ("text".to_string(), Value::String(entry.text.clone())),
    ])
}
fn response(status: u16, value: Value) -> Reply {
    let text = match json::stringify(&value) {
        Ok(text) => text,
        Err(problem) => panic!("journal response JSON: {}", problem.message),
    };
    Reply::new(status, "application/json", Body::Message(format!("{text}\n")))
}
fn error(status: u16, message: &str) -> Reply {
    response(status, Value::Object(vec![("error".to_string(), Value::String(message.to_string()))]))
}
fn request_text(bytes: &[u8]) -> Result<String, String> {
    let value = match json::parse(bytes) {
        Ok(value) => value,
        Err(problem) => return Err(format!("JSON byte {}: {}", problem.offset, problem.message)),
    };
    match value {
        Value::Object(mut fields) => {
            if fields.len() != 1usize { return Err("expected exactly one text field".to_string()); }
            let (name, value) = fields.pop().unwrap();
            if name != "text" { return Err("expected text field".to_string()); }
            match value {
                Value::String(text) => Ok(text),
                _ => Err("text must be a JSON string".to_string()),
            }
        }
        _ => Err("expected a JSON object with one text field".to_string()),
    }
}
enum Route { All, One(usize), Range(usize, usize), Status, Missing }
fn route(path: &str) -> Result<Route, String> {
    let mut parts = path.split('/');
    let first = parts.next().unwrap_or("");
    if first == "status" && parts.next().is_none() { return Ok(Route::Status); }
    if first != "entries" { return Ok(Route::Missing); }
    let first = match parts.next() {
        Some(text) => match text.parse::<usize>() {
            Ok(id) => id,
            Err(_) => return Err("entry id must be decimal".to_string()),
        },
        None => return Ok(Route::All),
    };
    if first == 0usize { return Err("entry ids start at 1".to_string()); }
    let end = match parts.next() {
        Some(text) => match text.parse::<usize>() {
            Ok(id) => id,
            Err(_) => return Err("range end must be decimal".to_string()),
        },
        None => return Ok(Route::One(first)),
    };
    if parts.next().is_some() { return Ok(Route::Missing); }
    if end < first { return Err("range end precedes its first id".to_string()); }
    Ok(Route::Range(first, end))
}
struct Journal { store: Store, sessions: Sessions }
impl Journal {
    fn read(&self, route: Route) -> Reply {
        match route {
            Route::Status => response(200u16, Value::Object(vec![
                ("entries".to_string(), number(self.store.entries().len())),
                ("next_id".to_string(), number(self.store.next_id())),
                ("revision".to_string(), number(self.store.revision())),
            ])),
            Route::One(id) => match self.store.get(id) {
                Some(entry) => response(200u16, entry_value(entry)),
                None => error(404u16, "entry not found"),
            },
            Route::All => self.range(1usize, self.store.next_id()),
            Route::Range(first, end) => self.range(first, end),
            Route::Missing => error(404u16, "route not found"),
        }
    }
    fn range(&self, first: usize, end: usize) -> Reply {
        let mut values: Vec<Value> = Vec::new();
        for entry in self.store.entries() {
            if entry.id >= first && entry.id < end { values.push(entry_value(entry)); }
        }
        response(200u16, Value::Object(vec![("entries".to_string(), Value::Array(values))]))
    }
}
impl Handler for Journal {
    fn methods(&self) -> &'static str { "GET, HEAD, POST, PUT" }
    fn retain_body(&self, request: &Request) -> bool {
        request.method == "POST" || request.method == "PUT"
    }
    fn finish(&mut self) -> Result<(), String> { self.store.sync() }
    fn preflight(&mut self, request: &Request, now: i64) -> Option<Reply> {
        if request.method == "POST" || request.method == "PUT" {
            let authorization = match &request.authorization {
                Some(text) => text.as_str(),
                None => "",
            };
            if !self.sessions.authorize(authorization, now) {
                return Some(error(401u16, "a valid unexpired session token is required")
                    .header("WWW-Authenticate", "Bearer realm=\"journal\""));
            }
        }
        None
    }
    fn respond(&mut self, request: &Request, body: Vec<u8>, now: i64) -> Result<Reply, String> {
        // A token can expire while an admitted body is arriving.
        if let Some(reply) = self.preflight(request, now) { return Ok(reply); }
        let route = match route(&request.path) {
            Ok(route) => route,
            Err(message) => return Ok(error(400u16, &message)),
        };
        if request.method == "GET" || request.method == "HEAD" { return Ok(self.read(route)); }
        let id = match route {
            Route::All if request.method == "POST" => 0usize,
            Route::One(id) if request.method == "PUT" => {
                if self.store.get(id).is_none() { return Ok(error(404u16, "entry not found")); }
                id
            }
            Route::Missing => return Ok(error(404u16, "route not found")),
            _ => return Ok(error(405u16, "method is not supported for this route")),
        };
        let text = match request_text(&body) {
            Ok(text) => text,
            Err(message) => return Ok(error(400u16, &message)),
        };
        // An uncertain storage failure stops the process. Recovery resolves its
        // tail before any further mutation or successful HTTP response.
        let (status, id) = if id == 0usize {
            match self.store.append(text) {
                Ok(id) => (201u16, id),
                Err(problem) => return Err(format!("journal append: {problem}")),
            }
        } else {
            match self.store.replace(id, text) {
                Ok(()) => (200u16, id),
                Err(problem) => return Err(format!("journal replace: {problem}")),
            }
        };
        Ok(response(status, entry_value(self.store.get(id).unwrap())))
    }
}
pub fn main(mut output: Stdout, mut root: Directory, listener: Listener,
    clock: Clock, ready: Readiness, stop: Stop, mut entropy: Entropy) {
    let policy = match http_policy::read(&mut root) {
        Ok(policy) => policy,
        Err(problem) => panic!("{problem}"),
    };
    let mut store = match Store::open(root, &mut output) {
        Ok(store) => store,
        Err(problem) => {
            output.write(format!("journal error: {problem}\n").as_bytes());
            panic!("journal recovery failed");
        }
    };
    let opened = { Sessions::open(store.directory(), &mut entropy) };
    let sessions = match opened {
        Ok(sessions) => sessions,
        Err(problem) => {
            output.write(format!("journal sessions: {problem}\n").as_bytes());
            panic!("journal session initialization failed");
        }
    };
    output.write(format!("ready {} {}\n", store.revision(), store.entries().len()).as_bytes());
    http_server::serve(output, listener, clock, ready, stop, policy, Journal { store, sessions });
}

#[test]
fn journal_routes() {
    assert!(matches!(route("entries"), Ok(Route::All)));
    assert!(matches!(route("entries/1"), Ok(Route::One(1usize))));
    assert!(matches!(route("entries/2/5"), Ok(Route::Range(2usize, 5usize))));
    assert!(matches!(route("status"), Ok(Route::Status)));
    assert!(matches!(route("../head"), Ok(Route::Missing)));
    assert!(route("entries/0").is_err());
    assert!(route("entries/no").is_err());
    assert!(route("entries/2/1").is_err());
}
#[test]
fn journal_json_contract() {
    assert!(request_text(b"{\"text\":\"snow \\u2603\\n\"}").unwrap() == "snow ☃\n");
    assert!(request_text(b"{\"text\":null}").is_err());
    assert!(request_text(b"{\"text\":\"x\",\"extra\":1}").is_err());
    assert!(request_text(b"{\"text\":\"x\",\"t\\u0065xt\":\"y\"}").is_err());
    assert!(request_text(b"[\"x\"]").is_err());
}
