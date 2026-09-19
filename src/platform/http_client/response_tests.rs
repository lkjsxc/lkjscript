//! Literal-wire oracles independent of the product's framing selection.

use super::*;
use std::io::{self, Read, Write};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::ReadBuf;

struct Wire {
    bytes: Vec<u8>,
    offset: usize,
    fragment: usize,
    forbid_eof_read: bool,
}

impl AsyncRead for Wire {
    fn poll_read(
        mut self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if buffer.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        if self.offset == self.bytes.len() {
            assert!(
                !self.forbid_eof_read,
                "a bodyless response must complete without another transport read"
            );
            return Poll::Ready(Ok(()));
        }
        let count = self
            .fragment
            .min(buffer.remaining())
            .min(self.bytes.len() - self.offset);
        buffer.put_slice(&self.bytes[self.offset..self.offset + count]);
        self.offset += count;
        Poll::Ready(Ok(()))
    }
}

impl AsyncWrite for Wire {
    fn poll_write(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
        _bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        panic!("the response reader must not write to its peer");
    }

    fn poll_flush(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

fn response(
    bytes: &[u8],
    fragment: usize,
    maximum_body: usize,
    forbid_eof_read: bool,
) -> Result<HttpClientResponse, ExecutionError> {
    let mut stream: Box<dyn HttpClientIo> = Box::new(Wire {
        bytes: bytes.to_vec(),
        offset: 0,
        fragment,
        forbid_eof_read,
    });
    let limits = HttpClientLimits {
        maximum_response_body_bytes: maximum_body,
        ..HttpClientLimits::default()
    };
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(read_response(&mut stream, &limits))
}

#[test]
fn not_modified_metadata_never_selects_a_body_or_machine_integer() {
    for metadata in [
        "",
        "Content-Length: 0\r\n",
        "Content-Length: 12345\r\n",
        "Content-Length: 9999999999999999999999999999999999999999\r\n",
        "Transfer-Encoding: chunked\r\n",
        "Transfer-Encoding: CHUNKED\r\n",
    ] {
        let wire = format!("HTTP/1.1 304 Not Modified\r\nETag: \"v1\"\r\n{metadata}\r\n");
        for fragment in [1, 7, 8192] {
            let result = response(wire.as_bytes(), fragment, 1, true).unwrap();
            assert_eq!(result.status, 304);
            assert!(result.body.is_empty());
            assert_eq!(result.headers[0].name, "etag");
            assert_eq!(result.headers[0].value, b"\"v1\"");
            if metadata.is_empty() {
                assert_eq!(result.headers.len(), 1);
            } else {
                assert_eq!(result.headers.len(), 2);
                let (name, value) = metadata.trim_end().split_once(": ").unwrap();
                assert_eq!(result.headers[1].name, name.to_ascii_lowercase());
                assert_eq!(result.headers[1].value, value.as_bytes());
            }
        }
    }
    for fragment in [1, 7, 8192] {
        let result = response(b"HTTP/1.1 204 No Content\r\n\r\n", fragment, 1, true).unwrap();
        assert_eq!(result.status, 204);
        assert!(result.body.is_empty());
    }
}

#[test]
fn bodyless_status_does_not_launder_invalid_framing() {
    for (status, fields) in [
        (204, "Content-Length: 0\r\n"),
        (204, "Content-Length: 1\r\n"),
        (204, "Transfer-Encoding: chunked\r\n"),
        (304, "Content-Length: +1\r\n"),
        (304, "Content-Length: 01\r\n"),
        (304, "Content-Length: 1, 1\r\n"),
        (304, "Content-Length: 1\r\nContent-Length: 1\r\n"),
        (
            304,
            "Transfer-Encoding: chunked\r\nTransfer-Encoding: chunked\r\n",
        ),
        (304, "Content-Length: 1\r\nTransfer-Encoding: chunked\r\n"),
        (304, "Transfer-Encoding: gzip\r\n"),
    ] {
        let wire = format!("HTTP/1.1 {status} Result\r\n{fields}\r\n");
        for fragment in [1, 7, 8192] {
            let error = response(wire.as_bytes(), fragment, 1, true).unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Capability);
            assert_eq!(error.code, "http_client_protocol");
        }
    }
    for status in [204, 304] {
        let wire = format!("HTTP/1.1 {status} Result\r\n\r\nx");
        assert_eq!(
            response(wire.as_bytes(), 8192, 1, true).unwrap_err().code,
            "http_client_protocol"
        );
    }
}

fn chunked(line_bytes: usize) -> Vec<u8> {
    let mut wire = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n1;".to_vec();
    wire.extend(std::iter::repeat_n(b'a', line_bytes - 4));
    wire.extend_from_slice(b"\r\nx\r\n0\r\n\r\n");
    wire
}

#[test]
fn chunk_line_limit_includes_crlf_and_is_fragmentation_independent() {
    // These literal protocol boundaries do not derive expectations from the reader's constant.
    for line_bytes in [8191, 8192, 8193, 16384] {
        for fragment in [1, 7, 8192, 16384] {
            let result = response(&chunked(line_bytes), fragment, 1, true);
            if line_bytes <= 8192 {
                assert_eq!(result.unwrap().body, b"x");
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.class, ExecutionFailureClass::Resource);
                assert_eq!(error.code, "http_client_response_header_limit");
            }
        }
    }
}

