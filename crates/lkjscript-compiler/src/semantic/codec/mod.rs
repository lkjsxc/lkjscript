mod response;

use serde::Serialize;

use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, Request};

pub(crate) use response::{encode_prepared, prepare_response, PreparedResponse};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_JSON_DEPTH: u32 = 64;
pub(crate) const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;

pub(crate) fn decode_request(input: &[u8]) -> Result<Request, ProtocolError> {
    if input.len() > MAX_REQUEST_BYTES {
        return Err(error(
            ProtocolErrorCode::ResourceLimit,
            format!("request bytes {} exceed {MAX_REQUEST_BYTES}", input.len()),
        ));
    }
    std::str::from_utf8(input).map_err(|_| {
        error(
            ProtocolErrorCode::InvalidJson,
            "request is not well-formed UTF-8",
        )
    })?;
    check_json_depth(input)?;
    let request: Request = serde_json::from_slice(input).map_err(|failure| {
        error(
            ProtocolErrorCode::InvalidJson,
            format!("strict JSON request rejected: {failure}"),
        )
    })?;
    if request.schema != super::SCHEMA {
        return Err(error(
            ProtocolErrorCode::InvalidSchema,
            format!("unknown schema {:?}", request.schema),
        ));
    }
    if lkjscript_contracts::ContractDigest::from_hex(&request.contract) != Some(super::CONTRACT) {
        return Err(error(
            ProtocolErrorCode::ContractMismatch,
            format!(
                concat!(
                    "contract mismatch for {}: expected {}, actual {}; ",
                    "producer=semantic request, consumer=lkjscript compiler; update the producer"
                ),
                super::SCHEMA,
                super::CONTRACT,
                request.contract,
            ),
        ));
    }
    Ok(request)
}

fn check_json_depth(input: &[u8]) -> Result<(), ProtocolError> {
    let mut depth = 0_u32;
    let mut quoted = false;
    let mut escaped = false;
    for byte in input {
        if quoted {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match *byte {
            b'"' => quoted = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > MAX_JSON_DEPTH {
                    return Err(error(
                        ProtocolErrorCode::ResourceLimit,
                        format!("JSON nesting exceeds {MAX_JSON_DEPTH}"),
                    ));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

pub(crate) fn error(code: ProtocolErrorCode, message: impl Into<String>) -> ProtocolError {
    ProtocolError {
        code,
        message: message.into(),
        diagnostic: None,
    }
}

pub(crate) fn measure_json<T: Serialize>(value: &T) -> Result<usize, ProtocolError> {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self.0.checked_add(bytes.len()).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::FileTooLarge, "JSON size overflow")
            })?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value).map_err(|failure| {
        error(
            ProtocolErrorCode::ResourceLimit,
            format!("measure typed JSON: {failure}"),
        )
    })?;
    Ok(counter.0)
}
