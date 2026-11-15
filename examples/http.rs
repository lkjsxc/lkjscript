//! Static files below the granted Directory, with framed persistent connections.
mod http_protocol;
use http_protocol::{header_end, parse, Request};
use std::buffer::ReadBuffer;
fn read_header(client: &mut File, bytes: &mut Vec<u8>) -> Result<usize, i64> {
    let mut buffer = [0u8; 8192];
    let mut start = 0usize;
    loop {
        if let Some(end) = header_end(bytes, start) {
            return Ok(end);
        }
        start = if bytes.len() > 3 { bytes.len() - 3 } else { 0 };
        let count = client.read(&mut buffer)?;
        if count == 0 {
            return Ok(bytes.len());
        }
        bytes.extend_from_slice(&buffer[..count]);
    }
}
fn drain_body(
    client: &mut File,
    bytes: &mut Vec<u8>,
    header: usize,
    body: usize,
) -> Result<bool, i64> {
    let available = bytes.len() - header;
    if body <= available {
        let remaining = bytes[header + body..].to_vec();
        *bytes = remaining;
        return Ok(true);
    }
    let mut remaining = body - available;
    bytes.clear();
    let mut buffer = [0u8; 8192];
    while remaining > 0 {
        let wanted = if remaining < buffer.len() {
            remaining
        } else {
            buffer.len()
        };
        let count = client.read(&mut buffer[..wanted])?;
        if count == 0 {
            return Ok(false);
        }
        remaining -= count;
    }
    Ok(true)
}
fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        417 => "Expectation Failed",
        501 => "Not Implemented",
        505 => "HTTP Version Not Supported",
        _ => "Internal Server Error",
    }
}
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
fn send_header(
    client: &mut File,
    clock: &mut Clock,
    start: usize,
    status: u16,
    keep_alive: bool,
    kind: &str,
    length: usize,
) -> Result<(), i64> {
    let elapsed = clock.nanoseconds()? - start;
    let connection = if keep_alive { "keep-alive" } else { "close" };
    let allow = if status == 405 {
        "Allow: GET, HEAD\r\n"
    } else {
        ""
    };
    let headers = format!(
        "HTTP/1.1 {status} {}\r\nContent-Length: {length}\r\nContent-Type: {kind}\r\nConnection: {connection}\r\nX-Content-Type-Options: nosniff\r\nX-Elapsed-Nanoseconds: {elapsed}\r\n{allow}\r\n",
        reason(status)
    );
    client.write_all(headers.as_bytes())
}
fn send(
    client: &mut File,
    clock: &mut Clock,
    start: usize,
    status: u16,
    head: bool,
    keep_alive: bool,
    kind: &str,
    body: &[u8],
) -> Result<(), i64> {
    send_header(client, clock, start, status, keep_alive, kind, body.len())?;
    if !head { client.write_all(body)?; }
    Ok(())
}
fn error(
    client: &mut File,
    clock: &mut Clock,
    start: usize,
    status: u16,
    head: bool,
    keep_alive: bool,
) -> Result<(), i64> {
    let body = format!("{status} {}\n", reason(status));
    send(
        client,
        clock,
        start,
        status,
        head,
        keep_alive,
        "text/plain; charset=utf-8",
        body.as_bytes(),
    )
}
fn respond(
    client: &mut File,
    root: &mut Directory,
    clock: &mut Clock,
    start: usize,
    request: &Request,
) -> Result<(), i64> {
    let mut file = match root.open(&request.path) {
        Ok(file) => file,
        Err(errno) => {
            let status = match errno {
                1 | 13 | 18 | 40 => 403u16,
                2 | 20 | 21 => 404u16,
                _ => 500u16,
            };
            return error(client, clock, start, status, request.head, request.keep_alive);
        }
    };
    let body = match ReadBuffer::read(&mut file) {
        Ok(body) => body,
        Err(errno) => {
            return error(
                client,
                clock,
                start,
                if errno == 21 { 404u16 } else { 500u16 },
                request.head,
                request.keep_alive,
            );
        }
    };
    send_header(
        client,
        clock,
        start,
        200u16,
        request.keep_alive,
        content_type(&request.path),
        body.len(),
    )?;
    if !request.head { body.write_to(client)?; }
    Ok(())
}
fn serve(client: &mut File, root: &mut Directory, clock: &mut Clock) -> Result<(), i64> {
    let mut bytes: Vec<u8> = Vec::new();
    loop {
        let start = clock.nanoseconds()?;
        let end = read_header(client, &mut bytes)?;
        if bytes.is_empty() {
            return Ok(());
        }
        let request = match parse(&bytes[..end]) {
            Ok(request) => request,
            Err(status) => {
                return error(
                    client,
                    clock,
                    start,
                    status,
                    bytes.len() >= 5 && &bytes[..5] == b"HEAD ",
                    false,
                );
            }
        };
        if !drain_body(client, &mut bytes, end, request.body)? {
            return error(client, clock, start, 400u16, request.head, false);
        }
        respond(client, root, clock, start, &request)?;
        if !request.keep_alive {
            return Ok(());
        }
    }
}
pub fn main(mut output: Stdout, mut root: Directory, mut listener: Listener, mut clock: Clock) {
    let port = listener.port().unwrap();
    output.write(format!("port {port}\n").as_bytes());
    loop {
        let mut client = match listener.accept_nodelay() {
            Ok(client) => client,
            Err(_) => continue,
        };
        let _ = serve(&mut client, &mut root, &mut clock);
    }
}
