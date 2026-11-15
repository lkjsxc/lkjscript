//! A buffered, strict HTTP/1 response reader shared by the native clients.
//! Content-Length frames persistent responses; HEAD and no-body status codes
//! consume no body. The remaining bytes stay available for the next response.
pub struct Response {
    pub header: Vec<u8>,
    pub status: u16,
    pub length: usize,
    pub close: bool,
}
pub struct Connection {
    pub stream: File,
    buffer: [u8; 65536],
    start: usize,
    end: usize,
}
pub fn io<T>(value: Result<T, i64>, operation: &str) -> Result<T, String> {
    match value {
        Ok(value) => Ok(value),
        Err(errno) => Err(format!("{operation}: errno {errno}")),
    }
}
pub fn require(value: bool, message: &str) -> Result<(), String> {
    if value { Ok(()) } else { Err(message.to_string()) }
}
pub fn find(bytes: &[u8], needle: &[u8]) -> Option<usize> {
    if bytes.len() < needle.len() { return None; }
    let mut at = 0usize;
    while at <= bytes.len() - needle.len() {
        if &bytes[at..at + needle.len()] == needle { return Some(at); }
        at += 1;
    }
    None
}
fn lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' { byte + 32u8 } else { byte }
}
fn equal_name(bytes: &[u8], text: &[u8]) -> bool {
    if bytes.len() != text.len() { return false; }
    for at in 0usize..bytes.len() {
        if lower(bytes[at]) != text[at] { return false; }
    }
    true
}
fn trim(bytes: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && (bytes[start] == b' ' || bytes[start] == b'\t') { start += 1; }
    while end > start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') { end -= 1; }
    &bytes[start..end]
}
fn decimal(bytes: &[u8]) -> Result<usize, String> {
    require(!bytes.is_empty(), "empty response Content-Length")?;
    let mut value = 0usize;
    for byte in bytes {
        require(*byte >= b'0' && *byte <= b'9', "invalid response Content-Length")?;
        let digit = (*byte - b'0') as usize;
        require(value <= ((!0usize) - digit) / 10usize, "response Content-Length overflow")?;
        value = value * 10usize + digit;
    }
    Ok(value)
}
pub fn parse_header(header: Vec<u8>, head: bool) -> Result<Response, String> {
    let bytes = header.as_slice();
    require(bytes.len() >= 16usize, "short HTTP response")?;
    require(&bytes[..9] == b"HTTP/1.1 " || &bytes[..9] == b"HTTP/1.0 ", "invalid HTTP response version")?;
    require(bytes[9] >= b'1' && bytes[9] <= b'5'
        && bytes[10] >= b'0' && bytes[10] <= b'9'
        && bytes[11] >= b'0' && bytes[11] <= b'9'
        && bytes[12] == b' ', "invalid HTTP response status")?;
    let status = ((bytes[9] - b'0') as u16) * 100u16
        + ((bytes[10] - b'0') as u16) * 10u16 + (bytes[11] - b'0') as u16;
    let mut at = match find(bytes, b"\r\n") {
        Some(end) => end + 2usize,
        None => return Err("missing response status line ending".to_string()),
    };
    let mut length = None;
    let old_version = &bytes[..8] == b"HTTP/1.0";
    let mut explicit_close = false;
    let mut keep_alive = false;
    while at < bytes.len() {
        let end = match find(&bytes[at..], b"\r\n") {
            Some(end) => at + end,
            None => return Err("missing response header line ending".to_string()),
        };
        if end == at {
            require(end + 2usize == bytes.len(), "bytes after response header")?;
            break;
        }
        let line = &bytes[at..end];
        let colon = match find(line, b":") {
            Some(colon) => colon,
            None => return Err("missing response header colon".to_string()),
        };
        let name = &line[..colon];
        let value = trim(&line[colon + 1usize..]);
        if equal_name(name, b"content-length") {
            require(length.is_none(), "duplicate response Content-Length")?;
            length = Some(decimal(value)?);
        } else if equal_name(name, b"transfer-encoding") {
            return Err("unsupported response Transfer-Encoding".to_string());
        } else if equal_name(name, b"connection") {
            let mut token_start = 0usize;
            for index in 0usize..value.len() + 1usize {
                if index == value.len() || value[index] == b',' {
                    if equal_name(trim(&value[token_start..index]), b"close") { explicit_close = true; }
                    if equal_name(trim(&value[token_start..index]), b"keep-alive") { keep_alive = true; }
                    token_start = index + 1usize;
                }
            }
        }
        at = end + 2usize;
    }
    let no_body = head || status < 200u16 || status == 204u16 || status == 304u16;
    let body_length = match length {
        Some(value) => value,
        None if no_body => 0usize,
        None => return Err("response missing Content-Length".to_string()),
    };
    let close = explicit_close || (old_version && !keep_alive);
    Ok(Response { header, status, length: body_length, close })
}
impl Connection {
    pub fn new(stream: File) -> Connection {
        Connection { stream, buffer: [0u8; 65536], start: 0usize, end: 0usize }
    }
    fn available(&mut self) -> Result<usize, String> {
        if self.start == self.end {
            self.start = 0usize;
            self.end = io(self.stream.read(&mut self.buffer), "read response")?;
        }
        Ok(self.end - self.start)
    }
    pub fn send(&mut self, request: &[u8]) -> Result<(), String> {
        io(self.stream.write_all(request), "send request")
    }
    pub fn header(&mut self, head: bool) -> Result<Response, String> {
        let mut header: Vec<u8> = Vec::new();
        let mut tail = 0u32;
        loop {
            require(self.available()? != 0usize, "EOF before response header")?;
            let from = self.start;
            while self.start < self.end {
                tail = (tail << 8u64) | self.buffer[self.start] as u32;
                self.start += 1usize;
                if tail == 0x0d0a0d0au32 {
                    header.extend_from_slice(&self.buffer[from..self.start]);
                    return parse_header(header, head);
                }
            }
            header.extend_from_slice(&self.buffer[from..self.start]);
        }
    }
    pub fn body(&mut self, length: usize) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        while bytes.len() < length {
            let available = self.available()?;
            require(available != 0usize, "EOF inside response body")?;
            let remaining = length - bytes.len();
            let count = if remaining < available { remaining } else { available };
            bytes.extend_from_slice(&self.buffer[self.start..self.start + count]);
            self.start += count;
        }
        Ok(bytes)
    }
    pub fn verify_body(&mut self, expected: &[u8]) -> Result<(), String> {
        let mut offset = 0usize;
        while offset < expected.len() {
            let available = self.available()?;
            require(available != 0usize, "EOF inside verified response body")?;
            let remaining = expected.len() - offset;
            let count = if remaining < available { remaining } else { available };
            require(&self.buffer[self.start..self.start + count] == &expected[offset..offset + count],
                "response body differs from the expected file")?;
            self.start += count;
            offset += count;
        }
        Ok(())
    }
    pub fn discard_body(&mut self, length: usize) -> Result<(), String> {
        let mut remaining = length;
        while remaining > 0usize {
            let available = self.available()?;
            require(available != 0usize, "EOF inside discarded response body")?;
            let count = if remaining < available { remaining } else { available };
            self.start += count;
            remaining -= count;
        }
        Ok(())
    }
    pub fn eof(&mut self) -> Result<(), String> {
        require(self.available()? == 0usize, "unexpected bytes after final response")
    }
}
#[test]
fn response_framing() {
    let value = parse_header(b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\nConnection: keep-alive\r\n\r\n"[..].to_vec(), false).unwrap();
    assert_eq!(value.status, 200u16);
    assert_eq!(value.length, 12usize);
    assert!(!value.close);
    assert!(parse_header(b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\n"[..].to_vec(), false).is_err());
    assert!(parse_header(b"HTTP/1.1 200 OK\r\nContent-Length: -1\r\n\r\n"[..].to_vec(), false).is_err());
    assert!(parse_header(b"HTTP/1.1 200 OK\r\nContent-Length: 18446744073709551616\r\n\r\n"[..].to_vec(), false).is_err());
    assert!(parse_header(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n"[..].to_vec(), false).is_err());
    assert!(parse_header(b"HTTP/1.1 200 OK\r\n\r\n"[..].to_vec(), false).is_err());
    assert_eq!(parse_header(b"HTTP/1.1 304 Not Modified\r\n\r\n"[..].to_vec(), false).unwrap().length, 0usize);
    assert!(parse_header(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: keep-alive, CLOSE\r\n\r\n"[..].to_vec(), false).unwrap().close);
}
