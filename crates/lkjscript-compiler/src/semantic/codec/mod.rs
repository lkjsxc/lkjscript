mod response;

use serde::{Deserialize, Serialize};

use crate::semantic::schema::{ProtocolError, ProtocolErrorCode, Request};

pub(crate) use response::{encode_prepared, prepare_response, PreparedResponse};

pub const MAX_REQUEST_BYTES: usize = 1024 * 1024;
pub(crate) const MAX_OUTPUT_BYTES: usize = 8 * 1024 * 1024;
pub(super) const SERDE_STACK_RED_ZONE_BYTES: usize = 256 * 1024;

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
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    deserializer.disable_recursion_limit();
    let mut stacked = serde_stacker::Deserializer::new(&mut deserializer);
    stacked.red_zone = SERDE_STACK_RED_ZONE_BYTES;
    let request = Request::deserialize(stacked).map_err(json_error)?;
    deserializer.end().map_err(json_error)?;
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

fn json_error(failure: serde_json::Error) -> ProtocolError {
    error(
        ProtocolErrorCode::InvalidJson,
        format!("strict JSON request rejected: {failure}"),
    )
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
    write_json(&mut counter, value).map_err(|failure| {
        error(
            ProtocolErrorCode::ResourceLimit,
            format!("measure typed JSON: {failure}"),
        )
    })?;
    Ok(counter.0)
}

pub(crate) fn write_json<T: Serialize>(
    output: &mut impl std::io::Write,
    value: &T,
) -> Result<(), serde_json::Error> {
    let mut serializer = serde_json::Serializer::new(output);
    let mut stacked = serde_stacker::Serializer::new(&mut serializer);
    stacked.red_zone = SERDE_STACK_RED_ZONE_BYTES;
    value.serialize(stacked)
}
