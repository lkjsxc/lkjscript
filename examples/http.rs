//! Static documents served by the shared readiness-based HTTP engine.
mod http_protocol;
mod http_policy;
mod http_server;
use http_protocol::Request;
use http_server::{Body, Handler, Reply};
use std::buffer::ReadBuffer;

fn content_type(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".js") {
        "text/javascript; charset=utf-8"
    } else if path.ends_with(".json") {
        "application/json"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".jpg") || path.ends_with(".jpeg") {
        "image/jpeg"
    } else {
        "text/plain; charset=utf-8"
    }
}


struct Documents { root: Directory }
impl Handler for Documents {
    fn methods(&self) -> &'static str { "GET, HEAD" }
    fn retain_body(&self) -> bool { false }
    fn respond(&mut self, request: &Request, _body: Vec<u8>) -> Result<Reply, String> {
        let mut file = match self.root.open(&request.path) {
            Ok(file) => file,
            Err(errno) => {
                let status = match errno {
                    1 | 13 | 18 | 40 => 403u16,
                    2 | 20 | 21 => 404u16,
                    _ => 500u16,
                };
                return Ok(Reply::error(status));
            }
        };
        Ok(match ReadBuffer::read(&mut file) {
            Ok(body) => Reply::new(200u16, content_type(&request.path), Body::File(body)),
            Err(errno) => Reply::error(if errno == 21i64 { 404u16 } else { 500u16 }),
        })
    }
}
pub fn main(output: Stdout, mut root: Directory, listener: Listener,
    clock: Clock, ready: Readiness) {
    let policy = match http_policy::read(&mut root) {
        Ok(policy) => policy,
        Err(problem) => panic!("{problem}"),
    };
    http_server::serve(output, listener, clock, ready, policy, Documents { root });
}
