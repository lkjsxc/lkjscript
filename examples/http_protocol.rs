//! A persistent static-file HTTP consumer with an explicit authority manifest.
//! Each connection receives one response and closes; all request storage is local.
pub struct Request {
    pub path: String,
    pub head: bool,
    pub body: usize,
}
fn lower(byte: u8) -> u8 {
    if byte >= b'A' && byte <= b'Z' {
        byte + (b'a' - b'A')
    } else {
        byte
    }
}
fn same(bytes: &[u8], text: &[u8]) -> bool {
    if bytes.len() != text.len() {
        return false;
    }
    for index in 0usize..bytes.len() {
        if lower(bytes[index]) != text[index] {
            return false;
        }
    }
    true
}
fn token(byte: u8) -> bool {
    (byte >= b'a' && byte <= b'z')
        || (byte >= b'A' && byte <= b'Z')
        || (byte >= b'0' && byte <= b'9')
        || byte == b'!'
        || byte == b'#'
        || byte == b'$'
        || byte == b'%'
        || byte == b'&'
        || byte == b'\''
        || byte == b'*'
        || byte == b'+'
        || byte == b'-'
        || byte == b'.'
        || byte == b'^'
        || byte == b'_'
        || byte == 96u8
        || byte == b'|'
        || byte == b'~'
}
fn trim(bytes: &[u8]) -> &[u8] {
    let mut start = 0usize;
    let mut end = bytes.len();
    while start < end && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    while end > start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
        end -= 1;
    }
    &bytes[start..end]
}
fn number(bytes: &[u8]) -> Result<usize, u16> {
    if bytes.is_empty() {
        return Err(400u16);
    }
    let mut value = 0usize;
    for byte in bytes {
        if *byte < b'0' || *byte > b'9' {
            return Err(400u16);
        }
        let digit = (*byte - b'0') as usize;
        if value > ((!0usize) - digit) / 10usize {
            return Err(400u16);
        }
        value = value * 10usize + digit;
    }
    Ok(value)
}
fn authority(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let mut colon = None;
    let mut index = 0usize;
    if bytes[0] == b'[' {
        index = 1;
        while index < bytes.len() && bytes[index] != b']' {
            let byte = lower(bytes[index]);
            if !((byte >= b'0' && byte <= b'9')
                || (byte >= b'a' && byte <= b'f')
                || byte == b':'
                || byte == b'.')
            {
                return false;
            }
            index += 1;
        }
        if index == 1 || index >= bytes.len() {
            return false;
        }
        index += 1;
        if index < bytes.len() {
            if bytes[index] != b':' {
                return false;
            }
            colon = Some(index);
        }
    } else {
        while index < bytes.len() {
            let byte = bytes[index];
            if byte == b':' {
                colon = Some(index);
                break;
            }
            if !((byte >= b'a' && byte <= b'z')
                || (byte >= b'A' && byte <= b'Z')
                || (byte >= b'0' && byte <= b'9')
                || byte == b'.'
                || byte == b'-'
                || byte == b'_')
            {
                return false;
            }
            index += 1;
        }
        if index == 0 {
            return false;
        }
    }
    if let Some(at) = colon {
        if at + 1 == bytes.len() {
            return false;
        }
        for byte in &bytes[at + 1..] {
            if *byte < b'0' || *byte > b'9' {
                return false;
            }
        }
    }
    true
}
fn hex(byte: u8) -> Option<u8> {
    if byte >= b'0' && byte <= b'9' {
        Some(byte - b'0')
    } else if lower(byte) >= b'a' && lower(byte) <= b'f' {
        Some(lower(byte) - b'a' + 10u8)
    } else {
        None
    }
}
fn target_path(target: &[u8]) -> Result<String, u16> {
    if target.is_empty() {
        return Err(400u16);
    }
    let mut start = 0usize;
    if target.len() >= 7 && &target[..7] == b"http://" {
        let mut end = 7usize;
        while end < target.len() && target[end] != b'/' && target[end] != b'?' {
            end += 1;
        }
        if !authority(&target[7..end]) {
            return Err(400u16);
        }
        start = end;
    }
    if start > 0 && start == target.len() {
        return Ok("language.md".to_string());
    }
    if target[start] == b'/' {
        start += 1;
    } else if start == 0 || target[start] != b'?' {
        return Err(400u16);
    }
    let mut decoded: Vec<u8> = Vec::new();
    let mut index = start;
    while index < target.len() && target[index] != b'?' {
        let mut byte = target[index];
        if byte == b'#' {
            return Err(400u16);
        }
        if byte == b'%' {
            if target.len() - index < 3 {
                return Err(400u16);
            }
            let first = hex(target[index + 1]).ok_or(400u16)?;
            let second = hex(target[index + 2]).ok_or(400u16)?;
            byte = first * 16u8 + second;
            index += 2;
        }
        if byte == 0u8 || byte == b'\\' || byte < 32u8 || byte == 127u8 {
            return Err(400u16);
        }
        decoded.push(byte);
        index += 1;
    }
    for byte in &target[index..] {
        if *byte <= 32u8 || *byte == 127u8 || *byte == b'#' {
            return Err(400u16);
        }
    }
    if decoded.is_empty() {
        return Ok("language.md".to_string());
    }
    match String::from_utf8(decoded) {
        Ok(path) => Ok(path),
        Err(_) => Err(400u16),
    }
}
fn crlf(bytes: &[u8], start: usize) -> Result<usize, u16> {
    let mut index = start;
    while index < bytes.len() {
        if bytes[index] == b'\r' {
            if index + 1 < bytes.len() && bytes[index + 1] == b'\n' {
                return Ok(index);
            }
            return Err(400u16);
        }
        if bytes[index] == b'\n' {
            return Err(400u16);
        }
        index += 1;
    }
    Err(400u16)
}
pub fn parse(bytes: &[u8]) -> Result<Request, u16> {
    let line_end = crlf(bytes, 0)?;
    let line = &bytes[..line_end];
    let mut first = 0usize;
    while first < line.len() && line[first] != b' ' {
        if !token(line[first]) {
            return Err(400u16);
        }
        first += 1;
    }
    if first == 0 || first == line.len() {
        return Err(400u16);
    }
    let mut second = first + 1;
    while second < line.len() && line[second] != b' ' {
        if line[second] <= 32u8 || line[second] >= 127u8 {
            return Err(400u16);
        }
        second += 1;
    }
    if second == first + 1 || second == line.len() {
        return Err(400u16);
    }
    let version = &line[second + 1..];
    if version != b"HTTP/1.1" && version != b"HTTP/1.0" {
        if version.len() == 8
            && &version[..5] == b"HTTP/"
            && version[5] >= b'0'
            && version[5] <= b'9'
            && version[6] == b'.'
            && version[7] >= b'0'
            && version[7] <= b'9'
        {
            return Err(505u16);
        }
        return Err(400u16);
    }
    let head = &line[..first] == b"HEAD";
    let supported = head || &line[..first] == b"GET";
    let mut host = false;
    let mut length = None;
    let mut transfer = false;
    let mut expectation = false;
    let mut at = line_end + 2;
    loop {
        let end = crlf(bytes, at)?;
        if end == at {
            if end + 2 != bytes.len() {
                return Err(400u16);
            }
            break;
        }
        let field = &bytes[at..end];
        let mut colon = 0usize;
        while colon < field.len() && field[colon] != b':' {
            if !token(field[colon]) {
                return Err(400u16);
            }
            colon += 1;
        }
        if colon == 0 || colon == field.len() {
            return Err(400u16);
        }
        let name = &field[..colon];
        let value = trim(&field[colon + 1..]);
        for byte in value {
            if (*byte < 32u8 && *byte != b'\t') || *byte == 127u8 {
                return Err(400u16);
            }
        }
        if same(name, b"host") {
            if host || !authority(value) {
                return Err(400u16);
            }
            host = true;
        } else if same(name, b"content-length") {
            if length.is_some() {
                return Err(400u16);
            }
            length = Some(number(value)?);
        } else if same(name, b"transfer-encoding") {
            transfer = true;
        } else if same(name, b"expect") {
            expectation = true;
        }
        at = end + 2;
    }
    if version == b"HTTP/1.1" && !host {
        return Err(400u16);
    }
    if transfer && length.is_some() {
        return Err(400u16);
    }
    // Reject before reading a body that the sender may withhold until 100 Continue.
    if expectation {
        return Err(417u16);
    }
    if !supported {
        return Err(405u16);
    }
    if transfer {
        return Err(501u16);
    }
    let path = target_path(&line[first + 1..second])?;
    Ok(Request {
        path,
        head,
        body: length.unwrap_or(0usize),
    })
}
pub fn header_end(bytes: &[u8], start: usize) -> Option<usize> {
    if start > bytes.len() {
        return None;
    }
    let mut index = start;
    while bytes.len() - index >= 4 {
        if &bytes[index..index + 4] == b"\r\n\r\n" {
            return Some(index + 4);
        }
        index += 1;
    }
    None
}

