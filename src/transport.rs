use crate::error::{ErrorCode, LkError, Result};
use crate::ids::RequestId;
use crate::machine;
use crate::protocol::{Request, Response};
use std::io::{ErrorKind, Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const MAXIMUM_REQUEST_FRAME_BYTES: usize = machine::MAX_JSON_INPUT_BYTES;
pub const MAXIMUM_RESPONSE_FRAME_BYTES: usize = machine::MAX_JSON_OUTPUT_BYTES;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);

pub struct Client {
    endpoint: PathBuf,
}

impl Client {
    pub fn new(endpoint: impl Into<PathBuf>) -> Self {
        Self {
            endpoint: endpoint.into(),
        }
    }

    pub fn request(&self, request_id: RequestId, request: &Request) -> Result<Response> {
        let body = machine::encode_request(request_id, request).map_err(machine_error)?;
        let stream = UnixStream::connect(&self.endpoint)?;
        let mut stream = DeadlineStream::new(stream, CLIENT_IO_TIMEOUT);
        write_request_body(&mut stream, &body)?;
        stream.shutdown_write()?;
        let response_body = read_response_body(&mut stream)?.ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "daemon closed the connection before a response frame",
            )
        })?;
        match machine::decode_daemon_response(&response_body, request_id).map_err(machine_error)? {
            machine::DaemonResponseEnvelope::Response(envelope) => Ok(envelope.response),
            machine::DaemonResponseEnvelope::BoundaryError(envelope) => {
                Err(boundary_error(envelope.error))
            }
        }
    }

    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }
}

pub(crate) fn read_request_body(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    read_single_frame(reader, MAXIMUM_REQUEST_FRAME_BYTES, "request")
}

pub(crate) fn read_response_body(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    read_single_frame(reader, MAXIMUM_RESPONSE_FRAME_BYTES, "response")
}

pub(crate) fn write_request_body(writer: &mut impl Write, body: &[u8]) -> Result<()> {
    write_frame(writer, body, MAXIMUM_REQUEST_FRAME_BYTES, "request")
}

pub(crate) fn write_response_body(writer: &mut impl Write, body: &[u8]) -> Result<()> {
    write_frame(writer, body, MAXIMUM_RESPONSE_FRAME_BYTES, "response")
}

fn read_single_frame(
    reader: &mut impl Read,
    maximum_bytes: usize,
    direction: &str,
) -> Result<Option<Vec<u8>>> {
    let mut length_bytes = [0_u8; 4];
    let first = match reader.read(&mut length_bytes[..1]) {
        Ok(0) => return Ok(None),
        Ok(count) => count,
        Err(error) => return Err(error.into()),
    };
    debug_assert_eq!(first, 1);
    read_exact_boundary(
        reader,
        &mut length_bytes[1..],
        &format!("{direction} frame length is truncated"),
    )?;
    let length = usize::try_from(u32::from_le_bytes(length_bytes)).map_err(|_| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{direction} frame length cannot be represented"),
        )
    })?;
    if length > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{direction} frame exceeds boundary byte policy"),
        ));
    }
    let mut body = vec![0_u8; length];
    read_exact_boundary(
        reader,
        &mut body,
        &format!("{direction} frame body is shorter than its declared length"),
    )?;
    require_connection_eof(reader, direction)?;
    Ok(Some(body))
}

fn write_frame(
    writer: &mut impl Write,
    body: &[u8],
    maximum_bytes: usize,
    direction: &str,
) -> Result<()> {
    if body.len() > maximum_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{direction} frame exceeds boundary byte policy"),
        ));
    }
    let length = u32::try_from(body.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{direction} frame length exceeds canonical u32 encoding"),
        )
    })?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(body)?;
    writer.flush()?;
    Ok(())
}

fn read_exact_boundary(reader: &mut impl Read, bytes: &mut [u8], message: &str) -> Result<()> {
    match reader.read_exact(bytes) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::UnexpectedEof => {
            Err(LkError::new(ErrorCode::ProtocolMalformed, message))
        }
        Err(error) => Err(error.into()),
    }
}

fn require_connection_eof(reader: &mut impl Read, direction: &str) -> Result<()> {
    let mut trailing = [0_u8; 1];
    match reader.read(&mut trailing) {
        Ok(0) => Ok(()),
        Ok(_) => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("connection contains bytes after its single {direction} frame"),
        )),
        Err(error) => Err(error.into()),
    }
}

