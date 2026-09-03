//! Implementation-disjoint bounded RFC 6455 client used by service acceptance.
//!
//! This module intentionally imports no production HTTP/WebSocket helper. It constructs the
//! upgrade, SHA-1 accept value, client masking, frame lengths, fragmentation, and close payloads
//! directly over `TcpStream`.

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

const ACCEPT_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAXIMUM_HANDSHAKE_BYTES: usize = 64 * 1024;
const MAXIMUM_FRAME_BYTES: usize = 16 * 1024 * 1024;
const MAXIMUM_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawWebSocketError {
    pub code: &'static str,
    pub message: String,
}

impl RawWebSocketError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RawHandshake {
    pub status: u16,
    pub response_headers: BTreeMap<String, String>,
    pub accept_matches: bool,
    pub extensions_absent: bool,
    pub subprotocol_absent: bool,
    pub response_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RawMessage {
    Text(String),
    Binary(Vec<u8>),
    Pong(Vec<u8>),
    Close { code: Option<u16>, reason: String },
}

pub(crate) struct RawWebSocket {
    stream: TcpStream,
    timeout: Duration,
    transcript: Sha256,
    transcript_bytes: u64,
    fragmented_opcode: Option<u8>,
    fragmented: Vec<u8>,
}

impl RawWebSocket {
    pub(crate) fn connect(
        address: SocketAddr,
        path: &str,
        headers: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<(Self, RawHandshake), RawWebSocketError> {
        let (socket, handshake) = Self::request_upgrade(address, path, headers, timeout)?;
        match socket {
            Some(socket) => Ok((socket, handshake)),
            None => Err(RawWebSocketError::new(
                "raw_websocket_upgrade",
                format!("raw WebSocket upgrade returned HTTP {}", handshake.status),
            )),
        }
    }

    pub(crate) fn request_upgrade(
        address: SocketAddr,
        path: &str,
        headers: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<(Option<Self>, RawHandshake), RawWebSocketError> {
        if path.is_empty()
            || !path.starts_with('/')
            || path.contains('\0')
            || path.contains('\r')
            || path.contains('\n')
        {
            return Err(RawWebSocketError::new(
                "raw_websocket_path",
                "raw WebSocket path is not a bounded origin-form target",
            ));
        }
        let mut nonce = [0_u8; 16];
        random_bytes(&mut nonce)?;
        let key = base64(&nonce);
        let mut request = Vec::new();
        push_text(&mut request, "GET ")?;
        push_text(&mut request, path)?;
        push_text(&mut request, " HTTP/1.1\r\nHost: ")?;
        push_text(&mut request, &address.to_string())?;
        push_text(
            &mut request,
            "\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: ",
        )?;
        push_text(&mut request, &key)?;
        push_text(&mut request, "\r\nSec-WebSocket-Version: 13\r\n")?;
        for (name, value) in headers {
            if name.is_empty()
                || name.len() > 256
                || value.len() > 8 * 1024
                || name
                    .bytes()
                    .any(|byte| !byte.is_ascii_graphic() || byte == b':')
                || value.bytes().any(|byte| matches!(byte, b'\r' | b'\n' | 0))
            {
                return Err(RawWebSocketError::new(
                    "raw_websocket_header",
                    "raw WebSocket request header is invalid or excessive",
                ));
            }
            push_text(&mut request, name)?;
            push_text(&mut request, ": ")?;
            push_text(&mut request, value)?;
            push_text(&mut request, "\r\n")?;
        }
        push_text(&mut request, "\r\n")?;
        if request.len() > MAXIMUM_HANDSHAKE_BYTES {
            return Err(RawWebSocketError::new(
                "raw_websocket_handshake_limit",
                "raw WebSocket request exceeds its bounded handshake limit",
            ));
        }

        let mut stream = TcpStream::connect_timeout(&address, timeout)
            .map_err(|error| RawWebSocketError::new("raw_websocket_connect", error.to_string()))?;
        stream
            .set_read_timeout(Some(timeout))
            .and_then(|_| stream.set_write_timeout(Some(timeout)))
            .map_err(|error| RawWebSocketError::new("raw_websocket_timeout", error.to_string()))?;
        stream
            .write_all(&request)
            .map_err(|error| RawWebSocketError::new("raw_websocket_write", error.to_string()))?;
        let response = read_http_head(&mut stream)?;
        let (status, response_headers) = parse_http_head(&response)?;
        let expected_accept = websocket_accept(&key);
        let accept_matches = response_headers
            .get("sec-websocket-accept")
            .is_some_and(|value| value == &expected_accept);
        let upgrade_matches = response_headers
            .get("upgrade")
            .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
        let connection_matches = response_headers.get("connection").is_some_and(|value| {
            value
                .split(',')
                .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
        });
        let handshake = RawHandshake {
            status,
            accept_matches,
            extensions_absent: !response_headers.contains_key("sec-websocket-extensions"),
            subprotocol_absent: !response_headers.contains_key("sec-websocket-protocol"),
            response_headers,
            response_bytes: u64::try_from(response.len()).unwrap_or(u64::MAX),
        };
        if status == 101 && (!accept_matches || !upgrade_matches || !connection_matches) {
            return Err(RawWebSocketError::new(
                "raw_websocket_upgrade",
                "raw WebSocket upgrade returned invalid switching-protocol headers",
            ));
        }
        let mut transcript = Sha256::new();
        transcript.update(&request);
        transcript.update(&response);
        let transcript_bytes = u64::try_from(request.len())
            .ok()
            .and_then(|length| length.checked_add(handshake.response_bytes))
            .ok_or_else(|| {
                RawWebSocketError::new(
                    "raw_websocket_transcript_limit",
                    "raw WebSocket transcript length overflowed",
                )
            })?;
        let socket = (status == 101).then_some(Self {
            stream,
            timeout,
            transcript,
            transcript_bytes,
            fragmented_opcode: None,
            fragmented: Vec::new(),
        });
        Ok((socket, handshake))
    }

    pub(crate) fn send_text(&mut self, value: &str) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x1, value.as_bytes(), true, 0)
    }

