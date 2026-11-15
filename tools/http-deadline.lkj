//! Buffered pipeline deadlines must survive a preceding response's backpressure.
mod http_client;
use http_client::{Connection, io, require};

const SECOND: usize = 1_000_000_000usize;
fn pause(clock: &mut Clock, ready: &mut Readiness, duration: usize) -> Result<(), String> {
    let until = io(clock.nanoseconds(), "pause clock")?.checked_add(duration)
        .ok_or("pause deadline overflow".to_string())?;
    while io(clock.nanoseconds(), "pause clock")? < until {
        io(ready.wait(clock, until), "wait without socket interest")?;
    }
    Ok(())
}
fn small(connection: &mut Connection) -> Result<(), String> {
    let response = connection.header(false)?;
    require(response.status == 200u16 && response.length == 3usize, "expected small successful response")?;
    connection.verify_body(b"ok\n")
}
fn expired(output: &mut Stdout, target: &mut Connector, clock: &mut Clock,
    ready: &mut Readiness, name: &str, prefix: &[u8], suffix: &[u8]) -> Result<(), String> {
    let mut stream = io(target.connect(), "connect delayed pipeline")?;
    let mut batch = b"GET /large.bin HTTP/1.1\r\nHost: localhost\r\n\r\n"[..].to_vec();
    batch.extend_from_slice(prefix);
    // Keep both requests within one loopback write and an 8 KiB server read.
    require(io(stream.write(&batch), "send one pipeline write")? == batch.len(),
        "small pipeline write was unexpectedly partial")?;
    let mut connection = Connection::new(stream);
    let first = connection.header(false)?;
    require(first.status == 200u16 && first.length == 33_554_432usize && !first.close,
        "expected backpressuring 32 MiB response")?;
    let start = io(clock.nanoseconds(), "delayed pipeline clock")?;
    // The fixture's request deadline is 1s, its response-write deadline 10s.
    // Stop reading long enough to expire the already-buffered next request.
    pause(clock, ready, 2usize * SECOND)?;
    connection.discard_body(first.length)?;
    match connection.stream.write(suffix) {
        Ok(count) => require(count == suffix.len(), "request completion write was partial")?,
        Err(32i64) | Err(104i64) => {}
        Err(errno) => return Err(format!("complete expired {name}: errno {errno}")),
    }
    match connection.eof() {
        Ok(()) => {}
        // An unread late suffix may cause a reset; extra response bytes still fail.
        Err(problem) => {
            require(problem == "read response: errno 104",
                &format!("expired buffered {name}: {problem}"))?;
        }
    }
    let elapsed = io(clock.nanoseconds(), "delayed pipeline clock")? - start;
    output.write(format!("ok buffered {name} expired behind response after {elapsed} ns; request deadline 1000000000 ns\n").as_bytes());
    Ok(())
}
fn fresh_arrivals(output: &mut Stdout, target: &mut Connector, clock: &mut Clock,
    ready: &mut Readiness) -> Result<(), String> {
    let mut connection = Connection::new(io(target.connect(), "connect later arrivals")?);
    let first_start = io(clock.nanoseconds(), "first header clock")?;
    connection.send(b"GET /ok.txt HTTP/1.1\r\nHost:")?;
    // Establish event-loop progress while the first incomplete header is queued.
    let mut control = Connection::new(io(target.connect(), "connect control peer")?);
    for _round in 0usize..4usize {
        control.send(b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\n\r\n")?;
        small(&mut control)?;
    }
    drop(control);
    pause(clock, ready, 600_000_000usize)?;
    let later_start = io(clock.nanoseconds(), "later header clock")?;
    require(later_start - first_start < SECOND,
        "timing precondition: scheduling consumed the first request deadline")?;
    // This later read finishes the first header and starts a different request.
    connection.send(b" localhost\r\n\r\nGET /ok.txt HTTP/1.1\r\nHost: localhost\r\n")?;
    small(&mut connection)?;
    pause(clock, ready, 600_000_000usize)?;
    let later_finish = io(clock.nanoseconds(), "later header clock")?;
    require(later_finish - later_start < SECOND,
        "timing precondition: scheduling consumed the later request deadline")?;
    connection.send(b"\r\n")?;
    small(&mut connection)?;
    // An empty keep-alive buffer has the idle deadline, not the prior request's.
    pause(clock, ready, 1_200_000_000usize)?;
    connection.send(b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")?;
    small(&mut connection)?;
    connection.eof()?;
    output.write(b"ok later lookahead uses its own receive time; fresh idle request starts a new deadline\n");
    Ok(())
}
fn exercise(output: &mut Stdout, target: &mut Connector, clock: &mut Clock,
    ready: &mut Readiness) -> Result<(), String> {
    expired(output, target, clock, ready, "header",
        b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\n", b"\r\n")?;
    expired(output, target, clock, ready, "body",
        b"GET /ok.txt HTTP/1.1\r\nHost: localhost\r\nContent-Length: 1\r\n\r\n", b"x")?;
    fresh_arrivals(output, target, clock, ready)
}
pub fn main(mut output: Stdout, mut target: Connector, mut clock: Clock, mut ready: Readiness) {
    if let Err(problem) = exercise(&mut output, &mut target, &mut clock, &mut ready) {
        output.write(format!("HTTP deadline case: {problem}\n").as_bytes());
        panic!("{problem}");
    }
}