fn machine_error(error: machine::MachineError) -> LkError {
    let code = match error.kind {
        machine::BoundaryErrorKind::InputTooLarge | machine::BoundaryErrorKind::Output => {
            ErrorCode::PolicyExceeded
        }
        machine::BoundaryErrorKind::Transport => ErrorCode::Io,
        machine::BoundaryErrorKind::InvalidJson | machine::BoundaryErrorKind::Usage => {
            ErrorCode::ProtocolMalformed
        }
    };
    LkError::new(code, error.to_string())
}

fn boundary_error(error: machine::BoundaryError) -> LkError {
    let code = match error.kind {
        machine::BoundaryErrorKind::InvalidJson | machine::BoundaryErrorKind::Usage => {
            ErrorCode::ProtocolMalformed
        }
        machine::BoundaryErrorKind::InputTooLarge | machine::BoundaryErrorKind::Output => {
            ErrorCode::PolicyExceeded
        }
        machine::BoundaryErrorKind::Transport => ErrorCode::Io,
    };
    LkError::new(
        code,
        format!(
            "daemon boundary error ({}): {}",
            error.kind.machine_name(),
            error.message
        ),
    )
}

struct DeadlineStream {
    stream: UnixStream,
    deadline: Instant,
}

impl DeadlineStream {
    fn new(stream: UnixStream, timeout: Duration) -> Self {
        Self {
            stream,
            deadline: Instant::now() + timeout,
        }
    }

    fn remaining(&self) -> std::io::Result<Duration> {
        match self.deadline.checked_duration_since(Instant::now()) {
            Some(remaining) if !remaining.is_zero() => Ok(remaining),
            _ => Err(std::io::Error::new(
                ErrorKind::TimedOut,
                "client connection deadline expired",
            )),
        }
    }

    fn shutdown_write(&self) -> std::io::Result<()> {
        self.stream.shutdown(Shutdown::Write)
    }
}

impl Read for DeadlineStream {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(bytes)
    }
}

impl Write for DeadlineStream {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(bytes)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine::JSON_ENVELOPE_VERSION;
    use crate::protocol::Response;
    use std::os::unix::net::UnixListener;
    use std::thread;