    pub(crate) fn send_binary(&mut self, value: &[u8]) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x2, value, true, 0)
    }

    pub(crate) fn send_fragmented_text(
        &mut self,
        first: &[u8],
        final_part: &[u8],
    ) -> Result<(), RawWebSocketError> {
        self.send_frame(false, 0x1, first, true, 0)?;
        self.send_frame(true, 0x0, final_part, true, 0)
    }

    pub(crate) fn send_ping(&mut self, value: &[u8]) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x9, value, true, 0)
    }

    pub(crate) fn send_close(&mut self, code: u16, reason: &str) -> Result<(), RawWebSocketError> {
        let mut payload = Vec::with_capacity(reason.len().saturating_add(2));
        payload.extend_from_slice(&code.to_be_bytes());
        payload.extend_from_slice(reason.as_bytes());
        self.send_frame(true, 0x8, &payload, true, 0)
    }

    pub(crate) fn send_unmasked_text(&mut self, value: &str) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x1, value.as_bytes(), false, 0)
    }

    pub(crate) fn send_invalid_text(&mut self) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x1, &[0xff, 0xfe], true, 0)
    }

    pub(crate) fn send_reserved_opcode(&mut self) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x3, b"reserved", true, 0)
    }

    pub(crate) fn send_reserved_bit(&mut self) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x1, b"reserved", true, 0x40)
    }

    pub(crate) fn send_orphan_continuation(&mut self) -> Result<(), RawWebSocketError> {
        self.send_frame(true, 0x0, b"orphan", true, 0)
    }

    pub(crate) fn send_fragmented_ping(&mut self) -> Result<(), RawWebSocketError> {
        self.send_frame(false, 0x9, b"fragmented", true, 0)
    }

    pub(crate) fn send_oversized(&mut self, bytes: usize) -> Result<(), RawWebSocketError> {
        if bytes > MAXIMUM_FRAME_BYTES {
            return Err(RawWebSocketError::new(
                "raw_websocket_frame_limit",
                "requested negative frame exceeds the raw oracle limit",
            ));
        }
        self.send_frame(true, 0x2, &vec![b'x'; bytes], true, 0)
    }

    pub(crate) fn is_quiet_for(&self, duration: Duration) -> Result<bool, RawWebSocketError> {
        self.stream
            .set_read_timeout(Some(duration))
            .map_err(|error| RawWebSocketError::new("raw_websocket_timeout", error.to_string()))?;
        let mut byte = [0_u8; 1];
        let result = match self.stream.peek(&mut byte) {
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                Ok(true)
            }
            Ok(0) => Err(RawWebSocketError::new(
                "raw_websocket_closed",
                "raw WebSocket peer closed while quiet was expected",
            )),
            Ok(_) => Ok(false),
            Err(error) => Err(RawWebSocketError::new(
                "raw_websocket_peek",
                error.to_string(),
            )),
        };
        let quiet = result?;
        self.stream
            .set_read_timeout(Some(self.timeout))
            .map_err(|error| RawWebSocketError::new("raw_websocket_timeout", error.to_string()))?;
        Ok(quiet)
    }

    pub(crate) fn read_message(&mut self) -> Result<RawMessage, RawWebSocketError> {
        loop {
            let frame = self.read_frame()?;
            match frame.opcode {
                0x0 => {
                    let opcode = self.fragmented_opcode.ok_or_else(|| {
                        RawWebSocketError::new(
                            "raw_websocket_continuation",
                            "server sent a continuation without a fragmented message",
                        )
                    })?;
                    append_bounded(&mut self.fragmented, &frame.payload)?;
                    if frame.fin {
                        self.fragmented_opcode = None;
                        let payload = std::mem::take(&mut self.fragmented);
                        return decode_message(opcode, payload);
                    }
                }
                opcode @ (0x1 | 0x2) => {
                    if self.fragmented_opcode.is_some() {
                        return Err(RawWebSocketError::new(
                            "raw_websocket_fragment",
                            "server started a second fragmented message",
                        ));
                    }
                    if frame.fin {
                        return decode_message(opcode, frame.payload);
                    }
                    self.fragmented_opcode = Some(opcode);
                    append_bounded(&mut self.fragmented, &frame.payload)?;
                }
                0x8 => return decode_close(frame.payload),
                0x9 => {
                    self.send_frame(true, 0xa, &frame.payload, true, 0)?;
                }
                0xa => return Ok(RawMessage::Pong(frame.payload)),
                _ => {
                    return Err(RawWebSocketError::new(
                        "raw_websocket_opcode",
                        "server emitted a reserved opcode",
                    ));
                }
            }
        }
    }

    pub(crate) fn transcript_digest(&self) -> String {
        hex(self.transcript.clone().finalize().as_slice())
    }

    pub(crate) const fn transcript_bytes(&self) -> u64 {
        self.transcript_bytes
    }

    pub(crate) fn disconnect(&self) -> Result<(), RawWebSocketError> {
        self.stream
            .shutdown(Shutdown::Both)
            .map_err(|error| RawWebSocketError::new("raw_websocket_shutdown", error.to_string()))
    }

    fn send_frame(
        &mut self,
        fin: bool,
        opcode: u8,
        payload: &[u8],
        masked: bool,
        reserved_bits: u8,
    ) -> Result<(), RawWebSocketError> {
        if payload.len() > MAXIMUM_FRAME_BYTES {
            return Err(RawWebSocketError::new(
                "raw_websocket_frame_limit",
                "raw WebSocket client frame exceeds its oracle limit",
            ));
        }
        let mut frame = Vec::with_capacity(payload.len().saturating_add(14));
        frame.push((if fin { 0x80 } else { 0 }) | reserved_bits | (opcode & 0x0f));
        let mask_bit = if masked { 0x80 } else { 0 };
        match payload.len() {
            length @ 0..=125 => frame.push(mask_bit | u8::try_from(length).unwrap_or(125)),
            length @ 126..=65_535 => {
                frame.push(mask_bit | 126);
                frame.extend_from_slice(&u16::try_from(length).unwrap_or(u16::MAX).to_be_bytes());
            }
            length => {
                frame.push(mask_bit | 127);
                frame.extend_from_slice(&u64::try_from(length).unwrap_or(u64::MAX).to_be_bytes());
            }
        }
        if masked {
            let mut mask = [0_u8; 4];
            random_bytes(&mut mask)?;
            frame.extend_from_slice(&mask);
            frame.extend(
                payload
                    .iter()
                    .enumerate()
                    .map(|(index, byte)| byte ^ mask[index % 4]),
            );
        } else {
            frame.extend_from_slice(payload);
        }
        self.stream
            .write_all(&frame)
            .map_err(|error| RawWebSocketError::new("raw_websocket_write", error.to_string()))?;
        self.observe(&frame)
    }

    fn read_frame(&mut self) -> Result<RawFrame, RawWebSocketError> {
        let mut header = [0_u8; 2];
        self.read_exact(&mut header)?;
        if header[0] & 0x70 != 0 {
            return Err(RawWebSocketError::new(
                "raw_websocket_reserved_bits",
                "server emitted reserved frame bits",
            ));
        }
        if header[1] & 0x80 != 0 {
            return Err(RawWebSocketError::new(
                "raw_websocket_server_mask",
                "server emitted a masked frame",
            ));
        }
        let fin = header[0] & 0x80 != 0;
        let opcode = header[0] & 0x0f;
        let short_length = header[1] & 0x7f;
        let length = match short_length {
            0..=125 => u64::from(short_length),
            126 => {
                let mut bytes = [0_u8; 2];
                self.read_exact(&mut bytes)?;
                let length = u64::from(u16::from_be_bytes(bytes));
                if length < 126 {
                    return Err(RawWebSocketError::new(
                        "raw_websocket_noncanonical_length",
                        "server emitted a noncanonical 16-bit frame length",
                    ));
                }
                length
            }
            127 => {
                let mut bytes = [0_u8; 8];
                self.read_exact(&mut bytes)?;
                let length = u64::from_be_bytes(bytes);
                if length < 65_536 || length & (1 << 63) != 0 {
                    return Err(RawWebSocketError::new(
                        "raw_websocket_noncanonical_length",
                        "server emitted a noncanonical 64-bit frame length",
                    ));
                }
                length
            }
            _ => {
                return Err(RawWebSocketError::new(
                    "raw_websocket_length",
                    "server frame length marker escaped its seven-bit field",
                ));
            }
        };
        let length = usize::try_from(length).map_err(|_| {
            RawWebSocketError::new(
                "raw_websocket_frame_limit",
                "server frame length does not fit the oracle address space",
            )
        })?;
        if length > MAXIMUM_FRAME_BYTES {
            return Err(RawWebSocketError::new(
                "raw_websocket_frame_limit",
                "server frame exceeds the bounded oracle limit",
            ));
        }
        if opcode >= 0x8 && (!fin || length > 125) {
            return Err(RawWebSocketError::new(
                "raw_websocket_control_frame",
                "server emitted a fragmented or oversized control frame",
            ));
        }
        let mut payload = vec![0_u8; length];
        self.read_exact(&mut payload)?;
        Ok(RawFrame {
            fin,
            opcode,
            payload,
        })
    }

    fn read_exact(&mut self, bytes: &mut [u8]) -> Result<(), RawWebSocketError> {
        self.stream
            .read_exact(bytes)
            .map_err(|error| RawWebSocketError::new("raw_websocket_read", error.to_string()))?;
        self.observe(bytes)
    }

    fn observe(&mut self, bytes: &[u8]) -> Result<(), RawWebSocketError> {
        self.transcript_bytes = self
            .transcript_bytes
            .checked_add(u64::try_from(bytes.len()).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                RawWebSocketError::new(
                    "raw_websocket_transcript_limit",
                    "raw WebSocket transcript length overflowed",
                )
            })?;
        self.transcript.update(bytes);
        Ok(())
    }
}

