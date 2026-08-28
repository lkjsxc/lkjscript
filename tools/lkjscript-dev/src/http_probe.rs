use crate::error::DevError;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

const MAXIMUM_HTTP_HEADER_BYTES: usize = 64 * 1024;
const MAXIMUM_HTTP_BODY_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) body: Vec<u8>,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) elapsed_nanoseconds: u64,
}

pub(crate) fn request(
    address: SocketAddr,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<HttpResponse, DevError> {
    require(
        address.ip() == IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        "HTTP acceptance may connect only to IPv4 loopback",
    )?;
    require(
        method.bytes().all(|byte| byte.is_ascii_uppercase()),
        "HTTP method is invalid",
    )?;
    require(
        path.starts_with('/') && !path.contains(['\r', '\n']),
        "HTTP path is invalid",
    )?;
    require(
        body.len() <= MAXIMUM_HTTP_BODY_BYTES,
        "HTTP request body exceeds the acceptance bound",
    )?;
    for (name, value) in headers {
        require(
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                && !value.contains(['\r', '\n']),
            "HTTP header is invalid",
        )?;
    }

    let started = Instant::now();
    let mut stream =
        TcpStream::connect_timeout(&address, Duration::from_secs(5)).map_err(|error| {
            DevError::infrastructure(format!(
                "connect to loopback HTTP listener {address}: {error}"
            ))
        })?;
    stream
        .set_read_timeout(Some(Duration::from_secs(35)))
        .and_then(|()| stream.set_write_timeout(Some(Duration::from_secs(35))))
        .map_err(|error| DevError::infrastructure(format!("set HTTP timeouts: {error}")))?;
    let mut encoded = Vec::with_capacity(
        method
            .len()
            .saturating_add(path.len())
            .saturating_add(body.len())
            .saturating_add(256),
    );
    write!(
        encoded,
        "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    )
    .map_err(|error| DevError::infrastructure(format!("encode HTTP request: {error}")))?;
    for (name, value) in headers {
        write!(encoded, "{name}: {value}\r\n")
            .map_err(|error| DevError::infrastructure(format!("encode HTTP header: {error}")))?;
    }
    encoded.extend_from_slice(b"\r\n");
    encoded.extend_from_slice(body);
    stream
        .write_all(&encoded)
        .and_then(|()| stream.flush())
        .map_err(|error| DevError::infrastructure(format!("write HTTP request: {error}")))?;

    let maximum_response = MAXIMUM_HTTP_HEADER_BYTES
        .checked_add(MAXIMUM_HTTP_BODY_BYTES)
        .and_then(|value| value.checked_add(1))
        .ok_or_else(|| DevError::infrastructure("HTTP response bound overflow"))?;
    let mut response = Vec::with_capacity(16 * 1024);
    let mut buffer = [0_u8; 16 * 1024];
    let mut header_end = None;
    let mut expected_total = None;
    loop {
        let read = stream
            .read(&mut buffer)
            .map_err(|error| DevError::infrastructure(format!("read HTTP response: {error}")))?;
        if read == 0 {
            break;
        }
        if response.len().saturating_add(read) > maximum_response {
            return Err(DevError::corrupt(
                "HTTP response exceeded the acceptance bound",
            ));
        }
        response.extend_from_slice(&buffer[..read]);
        if header_end.is_none() {
            header_end = find_bytes(&response, b"\r\n\r\n").map(|index| index + 4);
            if header_end.is_none() && response.len() > MAXIMUM_HTTP_HEADER_BYTES {
                return Err(DevError::corrupt(
                    "HTTP response headers exceeded the acceptance bound",
                ));
            }
            if let Some(end) = header_end {
                let (_, parsed_headers) = parse_response_head(&response[..end])?;
                if let Some(length) = parsed_headers
                    .get("content-length")
                    .map(|value| value.parse::<usize>())
                    .transpose()
                    .map_err(|_| DevError::corrupt("HTTP content length is invalid"))?
                {
                    require(
                        length <= MAXIMUM_HTTP_BODY_BYTES,
                        "HTTP response body exceeded the acceptance bound",
                    )?;
                    expected_total = end.checked_add(length);
                }
            }
        }
        if expected_total.is_some_and(|expected| response.len() >= expected) {
            break;
        }
    }
    let end =
        header_end.ok_or_else(|| DevError::corrupt("HTTP response omitted complete headers"))?;
    let (status, response_headers) = parse_response_head(&response[..end])?;
    let encoded_body = &response[end..];
    let body = if response_headers
        .get("transfer-encoding")
        .is_some_and(|value| value.eq_ignore_ascii_case("chunked"))
    {
        decode_chunked(encoded_body)?
    } else if let Some(length) = response_headers.get("content-length") {
        let length = length
            .parse::<usize>()
            .map_err(|_| DevError::corrupt("HTTP content length is invalid"))?;
        require(
            encoded_body.len() >= length,
            "HTTP response body was truncated",
        )?;
        encoded_body[..length].to_vec()
    } else {
        encoded_body.to_vec()
    };
    require(
        body.len() <= MAXIMUM_HTTP_BODY_BYTES,
        "HTTP response body exceeded the acceptance bound",
    )?;
    Ok(HttpResponse {
        status,
        body,
        headers: response_headers,
        elapsed_nanoseconds: u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
    })
}

fn parse_response_head(bytes: &[u8]) -> Result<(u16, BTreeMap<String, String>), DevError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| DevError::corrupt("HTTP response headers were not UTF-8"))?;
    let mut lines = text.split("\r\n");
    let status_line = lines
        .next()
        .ok_or_else(|| DevError::corrupt("HTTP response omitted a status line"))?;
    let mut status_parts = status_line.split_whitespace();
    let version = status_parts.next().unwrap_or_default();
    let status = status_parts
        .next()
        .ok_or_else(|| DevError::corrupt("HTTP status is absent"))?
        .parse::<u16>()
        .map_err(|_| DevError::corrupt("HTTP status is invalid"))?;
    require(
        version == "HTTP/1.1" && status_parts.next().is_some(),
        "HTTP status line is invalid",
    )?;
    let mut headers = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| DevError::corrupt("HTTP response header is malformed"))?;
        let name = name.to_ascii_lowercase();
        let value = value.trim().to_owned();
        if headers.insert(name, value).is_some() {
            return Err(DevError::corrupt(
                "HTTP response repeats a header in the acceptance client",
            ));
        }
    }
    Ok((status, headers))
}

