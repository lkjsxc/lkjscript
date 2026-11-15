//! Graceful-stop clients; the shell signals only its own published endpoint.
mod http_client;
use http_client::{Connection, Response, find};
fn checked<T>(output: &mut Stdout, action: &str, result: Result<T, String>) -> T {
    match result {
        Ok(value) => value,
        Err(problem) => {
            output.write(format!("shutdown case {action}: {problem}\n").as_bytes());
            panic!("shutdown client operation failed");
        }
    }
}
fn closed(connection: &mut Connection) -> Result<(), String> {
    match connection.eof() {
        Ok(()) => Ok(()),
        // A late request is deliberately unread. Linux can reset on that close.
        // eof() still rejects any buffered or newly read extra response bytes.
        Err(problem) => {
            if problem == "read response: errno 104" { Ok(()) } else { Err(problem) }
        }
    }
}
fn request(path: &str, length: usize) -> Vec<u8> {
    format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {length}\r\n\r\n").as_bytes().to_vec()
}
fn date(response: &Response) {
    let start = find(&response.header, b"\r\nDate: ").unwrap() + 8usize;
    let length = find(&response.header[start..], b"\r\n").unwrap();
    assert!(length == 29usize);
    assert!(&response.header[start + 25usize..start + 29usize] == b" GMT");
}
fn small(connection: &mut Connection, close: bool) {
    let response = connection.header(false).unwrap();
    assert!(response.status == 200u16 && response.length == 3usize && response.close == close);
    date(&response);
    let bytes = connection.body(response.length).unwrap();
    assert!(bytes.as_slice() == b"ok\n");
}
fn proceed(input: &mut Stdin) {
    let mut command = [0u8; 1];
    assert!(input.read(&mut command).unwrap() == 1usize);
}
fn interpreted_smoke(target: &mut Connector, output: &mut Stdout, input: &mut Stdin) {
    let mut partial = Connection::new(target.connect().unwrap());
    let mut batch = request("/ok.txt", 0usize);
    batch.extend_from_slice(&request("/ok.txt", 1usize));
    checked(output, "send tiny pipeline", partial.send(&batch));
    small(&mut partial, false);
    let mut control = Connection::new(target.connect().unwrap());
    // The partial body's header is queued before these fair event-loop turns.
    for _round in 0usize..4usize {
        checked(output, "send control request", control.send(&request("/ok.txt", 0usize)));
        small(&mut control, false);
    }
    output.write(b"armed\n");
    proceed(input);
    assert!(target.connect().is_err());
    checked(output, "finish admitted body", partial.send(b"x"));
    small(&mut partial, true);
    checked(output, "partial request closes", partial.eof());
    checked(output, "control idle closes", control.eof());
    output.write(b"drained\nok interpreted shutdown tiny GET, admitted partial request, idle close and repeated SIGTERM\n");
}
pub fn main(mut world: World) {
    let args = world.args();
    let interpreted = args.len() == 2usize && args[1].as_slice() == b"smoke";
    assert!(args.len() == 1usize || interpreted);
    let port = String::from_utf8(args[0].clone()).unwrap().as_str().parse::<u16>().unwrap();
    let mut target = world.connector("127.0.0.1", port).unwrap();
    let mut output = world.stdout();
    let mut input = world.stdin();
    if interpreted {
        interpreted_smoke(&mut target, &mut output, &mut input);
        return;
    }
    let mut idle = target.connect().unwrap();
    let mut pipeline = Connection::new(target.connect().unwrap());
    let mut requests = request("/large.bin", 0usize);
    requests.extend_from_slice(&request("/ok.txt", 0usize));
    requests.extend_from_slice(&request("/ok.txt", 1usize));
    pipeline.send(&requests).unwrap();
    let first = pipeline.header(false).unwrap();
    assert!(first.status == 200u16 && first.length == 33_554_432usize);
    date(&first);
    let mut blocked = Connection::new(target.connect().unwrap());
    blocked.send(&request("/large.bin", 0usize)).unwrap();
    let blocked_header = blocked.header(false).unwrap();
    assert!(blocked_header.status == 200u16 && blocked_header.length == first.length);
    let mut partial = Connection::new(target.connect().unwrap());
    partial.send(&request("/ok.txt", 1usize)).unwrap();
    let mut unfinished = target.connect().unwrap();
    unfinished.write_all(b"G").unwrap();
    let mut control = Connection::new(target.connect().unwrap());
    // Response roundtrips establish that queued connection turns are advancing.
    for _round in 0usize..64usize {
        control.send(&request("/ok.txt", 0usize)).unwrap();
        small(&mut control, false);
    }
    let mut missing = Connection::new(target.connect().unwrap());
    missing.send(&request("/missing.txt", 0usize)).unwrap();
    let error = missing.header(false).unwrap();
    assert!(error.status == 404u16);
    date(&error);
    missing.discard_body(error.length).unwrap();
    drop(missing);
    output.write(b"armed\n");
    proceed(&mut input);
    // The supervisor has observed the server's stopping marker before this byte.
    assert!(target.connect().is_err());
    idle.set_nonblocking(true).unwrap();
    let mut byte = [0u8; 1];
    assert!(matches!(idle.read(&mut byte), Ok(0usize) | Err(104i64)));
    let mut finish = vec![b'x'];
    finish.extend_from_slice(&request("/ok.txt", 0usize));
    checked(&mut output, "finish pipeline body", pipeline.send(&finish));
    checked(&mut output, "finish current body", partial.send(&finish));
    checked(&mut output, "drain first response", pipeline.discard_body(first.length));
    small(&mut pipeline, false);
    small(&mut pipeline, true);
    checked(&mut output, "pipeline closes", closed(&mut pipeline));
    small(&mut partial, true);
    checked(&mut output, "current request closes", closed(&mut partial));
    checked(&mut output, "control idle closes", control.eof());
    output.write(b"drained\n");
    // Keep the nonreader and incomplete request alive until server exit.
    proceed(&mut input);
    unfinished.set_nonblocking(true).unwrap();
    assert!(matches!(unfinished.read(&mut byte), Ok(0usize) | Err(104i64)));
    drop(blocked);
    output.write(b"ok shutdown admitted pipeline, current request, idle close, late request exclusion and fixed drain deadline\n");
}