struct RawFrame {
    fin: bool,
    opcode: u8,
    payload: Vec<u8>,
}

fn decode_message(opcode: u8, payload: Vec<u8>) -> Result<RawMessage, RawWebSocketError> {
    match opcode {
        0x1 => String::from_utf8(payload)
            .map(RawMessage::Text)
            .map_err(|_| {
                RawWebSocketError::new(
                    "raw_websocket_text_utf8",
                    "server text message is not valid UTF-8",
                )
            }),
        0x2 => Ok(RawMessage::Binary(payload)),
        _ => Err(RawWebSocketError::new(
            "raw_websocket_opcode",
            "fragmented message retained a foreign opcode",
        )),
    }
}

fn decode_close(payload: Vec<u8>) -> Result<RawMessage, RawWebSocketError> {
    if payload.len() == 1 {
        return Err(RawWebSocketError::new(
            "raw_websocket_close_length",
            "server close payload has a one-byte code",
        ));
    }
    let (code, reason) = if payload.is_empty() {
        (None, String::new())
    } else {
        let code = u16::from_be_bytes([payload[0], payload[1]]);
        let reason = String::from_utf8(payload[2..].to_vec()).map_err(|_| {
            RawWebSocketError::new(
                "raw_websocket_close_utf8",
                "server close reason is not valid UTF-8",
            )
        })?;
        (Some(code), reason)
    };
    Ok(RawMessage::Close { code, reason })
}