fn decode_chunked(bytes: &[u8]) -> Result<Vec<u8>, DevError> {
    let mut cursor = 0_usize;
    let mut output = Vec::new();
    loop {
        let line_end = find_bytes(&bytes[cursor..], b"\r\n")
            .map(|relative| cursor + relative)
            .ok_or_else(|| DevError::corrupt("chunked HTTP response omitted a chunk header"))?;
        let size_text = std::str::from_utf8(&bytes[cursor..line_end])
            .map_err(|_| DevError::corrupt("chunk size was not UTF-8"))?;
        let size = usize::from_str_radix(size_text.split(';').next().unwrap_or_default(), 16)
            .map_err(|_| DevError::corrupt("chunk size was invalid"))?;
        cursor = line_end.saturating_add(2);
        if size == 0 {
            return Ok(output);
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| DevError::corrupt("chunk size overflow"))?;
        if end.saturating_add(2) > bytes.len() || &bytes[end..end + 2] != b"\r\n" {
            return Err(DevError::corrupt("chunked HTTP response was truncated"));
        }
        if output.len().saturating_add(size) > MAXIMUM_HTTP_BODY_BYTES {
            return Err(DevError::corrupt(
                "chunked HTTP response exceeded the acceptance bound",
            ));
        }
        output.extend_from_slice(&bytes[cursor..end]);
        cursor = end + 2;
    }
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    (!needle.is_empty() && needle.len() <= haystack.len())
        .then(|| {
            haystack
                .windows(needle.len())
                .position(|window| window == needle)
        })
        .flatten()
}

fn require(condition: bool, message: &'static str) -> Result<(), DevError> {
    if condition {
        Ok(())
    } else {
        Err(DevError::corrupt(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_parser_is_strict_and_bounded() {
        let (status, headers) = parse_response_head(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nContent-Type: text/plain\r\n\r\n",
        )
        .expect("response head");
        assert_eq!(status, 200);
        assert_eq!(headers.get("content-length").map(String::as_str), Some("2"));
        assert_eq!(
            decode_chunked(b"2\r\nok\r\n0\r\n\r\n").expect("chunked"),
            b"ok"
        );
        assert!(parse_response_head(b"HTTP/1.1 bad\r\n\r\n").is_err());
    }
}
