//! Shared HTTP connection states, framing, readiness and fixed deadlines.
use crate::http_policy::Policy;
use crate::http_protocol::{header_end, parse_with_methods, Request};
use std::buffer::{ReadBuffer, WriteCursor};
use std::timers::DeadlineQueue;

const TURN_STEPS: usize = 16usize;
const ACCEPT_STEPS: usize = 64usize;
const EXPIRE_STEPS: usize = 64usize;
const ACCEPT_RETRY_NS: usize = 100_000_000usize;
const STOP_TOKEN: usize = !0usize;

fn deadline(now: usize, duration: usize) -> usize {
    now.checked_add(duration).unwrap_or(!0usize)
}
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        417 => "Expectation Failed",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "Internal Server Error",
    }
}
pub enum Body {
    Message(String),
    File(ReadBuffer),
}
pub struct Reply {
    status: u16,
    kind: String,
    body: Body,
    headers: Vec<(String, String)>,
}
impl Reply {
    pub fn new(status: u16, kind: &str, body: Body) -> Reply {
        Reply { status, kind: kind.to_string(), body, headers: Vec::new() }
    }
    pub fn header(mut self, name: &str, value: &str) -> Reply {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }
    pub fn error(status: u16) -> Reply {
        Reply::new(status, "text/plain; charset=utf-8",
            Body::Message(format!("{status} {}\n", reason(status))))
    }
}
pub trait Handler {
    fn methods(&self) -> &'static str;
    fn preflight(&mut self, request: &Request, now: i64) -> Option<Reply>;
    fn retain_body(&self, request: &Request) -> bool;
    fn respond(&mut self, request: &Request, body: Vec<u8>, now: i64) -> Result<Reply, String>;
    fn finish(&mut self) -> Result<(), String>;
}
struct Response {
    headers: String,
    header_offset: usize,
    body: Body,
    body_offset: usize,
    cursor: WriteCursor,
    head: bool,
    close: bool,
}
impl Response {
    fn new(status: u16, head: bool, keep_alive: bool, elapsed: usize, kind: &str, body: Body, methods: &str, date: &str, extra: Vec<(String, String)>) -> Response {
        let length = match &body {
            Body::Message(text) => text.len(),
            Body::File(bytes) => bytes.len(),
        };
        let connection = if keep_alive { "keep-alive" } else { "close" };
        let allow = if status == 405u16 { format!("Allow: {methods}\r\n") } else { String::new() };
        let mut headers = format!(
            "HTTP/1.1 {status} {}\r\nContent-Length: {length}\r\nContent-Type: {kind}\r\nConnection: {connection}\r\nDate: {date}\r\nX-Content-Type-Options: nosniff\r\nX-Elapsed-Nanoseconds: {elapsed}\r\n{allow}",
            reason(status)
        );
        for (name, value) in extra { headers.push_str(&format!("{name}: {value}\r\n")); }
        headers.push_str("\r\n");
        Response { headers, header_offset: 0usize, body, body_offset: 0usize,
            cursor: WriteCursor::new(), head, close: !keep_alive }
    }
    fn error(status: u16, head: bool, keep_alive: bool, elapsed: usize, methods: &str, date: &str) -> Response {
        Response::new(status, head, keep_alive, elapsed, "text/plain; charset=utf-8",
            Body::Message(format!("{status} {}\n", reason(status))), methods, date, Vec::new())
    }
    fn close_after(&mut self) {
        self.close = true;
        if self.header_offset == 0usize {
            let headers = { self.headers.replace("Connection: keep-alive\r\n", "Connection: close\r\n") };
            self.headers = headers;
        }
    }
    // At most one nonblocking syscall. False means more output remains.
    fn write_step(&mut self, stream: &mut File) -> Result<bool, i64> {
        if self.header_offset < self.headers.len() {
            let count = stream.write(&self.headers.as_bytes()[self.header_offset..])?;
            if count == 0usize { return Err(5i64); }
            self.header_offset += count;
            return Ok(false);
        }
        if self.head { return Ok(true); }
        match &self.body {
            Body::Message(text) => {
                if self.body_offset == text.len() { return Ok(true); }
                let count = stream.write(&text.as_bytes()[self.body_offset..])?;
                if count == 0usize { return Err(5i64); }
                self.body_offset += count;
                Ok(self.body_offset == text.len())
            }
            Body::File(bytes) => {
                if self.cursor.position() == bytes.len() { return Ok(true); }
                bytes.write_some(stream, &mut self.cursor)?;
                Ok(self.cursor.position() == bytes.len())
            }
        }
    }
}
#[derive(Clone, Copy)]
enum Phase { Idle, Header, Body, Write }
enum Step { Again, Wait, Close, Failure(String) }
#[derive(Clone, Copy)]
struct Drain { through: usize, until: usize }