#[test]
fn ordinary_requests() {
    let cases:Vec<(String,String,bool,usize)>=vec![
        ("GET / HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"language.md".to_string(),false,0usize),
        ("HEAD /ir.md?view=1 HTTP/1.1\r\nhOsT:\t127.0.0.1:8000 \r\n\r\n".to_string(),"ir.md".to_string(),true,0usize),
        ("GET /%6canguage.md HTTP/1.0\r\n\r\n".to_string(),"language.md".to_string(),false,0usize),
        ("GET http://localhost:80/direction.md?x=1 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"direction.md".to_string(),false,0usize),
        ("GET http://localhost HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"language.md".to_string(),false,0usize),
        ("GET http://localhost?view=1 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"language.md".to_string(),false,0usize),
        ("GET /file%23name%20with%20spaces HTTP/1.1\r\nHost: localhost\r\nContent-Length: 12345\r\n\r\n".to_string(),"file#name with spaces".to_string(),false,12345usize),
        ("GET /%2e%2e/AGENTS.md HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"../AGENTS.md".to_string(),false,0usize),
        ("GET /%CF%80 HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),"π".to_string(),false,0usize),
    ];
    for (input, path, head, body) in cases {
        let request = parse(input.as_bytes()).unwrap();
        assert!(request.path == path && request.head == head && request.body == body);
    }
}
#[test]
fn malformed_requests() {
    let cases: Vec<(String, u16)> = vec![
        ("GET / HTTP/1.1\r\nHost: localhost\r\nExpect: 100-continue\r\nContent-Length: 1\r\n\r\n".to_string(), 417u16),
        (
            "GET http://localhost?view=1#fragment HTTP/1.1\r\nHost: localhost\r\n\r\n".to_string(),
            400u16,
        ),
        ("GET / HTTP/1.1\r\n\r\n".to_string(), 400u16),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nHost: b\r\n\r\n".to_string(),
            400u16,
        ),
        ("GET / HTTP/1.1\r\nHost:\r\n\r\n".to_string(), 400u16),
        (
            "GET / HTTP/1.1\r\nHost: user@host\r\n\r\n".to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: host:bad\r\n\r\n".to_string(),
            400u16,
        ),
        ("GET  / HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        ("GET / bad HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        ("GET / HTTP/2.0\r\nHost: a\r\n\r\n".to_string(), 505u16),
        ("GET / HTTP/1.1\nHost: a\n\n".to_string(), 400u16),
        (
            "GET / HTTP/1.1\r\nHost: a\r\n continuation\r\n\r\n".to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nBad Header: value\r\n\r\n".to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nValue: \0\r\n\r\n".to_string(),
            400u16,
        ),
        ("POST / HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 405u16),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\n\r\n".to_string(),
            501u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nTransfer-Encoding: chunked\r\nContent-Length: 0\r\n\r\n"
                .to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n"
                .to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nContent-Length: -1\r\n\r\n".to_string(),
            400u16,
        ),
        (
            "GET / HTTP/1.1\r\nHost: a\r\nContent-Length: 18446744073709551616\r\n\r\n".to_string(),
            400u16,
        ),
        ("GET /% HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        ("GET /%GG HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        ("GET /%00 HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        ("GET /%FF HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
        (
            "GET /path#fragment HTTP/1.1\r\nHost: a\r\n\r\n".to_string(),
            400u16,
        ),
        ("GET ?query HTTP/1.1\r\nHost: a\r\n\r\n".to_string(), 400u16),
    ];
    for (input, status) in cases {
        assert!(matches!(parse(input.as_bytes()),Err(found) if found==status));
    }
}
#[test]
fn header_chunks_and_growth() {
    let mut request = "GET /language.md HTTP/1.1\r\nHost: localhost\r\nX-Large: ".to_string();
    for _index in 0usize..257usize {
        request.push('x');
    }
    request.push_str("\r\n\r\n");
    let bytes = request.as_bytes();
    assert_eq!(parse(bytes).unwrap().path, "language.md");
    for chunk in vec![1usize, 2usize, 3usize, 7usize, 127usize, 256usize] {
        let mut accumulated: Vec<u8> = Vec::new();
        let mut position = 0usize;
        while position < bytes.len() {
            let next = if bytes.len() - position > chunk {
                position + chunk
            } else {
                bytes.len()
            };
            let start = if accumulated.len() > 3 {
                accumulated.len() - 3
            } else {
                0
            };
            accumulated.extend_from_slice(&bytes[position..next]);
            position = next;
            if position < bytes.len() {
                assert!(header_end(&accumulated, start).is_none());
            } else {
                assert_eq!(header_end(&accumulated, start), Some(bytes.len()));
            }
        }
    }
}
