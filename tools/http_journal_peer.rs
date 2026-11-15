//! Nonblocking HTTP exchanges for the crash supervisor.
use crate::http_client::{find, io, parse_header, require, Response};

pub struct Peer {
    pub stream: File,
    pub sent: usize,
    outgoing: Vec<u8>,
    incoming: Vec<u8>,
    pub ended: bool,
}
pub fn request(token: &str, method: &str, path: &str, body: &[u8], close: bool) -> Vec<u8> {
    let connection = if close { "close" } else { "keep-alive" };
    let authorization = crate::http_journal_auth::header(token);
    let mut bytes = format!("{method} {path} HTTP/1.1\r\nHost: localhost\r\n{authorization}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: {connection}\r\n\r\n", body.len()).into_bytes();
    bytes.extend_from_slice(body);
    bytes
}
impl Peer {
    pub fn connect(world: &mut World, port: u16) -> Result<Peer, String> {
        let mut target = io(world.connector("127.0.0.1", port), "grant journal connector")?;
        let mut stream = io(target.connect(), "connect journal")?;
        io(stream.set_nonblocking(true), "nonblocking journal client")?;
        Ok(Peer { stream, sent: 0usize, outgoing: Vec::new(), incoming: Vec::new(), ended: false })
    }
    pub fn queue(&mut self, bytes: &[u8]) -> usize {
        self.outgoing.extend_from_slice(bytes);
        self.outgoing.len()
    }
    pub fn step(&mut self) -> Result<(), String> {
        if self.ended { return Ok(()); }
        if self.sent < self.outgoing.len() {
            match self.stream.write(&self.outgoing[self.sent..]) {
                Ok(count) => { self.sent += count; }
                Err(11i64) | Err(4i64) => {}
                Err(32i64) | Err(104i64) => { self.ended = true; }
                Err(errno) => return Err(format!("write journal request: errno {errno}")),
            }
        }
        let mut bytes = [0u8; 65536];
        match self.stream.read(&mut bytes) {
            Ok(0usize) => { self.ended = true; }
            Ok(count) => { self.incoming.extend_from_slice(&bytes[..count]); }
            Err(11i64) | Err(4i64) => {}
            Err(104i64) => { self.ended = true; }
            Err(errno) => return Err(format!("read journal response: errno {errno}")),
        }
        Ok(())
    }
    pub fn response(&mut self) -> Result<Option<(Response, Vec<u8>)>, String> {
        let end = match find(&self.incoming, b"\r\n\r\n") {
            Some(end) => end + 4usize,
            None => return Ok(None),
        };
        let response = parse_header(self.incoming[..end].to_vec(), false)?;
        let length = end.checked_add(response.length).ok_or("response length overflow".to_string())?;
        if self.incoming.len() < length { return Ok(None); }
        let body = self.incoming[end..length].to_vec();
        let remaining = self.incoming[length..].to_vec();
        self.incoming = remaining;
        Ok(Some((response, body)))
    }
    pub fn wait(&mut self, clock: &mut Clock, ready: &mut Readiness, deadline: usize) -> Result<(), String> {
        let interest = if self.sent < self.outgoing.len() { 3u8 } else { 1u8 };
        io(ready.watch_file(&mut self.stream, 1usize, interest), "watch journal client")?;
        io(ready.wait(clock, deadline), "wait journal client")?;
        Ok(())
    }
    pub fn unwatch(&mut self, ready: &mut Readiness) -> Result<(), String> {
        match ready.unwatch_file(&mut self.stream) {
            Ok(()) | Err(2i64) => Ok(()),
            Err(errno) => Err(format!("unwatch journal client: errno {errno}")),
        }
    }
}
pub fn json(bytes: &[u8]) -> Result<std::json::Value, String> {
    std::json::parse(bytes).map_err(|problem| format!("response JSON: {problem}"))
}
pub fn exchange(world: &mut World, port: u16, clock: &mut Clock, ready: &mut Readiness,
    token: &str, method: &str, path: &str, body: &[u8], expected: u16) -> Result<std::json::Value, String> {
    let mut peer = Peer::connect(world, port)?;
    peer.queue(&request(token, method, path, body, true));
    let deadline = io(clock.nanoseconds(), "start request clock")? + 30_000_000_000usize;
    loop {
        peer.step()?;
        if let Some((response, body)) = peer.response()? {
            peer.unwatch(ready)?;
            require(response.status == expected,
                &format!("{method} {path}: expected {expected}, got {}", response.status))?;
            return json(&body);
        }
        require(!peer.ended, "journal closed before complete response")?;
        require(io(clock.nanoseconds(), "request watchdog")? < deadline, "journal request watchdog expired")?;
        peer.wait(clock, ready, deadline)?;
    }
}