struct Connection {
    stream: File,
    input: Vec<u8>,
    consumed: usize,
    scan: usize,
    request: Option<Request>,
    body_left: usize,
    retaining: bool,
    body: Vec<u8>,
    response: Option<Response>,
    phase: Phase,
    started: usize,
    expires: usize,
    interest: u8,
    queued: bool,
    failed: bool,
    drain: Option<Drain>,
    policy: Policy,
}
impl Connection {
    fn new(stream: File, now: usize, policy: Policy) -> Connection {
        Connection { stream, input: Vec::new(), consumed: 0usize, scan: 0usize,
            request: None, body_left: 0usize, retaining: false, body: Vec::new(), response: None, phase: Phase::Idle,
            started: now, expires: deadline(now, policy.idle), interest: 0u8,
            queued: false, failed: false, drain: None, policy }
    }
    fn clamp_deadline(&mut self) {
        if let Some(drain) = self.drain {
            if drain.until < self.expires { self.expires = drain.until; }
        }
    }
    fn more_admitted(&self) -> bool {
        match self.drain { Some(drain) => drain.through > self.consumed, None => true }
    }
    fn shutdown(&mut self, until: usize) -> bool {
        if matches!(self.phase, Phase::Idle) && self.input.is_empty() { return false; }
        self.drain = Some(Drain { through: self.input.len(), until });
        self.clamp_deadline();
        if !self.more_admitted() {
            if let Some(response) = self.response.as_mut() { response.close_after(); }
        }
        true
    }
    fn update_interest(&mut self, ready: &mut Readiness, token: usize) -> Result<(), i64> {
        let wanted = match self.phase { Phase::Write => 2u8, _ => 1u8 };
        if wanted != self.interest {
            ready.watch_file(&mut self.stream, token, wanted)?;
            self.interest = wanted;
        }
        Ok(())
    }
    fn begin_write(&mut self, response: Response, now: usize) {
        self.response = Some(response);
        self.phase = Phase::Write;
        self.expires = deadline(now, self.policy.write);
        self.clamp_deadline();
    }
    fn bad_request(&mut self, status: u16, clock: &mut Clock, methods: &str) -> Result<Step, i64> {
        let now = clock.nanoseconds()?;
        let elapsed = now - self.started;
        let head = self.input.len() >= 5usize && &self.input[..5] == b"HEAD ";
        let wall = clock.unix_seconds()?;
        let date = match std::time::http_date(wall) {
            Ok(date) => date,
            Err(problem) => return Ok(Step::Failure(problem)),
        };
        self.begin_write(Response::error(status, head, false, elapsed, methods, &date), now);
        Ok(Step::Again)
    }
    fn prepare<H: Handler>(&mut self, handler: &mut H, clock: &mut Clock) -> Result<Step, i64> {
        let request = std::mem::replace(&mut self.request, None).unwrap();
        let body = std::mem::replace(&mut self.body, Vec::new());
        let wall = clock.unix_seconds()?;
        let date = match std::time::http_date(wall) {
            Ok(date) => date,
            Err(problem) => return Ok(Step::Failure(problem)),
        };
        let reply = match handler.respond(&request, body, wall) {
            Ok(reply) => reply,
            Err(problem) => return Ok(Step::Failure(problem)),
        };
        let now = clock.nanoseconds()?;
        let elapsed = now - self.started;
        let keep_alive = request.keep_alive && self.more_admitted();
        self.begin_write(Response::new(reply.status, request.head, keep_alive, elapsed,
            &reply.kind, reply.body, handler.methods(), &date, reply.headers), now);
        Ok(Step::Again)
    }
    fn read_header_step<H: Handler>(&mut self, handler: &mut H, clock: &mut Clock) -> Result<Step, i64> {
        let found = { header_end(&self.input, self.scan) };
        if let Some(end) = found {
            let parsed = { parse_with_methods(&self.input[..end], handler.methods()) };
            let request = match parsed {
                Ok(request) => request,
                Err(status) => return self.bad_request(status, clock, handler.methods()),
            };
            let wall = clock.unix_seconds()?;
            if let Some(reply) = handler.preflight(&request, wall) {
                let date = match std::time::http_date(wall) {
                    Ok(date) => date,
                    Err(problem) => return Ok(Step::Failure(problem)),
                };
                let now = clock.nanoseconds()?;
                let elapsed = now - self.started;
                // Reject before retaining or waiting for this request's body.
                // Its unread framing requires closing after the response.
                self.begin_write(Response::new(reply.status, request.head, false, elapsed,
                    &reply.kind, reply.body, handler.methods(), &date, reply.headers), now);
                return Ok(Step::Again);
            }
            self.retaining = handler.retain_body(&request);
            self.consumed = end;
            self.body_left = request.body;
            self.request = Some(request);
            self.phase = Phase::Body;
            return self.body_step(&mut *handler, clock);
        }
        self.scan = if self.input.len() > 3usize { self.input.len() - 3usize } else { 0usize };
        let mut buffer = [0u8; 8192];
        let count = self.stream.read(&mut buffer)?;
        if count == 0usize {
            if self.input.is_empty() { return Ok(Step::Close); }
            return self.bad_request(400u16, clock, handler.methods());
        }
        if self.input.is_empty() {
            let now = clock.nanoseconds()?;
            self.started = now;
            self.expires = deadline(now, self.policy.request);
            self.phase = Phase::Header;
            self.clamp_deadline();
        }
        self.input.extend_from_slice(&buffer[..count]);
        Ok(Step::Again)
    }
    fn body_step<H: Handler>(&mut self, handler: &mut H, clock: &mut Clock) -> Result<Step, i64> {
        let available = self.input.len() - self.consumed;
        let buffered = if self.body_left < available { self.body_left } else { available };
        if self.retaining {
            self.body.extend_from_slice(&self.input[self.consumed..self.consumed + buffered]);
        }
        self.consumed += buffered;
        self.body_left -= buffered;
        if self.body_left == 0usize { return self.prepare(&mut *handler, clock); }
        let mut buffer = [0u8; 8192];
        let wanted = if self.body_left < buffer.len() { self.body_left } else { buffer.len() };
        let count = self.stream.read(&mut buffer[..wanted])?;
        if count == 0usize { return self.bad_request(400u16, clock, handler.methods()); }
        if self.retaining { self.body.extend_from_slice(&buffer[..count]); }
        self.body_left -= count;
        if self.body_left == 0usize { self.prepare(&mut *handler, clock) } else { Ok(Step::Again) }
    }
    fn next_request(&mut self, now: usize) {
        if let Some(mut drain) = self.drain {
            drain.through = if drain.through > self.consumed { drain.through - self.consumed } else { 0usize };
            self.drain = Some(drain);
        }
        self.response = None;
        self.request = None;
        if self.consumed == self.input.len() {
            self.input.clear();
        } else {
            let remaining = { self.input[self.consumed..].to_vec() };
            self.input = remaining;
        }
        self.consumed = 0usize;
        self.scan = 0usize;
        self.body_left = 0usize;
        self.retaining = false;
        self.started = now;
        if self.input.is_empty() {
            self.phase = Phase::Idle;
            self.expires = deadline(now, self.policy.idle);
        } else {
            self.phase = Phase::Header;
            self.expires = deadline(now, self.policy.request);
        }
        self.clamp_deadline();
    }
    fn write_step(&mut self, clock: &mut Clock) -> Result<Step, i64> {
        let mut response = std::mem::replace(&mut self.response, None).unwrap();
        let result = response.write_step(&mut self.stream);
        let close = response.close;
        self.response = Some(response);
        match result {
            Err(errno) => Err(errno),
            Ok(false) => Ok(Step::Again),
            Ok(true) => {
                if close || !self.more_admitted() { return Ok(Step::Close); }
                self.next_request(clock.nanoseconds()?);
                Ok(Step::Again)
            }
        }
    }
    fn turn<H: Handler>(&mut self, handler: &mut H, clock: &mut Clock) -> Result<Step, i64> {
        for _step in 0usize..TURN_STEPS {
            if self.failed || clock.nanoseconds()? >= self.expires { return Ok(Step::Close); }
            let result = match self.phase {
                Phase::Idle | Phase::Header => self.read_header_step(&mut *handler, clock),
                Phase::Body => self.body_step(&mut *handler, clock),
                Phase::Write => self.write_step(clock),
            };
            match result {
                Err(11i64) => return Ok(Step::Wait),
                Err(errno) => return Err(errno),
                Ok(Step::Wait) => return Ok(Step::Wait),
                Ok(Step::Close) => return Ok(Step::Close),
                Ok(Step::Failure(problem)) => return Ok(Step::Failure(problem)),
                Ok(Step::Again) => {}
            }
        }
        Ok(Step::Again)
    }
}
struct Connections {
    slots: Vec<Option<Connection>>,
    free: Vec<usize>,
    retired: Vec<usize>,
    runnable: Vec<usize>,
    timers: DeadlineQueue,
    policy: Policy,
}
impl Connections {
    fn new(policy: Policy) -> Connections {
        Connections { slots: Vec::new(), free: Vec::new(), retired: Vec::new(),
            runnable: Vec::new(), timers: DeadlineQueue::new(), policy }
    }
    fn close(&mut self, mut connection: Connection, index: usize, ready: &mut Readiness) {
        if connection.interest != 0u8 { let _ = ready.unwatch_file(&mut connection.stream); }
        self.timers.cancel(index);
        self.retired.push(index);
    }
    fn expire(&mut self, now: usize, ready: &mut Readiness) {
        for _step in 0usize..EXPIRE_STEPS {
            let index = match self.timers.due(now) { Some(index) => index, None => break };
            if let Some(connection) = std::mem::replace(&mut self.slots[index], None) {
                self.close(connection, index, ready);
            }
        }
    }
    fn accept(&mut self, listener: &mut Listener, ready: &mut Readiness, clock: &mut Clock) -> Result<(), i64> {
        for _step in 0usize..ACCEPT_STEPS {
            let mut stream = match listener.accept_nodelay() {
                Ok(stream) => stream,
                Err(11i64) => return Ok(()),
                Err(errno) => return Err(errno),
            };
            stream.set_nonblocking(true)?;
            let now = clock.nanoseconds()?;
            let index = match self.free.pop() {
                Some(index) => index,
                None => { self.slots.push(None); self.slots.len() - 1usize }
            };
            let mut connection = Connection::new(stream, now, self.policy);
            match connection.update_interest(ready, index + 1usize) {
                Ok(()) => {
                    self.timers.schedule(index, connection.expires);
                    self.slots[index] = Some(connection);
                }
                Err(errno) => {
                    self.retired.push(index);
                    return Err(errno);
                }
            }
        }
        Ok(())
    }
    fn queue(&mut self, token: usize, flags: u8) {
        if token == 0usize || token - 1usize >= self.slots.len() { return; }
        let index = token - 1usize;
        if let Some(mut connection) = std::mem::replace(&mut self.slots[index], None) {
            // A read half-close still permits replies to buffered requests.
            // The next actual read observes EOF after their bytes are consumed.
            if flags & 8u8 != 0u8 { connection.failed = true; }
            if !connection.queued {
                connection.queued = true;
                self.runnable.push(index);
            }
            self.slots[index] = Some(connection);
        }
    }
    fn run<H: Handler>(&mut self, handler: &mut H, clock: &mut Clock, ready: &mut Readiness) -> Option<String> {
        let work = std::mem::replace(&mut self.runnable, Vec::new());
        for index in work {
            if let Some(mut connection) = std::mem::replace(&mut self.slots[index], None) {
                connection.queued = false;
                let result = connection.turn(&mut *handler, clock);
                let mut again = false;
                let alive = match result {
                    Ok(Step::Again) => { again = true; true }
                    Ok(Step::Wait) => true,
                    Ok(Step::Failure(problem)) => {
                        self.close(connection, index, ready);
                        return Some(problem);
                    }
                    _ => false,
                };
                if alive && connection.update_interest(ready, index + 1usize).is_ok() {
                    self.timers.schedule(index, connection.expires);
                    if again {
                        connection.queued = true;
                        self.runnable.push(index);
                    }
                    self.slots[index] = Some(connection);
                } else {
                    self.close(connection, index, ready);
                }
            }
        }
        None
    }
    fn shutdown(&mut self, until: usize, ready: &mut Readiness) {
        for index in 0usize..self.slots.len() {
            if let Some(mut connection) = std::mem::replace(&mut self.slots[index], None) {
                if connection.shutdown(until) {
                    self.timers.schedule(index, connection.expires);
                    if !connection.queued {
                        connection.queued = true;
                        self.runnable.push(index);
                    }
                    self.slots[index] = Some(connection);
                } else {
                    self.close(connection, index, ready);
                }
            }
        }
    }
    fn is_empty(&self) -> bool { self.timers.next_deadline().is_none() }
    fn release_retired(&mut self) {
        // The entire returned kernel event batch and prior runnable list have
        // been consumed. No old token can target a newly reused slot.
        for index in std::mem::replace(&mut self.retired, Vec::new()) { self.free.push(index); }
    }
}
pub fn serve<H: Handler>(mut output: Stdout, mut listener: Listener,
    mut clock: Clock, mut ready: Readiness, mut stop: Stop, policy: Policy, mut handler: H) {
    listener.set_nonblocking(true).unwrap();
    ready.watch_listener(&mut listener, 0usize).unwrap();
    ready.watch_stop(&mut stop, STOP_TOKEN).unwrap();
    let port = listener.port().unwrap();
    let target = stop.target().unwrap();
    output.write(format!("port {port}\nstop-target {target}\n").as_bytes());
    let mut accepting = Some(listener);
    let mut connections = Connections::new(policy);
    let mut accept_retry = None;
    let mut stopping = false;
    let mut check_stop = true;
    loop {
        let now = clock.nanoseconds().unwrap();
        if check_stop && !stopping {
            check_stop = false;
            if stop.requested().unwrap() {
                stopping = true;
                if let Some(mut listener) = accepting.take() {
                    let _ = ready.unwatch_listener(&mut listener);
                }
                accept_retry = None;
                let _ = ready.unwatch_stop(&mut stop);
                let until = deadline(now, policy.shutdown);
                connections.shutdown(until, &mut ready);
                output.write(format!("stopping {until}\n").as_bytes());
            }
        }
        connections.expire(now, &mut ready);
        if stopping && connections.is_empty() { break; }
        if let Some(retry) = accept_retry {
            if now >= retry {
                if let Some(listener) = accepting.as_mut() {
                    ready.watch_listener(&mut *listener, 0usize).unwrap();
                }
                accept_retry = None;
            }
        }
        let mut until = connections.timers.next_deadline().unwrap_or(!0usize);
        if let Some(retry) = accept_retry { if retry < until { until = retry; } }
        if !connections.runnable.is_empty() { until = now; }
        let events = ready.wait(&mut clock, until).unwrap();
        // Observe stop before accepting or advancing any other returned event.
        for event in &events { if event.0 == STOP_TOKEN { check_stop = true; } }
        if check_stop { continue; }
        for (token, flags) in events {
            if token == 0usize {
                if accept_retry.is_none() {
                    if let Some(listener) = accepting.as_mut() {
                        if connections.accept(&mut *listener, &mut ready, &mut clock).is_err() {
                            let _ = ready.unwatch_listener(&mut *listener);
                            accept_retry = Some(deadline(clock.nanoseconds().unwrap(), ACCEPT_RETRY_NS));
                        }
                    }
                }
            } else { connections.queue(token, flags); }
        }
        if let Some(problem) = connections.run(&mut handler, &mut clock, &mut ready) {
            output.write(format!("HTTP application error: {problem}\n").as_bytes());
            panic!("HTTP application failed");
        }
        connections.release_retired();
    }
    if let Err(problem) = handler.finish() {
        output.write(format!("HTTP shutdown error: {problem}\n").as_bytes());
        panic!("HTTP shutdown failed");
    }
    output.write(b"stopped\n");
}
