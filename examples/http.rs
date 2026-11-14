//! Static files below the granted Directory; one response per connection.
mod http_protocol;
use http_protocol::{header_end, parse};
fn read_header(client: &mut File) -> Result<(Vec<u8>, usize), i64> {
    let mut bytes: Vec<u8> = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let count = client.read(&mut buffer)?;
        if count == 0 {
            let end = bytes.len();
            return Ok((bytes, end));
        }
        let start = if bytes.len() > 3 { bytes.len() - 3 } else { 0 };
        bytes.extend_from_slice(&buffer[..count]);
        if let Some(end) = header_end(&bytes, start) {
            return Ok((bytes, end));
        }
    }
}
fn drain_body(client: &mut File, body: usize, available: usize) -> Result<bool, i64> {
    let mut remaining = if body > available {
        body - available
    } else {
        0
    };
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
fn send(
    client: &mut File,
    clock: &mut Clock,
    start: usize,
    status: u16,
    head: bool,
    kind: &str,
    body: &[u8],
) -> Result<(), i64> {
    let elapsed = clock.nanoseconds()? - start;
    let allow = if status == 405 {
        "Allow: GET, HEAD\r\n"
    } else {
        ""
    };
    let headers = format!(
        "HTTP/1.1 {status} {}\r\nContent-Length: {}\r\nContent-Type: {kind}\r\nConnection: close\r\nX-Content-Type-Options: nosniff\r\nX-Elapsed-Nanoseconds: {elapsed}\r\n{allow}\r\n",
        reason(status),
        body.len()
    );
    client.write_all(headers.as_bytes())?;
    if !head {
        client.write_all(body)?;
    }
    Ok(())
}
fn error(
    client: &mut File,
    clock: &mut Clock,
    start: usize,
    status: u16,
    head: bool,
) -> Result<(), i64> {
    let body = format!("{status} {}\n", reason(status));
    send(
        client,
        clock,
        start,
        status,
        head,
        "text/plain; charset=utf-8",
        body.as_bytes(),
    )
}
fn serve(client: &mut File, root: &mut Directory, clock: &mut Clock) -> Result<(), i64> {
    let start = clock.nanoseconds()?;
    let (bytes, end) = read_header(client)?;
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
            );
        }
    };
    if !drain_body(client, request.body, bytes.len() - end)? {
        return error(client, clock, start, 400u16, request.head);
    }
    let mut file = match root.open(&request.path) {
        Ok(file) => file,
        Err(errno) => {
            let status = match errno {
                1 | 13 | 18 | 40 => 403u16,
                2 | 20 | 21 => 404u16,
                _ => 500u16,
            };
            return error(client, clock, start, status, request.head);
        }
    };
    let body = match file.read_to_end() {
        Ok(body) => body,
        Err(errno) => {
            return error(
                client,
                clock,
                start,
                if errno == 21 { 404u16 } else { 500u16 },
                request.head,
            );
        }
    };
    send(
        client,
        clock,
        start,
        200u16,
        request.head,
        content_type(&request.path),
        &body,
    )
}
pub fn main(mut output: Stdout, mut root: Directory, mut listener: Listener, mut clock: Clock) {
    let port = listener.port().unwrap();
    output.write(format!("port {port}\n").as_bytes());
    loop {
        let mut client = match listener.accept() {
            Ok(client) => client,
            Err(_) => continue,
        };
        let _ = serve(&mut client, &mut root, &mut clock);
    }
}