    fn fake_client_error(response_wire: Vec<u8>) -> LkError {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let path = temporary.path().join("fake.sock");
        let listener = UnixListener::bind(&path).expect("listener");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept");
            read_request_body(&mut stream)
                .expect("request frame")
                .expect("request body");
            stream.write_all(&response_wire).expect("fake response");
        });
        let error = Client::new(&path)
            .request(RequestId::new(7), &Request::Shutdown)
            .expect_err("fake response must reject");
        server.join().expect("fake server");
        error
    }

    fn response_frame(body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_response_body(&mut bytes, body).expect("response frame");
        bytes
    }

    fn framed(body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        write_request_body(&mut bytes, body).expect("frame");
        bytes
    }

    #[test]
    fn framing_rejects_partial_oversized_short_long_and_second_input() {
        assert_eq!(
            read_request_body(&mut [1_u8, 0].as_slice())
                .expect_err("partial length")
                .code,
            ErrorCode::ProtocolMalformed
        );
        let oversized = u32::try_from(MAXIMUM_REQUEST_FRAME_BYTES + 1)
            .expect("oversized length")
            .to_le_bytes();
        assert_eq!(
            read_request_body(&mut oversized.as_slice())
                .expect_err("oversized frame")
                .code,
            ErrorCode::PolicyExceeded
        );

        let mut short = 2_u32.to_le_bytes().to_vec();
        short.push(b'{');
        assert_eq!(
            read_request_body(&mut short.as_slice())
                .expect_err("short body")
                .code,
            ErrorCode::ProtocolMalformed
        );

        let mut long = framed(b"{}");
        long.push(0);
        assert_eq!(
            read_request_body(&mut long.as_slice())
                .expect_err("long body")
                .code,
            ErrorCode::ProtocolMalformed
        );

        let mut second = framed(b"{}");
        second.extend_from_slice(&framed(b"{}"));
        assert_eq!(
            read_request_body(&mut second.as_slice())
                .expect_err("second frame")
                .code,
            ErrorCode::ProtocolMalformed
        );
    }

    #[test]
    fn clean_close_and_request_half_close_are_proven() {
        assert_eq!(
            read_request_body(&mut std::io::empty()).expect("clean close"),
            None
        );
        let (mut server, mut client) = UnixStream::pair().expect("pair");
        let sender = thread::spawn(move || {
            write_request_body(&mut client, b"{}").expect("request");
            client.shutdown(Shutdown::Write).expect("half close");
        });
        assert_eq!(
            read_request_body(&mut server).expect("request body"),
            Some(b"{}".to_vec())
        );
        sender.join().expect("sender");
    }

    #[test]
    fn response_bound_is_independent_and_accepts_more_than_eight_mib() {
        let body = vec![b'x'; MAXIMUM_REQUEST_FRAME_BYTES + 1];
        assert_eq!(
            write_request_body(&mut Vec::new(), &body)
                .expect_err("request limit")
                .code,
            ErrorCode::PolicyExceeded
        );
        let mut framed = Vec::new();
        write_response_body(&mut framed, &body).expect("response frame");
        assert_eq!(
            read_response_body(&mut framed.as_slice()).expect("response body"),
            Some(body)
        );
    }

    #[test]
    fn client_rejects_mismatched_id_and_dropped_response() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let mismatch_path = temporary.path().join("mismatch.sock");
        let mismatch_listener = UnixListener::bind(&mismatch_path).expect("listener");
        let mismatch_server = thread::spawn(move || {
            let (mut stream, _) = mismatch_listener.accept().expect("accept");
            read_request_body(&mut stream)
                .expect("request frame")
                .expect("request body");
            let body = machine::encode_response(RequestId::new(8), &Response::Acknowledged, false)
                .expect("response");
            write_response_body(&mut stream, &body).expect("response frame");
        });
        assert_eq!(
            Client::new(&mismatch_path)
                .request(RequestId::new(7), &Request::Shutdown)
                .expect_err("mismatched response ID")
                .code,
            ErrorCode::ProtocolMalformed
        );
        mismatch_server.join().expect("mismatch server");

        let dropped_path = temporary.path().join("dropped.sock");
        let dropped_listener = UnixListener::bind(&dropped_path).expect("listener");
        let dropped_server = thread::spawn(move || {
            let (mut stream, _) = dropped_listener.accept().expect("accept");
            read_request_body(&mut stream)
                .expect("request frame")
                .expect("request body");
        });
        assert_eq!(
            Client::new(&dropped_path)
                .request(RequestId::new(9), &Request::Shutdown)
                .expect_err("dropped response")
                .code,
            ErrorCode::ProtocolMalformed
        );
        dropped_server.join().expect("dropped server");
    }

    #[test]
    fn client_rejects_malformed_oversized_truncated_and_multiple_responses() {
        let valid = machine::encode_response(RequestId::new(7), &Response::Acknowledged, false)
            .expect("valid response");
        let valid_frame = response_frame(&valid);
        let mut truncated_body = 2_u32.to_le_bytes().to_vec();
        truncated_body.push(b'{');
        let mut trailing = valid_frame.clone();
        trailing.push(0);
        let mut second_frame = valid_frame.clone();
        second_frame.extend_from_slice(&valid_frame);

        let cases = [
            (
                "malformed JSON",
                response_frame(b"{"),
                ErrorCode::ProtocolMalformed,
            ),
            (
                "oversized length",
                u32::try_from(MAXIMUM_RESPONSE_FRAME_BYTES + 1)
                    .expect("oversized response length")
                    .to_le_bytes()
                    .to_vec(),
                ErrorCode::PolicyExceeded,
            ),
            ("truncated header", vec![1, 0], ErrorCode::ProtocolMalformed),
            (
                "truncated body",
                truncated_body,
                ErrorCode::ProtocolMalformed,
            ),
            ("trailing byte", trailing, ErrorCode::ProtocolMalformed),
            ("second frame", second_frame, ErrorCode::ProtocolMalformed),
        ];
        for (name, response, expected) in cases {
            assert_eq!(fake_client_error(response).code, expected, "{name}");
        }
    }

    #[test]
    fn client_decodes_boundary_errors_and_validates_optional_correlation() {
        let matching = machine::encode_boundary_error(
            Some(RequestId::new(7)),
            machine::BoundaryErrorKind::InvalidJson,
            "bad request JSON",
        );
        let matching_error = fake_client_error(response_frame(&matching));
        assert_eq!(matching_error.code, ErrorCode::ProtocolMalformed);
        assert!(matching_error.message.contains("invalid_json"));
        assert!(matching_error.message.contains("bad request JSON"));

        for (kind, expected) in [
            (
                machine::BoundaryErrorKind::InputTooLarge,
                ErrorCode::PolicyExceeded,
            ),
            (
                machine::BoundaryErrorKind::Output,
                ErrorCode::PolicyExceeded,
            ),
            (
                machine::BoundaryErrorKind::Usage,
                ErrorCode::ProtocolMalformed,
            ),
        ] {
            let boundary = machine::encode_boundary_error(
                Some(RequestId::new(7)),
                kind,
                "typed boundary failure",
            );
            assert_eq!(fake_client_error(response_frame(&boundary)).code, expected);
        }

        let uncorrelated = machine::encode_boundary_error(
            None,
            machine::BoundaryErrorKind::Transport,
            "frame failed",
        );
        assert_eq!(
            fake_client_error(response_frame(&uncorrelated)).code,
            ErrorCode::Io
        );

        let mismatched = machine::encode_boundary_error(
            Some(RequestId::new(8)),
            machine::BoundaryErrorKind::InputTooLarge,
            "too large",
        );
        let mismatch_error = fake_client_error(response_frame(&mismatched));
        assert_eq!(mismatch_error.code, ErrorCode::ProtocolMalformed);
        assert!(mismatch_error.message.contains("does not match"));

        let matching_text = String::from_utf8(matching).expect("boundary error UTF-8");
        for malformed in [
            matching_text.replacen("\"version\":6", "\"version\":6,\"version\":6", 1),
            matching_text.replacen('{', "{\"unknown\":true,", 1),
            format!("{matching_text} null"),
        ] {
            assert_eq!(
                fake_client_error(response_frame(malformed.as_bytes())).code,
                ErrorCode::ProtocolMalformed
            );
        }
    }

    #[test]
    fn client_deadline_is_absolute_despite_slow_progress() {
        let (reader, mut writer) = UnixStream::pair().expect("Unix stream pair");
        let sender = thread::spawn(move || {
            for byte in 0..20_u8 {
                if writer.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        let started = Instant::now();
        let mut reader = DeadlineStream::new(reader, Duration::from_millis(35));
        let mut bytes = [0_u8; 20];
        let error = reader
            .read_exact(&mut bytes)
            .expect_err("absolute deadline must expire");
        assert!(matches!(
            error.kind(),
            ErrorKind::TimedOut | ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(reader);
        sender.join().expect("slow sender");
    }

    #[test]
    fn uncorrelated_boundary_error_has_no_synthetic_request_id() {
        let bytes = machine::encode_boundary_error(
            None,
            machine::BoundaryErrorKind::InvalidJson,
            "malformed framed JSON",
        );
        let envelope: machine::BoundaryErrorEnvelope =
            serde_json::from_slice(&bytes).expect("boundary error JSON");
        assert_eq!(envelope.request_id, None);
        assert_eq!(envelope.version, JSON_ENVELOPE_VERSION);
    }

    #[test]
    fn response_json_requires_nonzero_matching_id_and_eof() {
        let valid = machine::encode_response(RequestId::new(7), &Response::Acknowledged, false)
            .expect("response");
        let decoded = machine::decode_response(&valid).expect("decode");
        assert_eq!(decoded.version, JSON_ENVELOPE_VERSION);
        assert_eq!(decoded.request_id, RequestId::new(7));

        let zero = valid
            .windows(b"\"request_id\":7".len())
            .position(|window| window == b"\"request_id\":7")
            .map(|position| {
                let mut bytes = valid.clone();
                bytes[position..position + b"\"request_id\":7".len()]
                    .copy_from_slice(b"\"request_id\":0");
                bytes
            })
            .expect("request ID field");
        assert!(machine::decode_response(&zero).is_err());

        let duplicate = String::from_utf8(valid.clone())
            .expect("UTF-8 response")
            .replacen("\"version\":6", "\"version\":6,\"version\":6", 1);
        assert!(machine::decode_response(duplicate.as_bytes()).is_err());
        let unknown = String::from_utf8(valid.clone())
            .expect("UTF-8 response")
            .replacen('{', "{\"unknown\":true,", 1);
        assert!(machine::decode_response(unknown.as_bytes()).is_err());
        assert!(machine::decode_response(b"[").is_err());

        let mut trailing = valid;
        trailing.extend_from_slice(b" null");
        assert!(machine::decode_response(&trailing).is_err());
    }
}