fn append_bounded(output: &mut Vec<u8>, bytes: &[u8]) -> Result<(), RawWebSocketError> {
    if output.len().saturating_add(bytes.len()) > MAXIMUM_MESSAGE_BYTES {
        return Err(RawWebSocketError::new(
            "raw_websocket_message_limit",
            "fragmented server message exceeds the bounded oracle limit",
        ));
    }
    output.extend_from_slice(bytes);
    Ok(())
}

fn read_http_head(stream: &mut TcpStream) -> Result<Vec<u8>, RawWebSocketError> {
    let mut output = Vec::new();
    let mut byte = [0_u8; 1];
    while output.len() < MAXIMUM_HANDSHAKE_BYTES {
        stream.read_exact(&mut byte).map_err(|error| {
            RawWebSocketError::new("raw_websocket_handshake_read", error.to_string())
        })?;
        output.push(byte[0]);
        if output.ends_with(b"\r\n\r\n") {
            return Ok(output);
        }
    }
    Err(RawWebSocketError::new(
        "raw_websocket_handshake_limit",
        "raw WebSocket response head exceeds its bounded limit",
    ))
}

fn parse_http_head(bytes: &[u8]) -> Result<(u16, BTreeMap<String, String>), RawWebSocketError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        RawWebSocketError::new(
            "raw_websocket_handshake_utf8",
            "raw WebSocket response head is not UTF-8",
        )
    })?;
    let mut lines = text.strip_suffix("\r\n\r\n").unwrap_or(text).split("\r\n");
    let status_line = lines.next().ok_or_else(|| {
        RawWebSocketError::new(
            "raw_websocket_status",
            "raw WebSocket response omitted its status line",
        )
    })?;
    let mut status_parts = status_line.split_whitespace();
    if status_parts.next() != Some("HTTP/1.1") {
        return Err(RawWebSocketError::new(
            "raw_websocket_status",
            "raw WebSocket response is not HTTP/1.1",
        ));
    }
    let status = status_parts
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| {
            RawWebSocketError::new(
                "raw_websocket_status",
                "raw WebSocket response has an invalid status",
            )
        })?;
    let mut headers = BTreeMap::new();
    for line in lines {
        let (name, value) = line.split_once(':').ok_or_else(|| {
            RawWebSocketError::new(
                "raw_websocket_response_header",
                "raw WebSocket response contains a malformed header",
            )
        })?;
        let name = name.trim().to_ascii_lowercase();
        if name.is_empty() || headers.insert(name, value.trim().to_owned()).is_some() {
            return Err(RawWebSocketError::new(
                "raw_websocket_response_header",
                "raw WebSocket response repeats or omits a header name",
            ));
        }
    }
    Ok((status, headers))
}