#[test]
fn ordinary_framing_limits_truncation_and_subsequent_success_are_preserved() {
    for fragment in [1, 7, 8192] {
        for wire in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5;x=y\r\nhello\r\n0\r\n\r\n",
            b"HTTP/1.1 200 OK\r\n\r\nhello",
        ] {
            let result = response(wire, fragment, 5, false).unwrap();
            assert_eq!(result.status, 200);
            assert_eq!(result.body, b"hello");
        }
        for wire in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 6\r\n\r\nabcdef".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nabcdef\r\n0\r\n\r\n",
            b"HTTP/1.1 200 OK\r\n\r\nabcdef",
            b"HTTP/1.1 200 OK\r\nContent-Length: 999999999999999999999999999999\r\n\r\n",
        ] {
            let error = response(wire, fragment, 5, false).unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "http_client_response_body_limit");
        }
        for wire in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhi".as_slice(),
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n5\r\nhi",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n0\r\nX-Trailer: no\r\n\r\n",
            b"HTTP/1.1 100 Continue\r\n\r\n",
        ] {
            assert_eq!(
                response(wire, fragment, 5, false).unwrap_err().code,
                "http_client_protocol"
            );
        }
        assert_eq!(
            response(
                b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx",
                fragment,
                1,
                true
            )
            .unwrap()
            .body,
            b"x"
        );
    }
}

#[test]
fn prepared_adapter_completes_bodyless_requests_and_recovers_without_replay() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let endpoint = format!("http://{}/resource", listener.local_addr().unwrap());
    // HttpClient uses Handle::block_on; keep an independent I/O driver running.
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap();
    let client = HttpClient::prepare(
        &endpoint,
        HttpClientAddressPolicy::LoopbackOnly,
        &HttpClientTrust::WebpkiRoots,
        None,
        HttpClientLimits {
            maximum_response_body_bytes: 1,
            ..HttpClientLimits::default()
        },
        runtime.handle().clone(),
    )
    .unwrap();
    std::thread::scope(|scope| {
        let peer = scope.spawn(move || {
            for wire in [
                b"HTTP/1.1 304 Not Modified\r\nContent-Length: 1000\r\n\r\n".as_slice(),
                b"HTTP/1.1 204 No Content\r\nContent-Length: 1\r\n\r\n",
                b"HTTP/1.1 200 OK\r\nContent-Length: 1\r\n\r\nx",
            ] {
                let deadline = Instant::now() + Duration::from_secs(15);
                let mut stream = loop {
                    match listener.accept() {
                        Ok((stream, _)) => break stream,
                        Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                            assert!(Instant::now() < deadline, "missing once-only request");
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        Err(error) => panic!("accept loopback request: {error}"),
                    }
                };
                stream
                    .set_read_timeout(Some(Duration::from_secs(15)))
                    .unwrap();
                stream
                    .set_write_timeout(Some(Duration::from_secs(15)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                    assert!(request.len() <= 8192);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("GET /resource HTTP/1.1\r\n"));
                assert!(request.contains("\r\nif-none-match: \"version-1\"\r\n"));
                stream.write_all(wire).unwrap();
                stream.flush().unwrap();
                // Keep the peer's write side open. Completion must not depend on EOF.
                assert_eq!(stream.read(&mut [0]).unwrap(), 0);
            }
        });
        let request = || {
            client.get(
                vec![HttpHeader {
                    name: "if-none-match".to_owned(),
                    value: b"\"version-1\"".to_vec(),
                }],
                &ExecutionControl::uncancelled(),
            )
        };
        let first = request().unwrap();
        assert_eq!(first.status, 304);
        assert!(first.body.is_empty());
        assert_eq!(request().unwrap_err().code, "http_client_protocol");
        let recovered = request().unwrap();
        assert_eq!(recovered.status, 200);
        assert_eq!(recovered.body, b"x");
        assert_eq!(client.active.load(Ordering::Acquire), 0);
        client.shutdown().unwrap();
        peer.join().unwrap();
    });
}
