//! Sealed documentation origin with embedded public content and dynamic search.
mod http_protocol;
mod http_policy;
mod http_server;
mod site_archive;
mod site_search;
use http_protocol::Request;
use http_server::{Body, Handler, Reply};
use site_archive::Archive;
use site_search::Search;

// The observed origin matches static routes literally, then decodes only the
// single path parameter beneath /docs/. It does not decode encoded separators.
fn route_path(request: &Request) -> String {
    let raw = request.raw_path.as_str();
    if raw.starts_with("/docs/") {
        let slug = &raw[6usize..];
        if !slug.is_empty() && !slug.contains('/') { return format!("/{}", request.path); }
    }
    request.raw_path.clone()
}
struct Site { archive: Archive, search: Search }
impl Site {
    fn reply(&self, index: Option<usize>, body: Body, search_status: Option<u16>) -> Reply {
        let page = self.archive.page(index);
        let kind = match search_status {
            Some(400u16) => "text/plain; charset=utf-8".to_string(),
            _ => page.kind.clone(),
        };
        let mut headers: Vec<(String, String)> = vec![("content-type".to_string(), kind)];
        for header in page.headers.clone() { headers.push(header); }
        Reply::from_headers(search_status.unwrap_or(page.status), body, headers)
    }
}
impl Handler for Site {
    fn methods(&self) -> &'static str { "GET, HEAD" }
    fn preflight(&mut self, _request: &Request, _now: i64) -> Option<Reply> { None }
    fn retain_body(&self, _request: &Request) -> bool { false }
    fn finish(&mut self) -> Result<(), String> { Ok(()) }
    fn respond(&mut self, request: &Request, _body: Vec<u8>, _now: i64) -> Result<Reply, String> {
        // Lookup selects only the embedded public pages. Dot components,
        // unknown paths and trailing slashes select the archived 404 response.
        let path = route_path(request);
        let index = self.archive.index(&path);
        if request.raw_path == "/search" {
            let (status, rendered) = self.search.render(&request.query);
            let body = if request.head { Body::Message(String::new()) } else { Body::Message(rendered) };
            return Ok(self.reply(index, body, Some(status)));
        }
        let body = if request.head {
            // Preserve the observed origin's HEAD convention: Content-Length 0.
            Body::Message(String::new())
        } else {
            Body::Message(self.archive.read(index)?)
        };
        Ok(self.reply(index, body, None))
    }
}
pub fn main(mut output: Stdout, listener: Listener, clock: Clock,
    ready: Readiness, stop: Stop) {
    let policy = http_policy::Policy {
        idle: 60_000_000_000usize, request: 2_000_000_000usize,
        write: 2_000_000_000usize, shutdown: 5_000_000_000usize,
    };
    let mut archive = match Archive::open() {
        Ok(archive) => archive,
        Err(problem) => {
            output.write(format!("site archive: {problem}\n").as_bytes());
            panic!("site archive failed");
        }
    };
    let search = match Search::open(&mut archive) {
        Ok(search) => search,
        Err(problem) => {
            output.write(format!("site search: {problem}\n").as_bytes());
            panic!("site search initialization failed");
        }
    };
    http_server::serve(output, listener, clock, ready, stop, policy, Site { archive, search });
}

#[test]
fn raw_route_aliases() {
    let cases: Vec<(String, String)> = vec![
        ("/%73earch".to_string(), "/%73earch".to_string()),
        ("/assets/%73ite.css".to_string(), "/assets/%73ite.css".to_string()),
        ("/docs%2fstart".to_string(), "/docs%2fstart".to_string()),
        ("/docs/%73tart".to_string(), "/docs/start".to_string()),
        ("/docs/%73tart/".to_string(), "/docs/%73tart/".to_string()),
    ];
    for (target, expected) in cases {
        let header = format!("GET {target}?q=cat HTTP/1.1\r\nHost: a\r\n\r\n");
        let request = http_protocol::parse(header.as_bytes()).unwrap();
        assert!(route_path(&request) == expected);
    }
}