fn websocket_accept(key: &str) -> String {
    let mut bytes = Vec::with_capacity(key.len().saturating_add(ACCEPT_GUID.len()));
    bytes.extend_from_slice(key.as_bytes());
    bytes.extend_from_slice(ACCEPT_GUID);
    base64(&sha1(&bytes))
}

fn sha1(bytes: &[u8]) -> [u8; 20] {
    let bit_length = u64::try_from(bytes.len())
        .unwrap_or(u64::MAX)
        .wrapping_mul(8);
    let mut message = bytes.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_length.to_be_bytes());
    let mut state = [
        0x6745_2301_u32,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];
    let (blocks, remainder) = message.as_chunks::<64>();
    debug_assert!(remainder.is_empty());
    for block in blocks {
        let mut words = [0_u32; 80];
        let (chunks, remainder) = block.as_chunks::<4>();
        debug_assert!(remainder.is_empty());
        for (index, chunk) in chunks.iter().enumerate() {
            words[index] = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, word) in words.into_iter().enumerate() {
            let (function, constant) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let next = a
                .rotate_left(5)
                .wrapping_add(function)
                .wrapping_add(e)
                .wrapping_add(constant)
                .wrapping_add(word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = next;
        }
        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
    }
    let mut output = [0_u8; 20];
    for (index, word) in state.into_iter().enumerate() {
        output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    output
}

fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        output.push(char::from(ALPHABET[usize::from(first >> 2)]));
        output.push(char::from(
            ALPHABET[usize::from(((first & 0x03) << 4) | (second >> 4))],
        ));
        output.push(if chunk.len() > 1 {
            char::from(ALPHABET[usize::from(((second & 0x0f) << 2) | (third >> 6))])
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            char::from(ALPHABET[usize::from(third & 0x3f)])
        } else {
            '='
        });
    }
    output
}

fn random_bytes(output: &mut [u8]) -> Result<(), RawWebSocketError> {
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(output))
        .map_err(|error| RawWebSocketError::new("raw_websocket_random", error.to_string()))
}

fn push_text(output: &mut Vec<u8>, value: &str) -> Result<(), RawWebSocketError> {
    if output.len().saturating_add(value.len()) > MAXIMUM_HANDSHAKE_BYTES {
        return Err(RawWebSocketError::new(
            "raw_websocket_handshake_limit",
            "raw WebSocket request exceeds its bounded handshake limit",
        ));
    }
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        output.push(char::from(ALPHABET[usize::from(byte >> 4)]));
        output.push(char::from(ALPHABET[usize::from(byte & 0x0f)]));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_sha1_and_accept_match_rfc6455_example() {
        assert_eq!(
            format!("{:02x?}", sha1(b"abc")),
            "[a9, 99, 3e, 36, 47, 06, 81, 6a, ba, 3e, 25, 71, 78, 50, c2, 6c, 9c, d0, d8, 9d]"
        );
        assert_eq!(
            websocket_accept("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn independent_base64_covers_padding_boundaries() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
    }
}
