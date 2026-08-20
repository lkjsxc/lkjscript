//! Strict, bounded JSON wire encoding for the topology-neutral logical protocol.

pub use crate::contract::{
    MACHINE_SCHEMA_IDENTITY, active_machine_schema_digest, describe_schema, machine_schema_digest,
    schema_description,
};
pub use crate::machine_contract::*;

use crate::ids::RequestId;
use crate::protocol::{Request, Response};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Write};

pub const JSON_ENVELOPE_VERSION: u16 = 13;
pub const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const TRANSACTION_FINGERPRINT_DOMAIN: &str = "lkjscript.apply-transaction.fingerprint.v14";
pub(crate) const MAX_BOUNDARY_ERROR_MESSAGE_BYTES: usize = 1024;
const BOUNDARY_ERROR_FALLBACK: &[u8] =
    b"{\"version\":13,\"error\":{\"kind\":\"output\",\"message\":\"cannot encode boundary error\"}}";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub version: u16,
    pub request_id: RequestId,
    pub request: Request,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub version: u16,
    pub request_id: RequestId,
    pub response: Response,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryErrorEnvelope {
    pub version: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
    pub error: BoundaryError,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryError {
    pub kind: BoundaryErrorKind,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryErrorKind {
    InvalidJson,
    InputTooLarge,
    Transport,
    Output,
    Usage,
}
impl BoundaryErrorKind {
    pub(crate) const fn machine_name(self) -> &'static str {
        match self {
            Self::InvalidJson => "invalid_json",
            Self::InputTooLarge => "input_too_large",
            Self::Transport => "transport",
            Self::Output => "output",
            Self::Usage => "usage",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MachineError {
    pub kind: BoundaryErrorKind,
    pub message: String,
}

impl MachineError {
    pub fn new(kind: BoundaryErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for MachineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for MachineError {}

pub fn encode_request(request_id: RequestId, request: &Request) -> Result<Vec<u8>, MachineError> {
    require_nonzero_request_id(request_id)?;
    let envelope = RequestEnvelope {
        version: JSON_ENVELOPE_VERSION,
        request_id,
        request: request.clone(),
    };
    encode_with_limit(&envelope, false, MAX_JSON_INPUT_BYTES)
}

pub fn decode_request(bytes: &[u8]) -> Result<RequestEnvelope, MachineError> {
    if bytes.len() > MAX_JSON_INPUT_BYTES {
        return Err(MachineError::new(
            BoundaryErrorKind::InputTooLarge,
            "JSON request exceeds input byte policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope = RequestEnvelope::deserialize(&mut deserializer)
        .map_err(|error| MachineError::new(BoundaryErrorKind::InvalidJson, error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| MachineError::new(BoundaryErrorKind::InvalidJson, error.to_string()))?;
    if envelope.version != JSON_ENVELOPE_VERSION {
        return Err(MachineError::new(
            BoundaryErrorKind::InvalidJson,
            "JSON envelope version is unsupported",
        ));
    }
    if let Request::DescribeSchema(request) = &envelope.request {
        request
            .validate()
            .map_err(|message| MachineError::new(BoundaryErrorKind::InvalidJson, message))?;
    }
    if let Request::Run { arguments, .. } = &envelope.request {
        if arguments.len() > crate::interpret::MAX_RUN_ARGUMENTS {
            return Err(MachineError::new(
                BoundaryErrorKind::InputTooLarge,
                "run argument count exceeds the JSON boundary policy",
            ));
        }
        let mut total_items = 0_usize;
        let mut total_bytes = 0_usize;
        for argument in arguments {
            let (items, bytes) =
                crate::interpret::runtime_value_policy_metrics(argument).map_err(|error| {
                    MachineError::new(BoundaryErrorKind::InputTooLarge, error.to_string())
                })?;
            total_items = total_items.checked_add(items).ok_or_else(|| {
                MachineError::new(
                    BoundaryErrorKind::InputTooLarge,
                    "run argument item accounting overflowed",
                )
            })?;
            total_bytes = total_bytes.checked_add(bytes).ok_or_else(|| {
                MachineError::new(
                    BoundaryErrorKind::InputTooLarge,
                    "run argument byte accounting overflowed",
                )
            })?;
        }
        if total_items > crate::interpret::MAX_RUNTIME_VALUE_ITEMS
            || total_bytes > crate::interpret::MAX_RUNTIME_VALUE_BYTES
        {
            return Err(MachineError::new(
                BoundaryErrorKind::InputTooLarge,
                "run arguments exceed the aggregate runtime value JSON policy",
            ));
        }
    }
    Ok(envelope)
}

pub fn decode_response(bytes: &[u8]) -> Result<ResponseEnvelope, MachineError> {
    let envelope: ResponseEnvelope = decode_response_json(bytes)?;
    require_response_version(envelope.version)?;
    Ok(envelope)
}

fn decode_response_json<T>(bytes: &[u8]) -> Result<T, MachineError>
where
    T: for<'de> Deserialize<'de>,
{
    if bytes.len() > MAX_JSON_OUTPUT_BYTES {
        return Err(MachineError::new(
            BoundaryErrorKind::InputTooLarge,
            "JSON response exceeds output byte policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope = T::deserialize(&mut deserializer)
        .map_err(|error| MachineError::new(BoundaryErrorKind::InvalidJson, error.to_string()))?;
    deserializer
        .end()
        .map_err(|error| MachineError::new(BoundaryErrorKind::InvalidJson, error.to_string()))?;
    Ok(envelope)
}

fn require_response_version(version: u16) -> Result<(), MachineError> {
    if version != JSON_ENVELOPE_VERSION {
        return Err(MachineError::new(
            BoundaryErrorKind::InvalidJson,
            "JSON envelope version is unsupported",
        ));
    }
    Ok(())
}

pub fn request_id_hint(bytes: &[u8]) -> Option<RequestId> {
    if bytes.len() > MAX_JSON_INPUT_BYTES {
        return None;
    }
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct CorrelationEnvelope {
        #[serde(rename = "version")]
        _version: u16,
        request_id: RequestId,
        #[serde(rename = "request")]
        _request: serde::de::IgnoredAny,
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let envelope = CorrelationEnvelope::deserialize(&mut deserializer).ok()?;
    deserializer.end().ok()?;
    Some(envelope.request_id)
}

pub fn encode_response(
    request_id: RequestId,
    response: &Response,
    pretty: bool,
) -> Result<Vec<u8>, MachineError> {
    require_nonzero_request_id(request_id)?;
    #[derive(Serialize)]
    struct BorrowedResponseEnvelope<'a> {
        version: u16,
        request_id: RequestId,
        response: &'a Response,
    }
    encode_bounded(
        &BorrowedResponseEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id,
            response,
        },
        pretty,
    )
}

fn require_nonzero_request_id(request_id: RequestId) -> Result<(), MachineError> {
    if request_id.get() == 0 {
        return Err(MachineError::new(
            BoundaryErrorKind::InvalidJson,
            "request ID zero is reserved",
        ));
    }
    Ok(())
}

pub fn encode_schema(
    request: &DescribeSchemaRequest,
    pretty: bool,
) -> Result<Vec<u8>, MachineError> {
    let result = describe_schema(request)
        .map_err(|message| MachineError::new(BoundaryErrorKind::Usage, message))?;
    encode_bounded(&result, pretty)
}

pub fn encode_boundary_error(
    request_id: Option<RequestId>,
    kind: BoundaryErrorKind,
    message: impl Into<String>,
) -> Vec<u8> {
    let message = bounded_message(&message.into(), MAX_BOUNDARY_ERROR_MESSAGE_BYTES);
    let envelope = BoundaryErrorEnvelope {
        version: JSON_ENVELOPE_VERSION,
        request_id,
        error: BoundaryError { kind, message },
    };
    encode_with_limit(&envelope, false, MAX_JSON_OUTPUT_BYTES)
        .unwrap_or_else(|_| BOUNDARY_ERROR_FALLBACK.to_vec())
}

fn bounded_message(message: &str, maximum_bytes: usize) -> String {
    if message.len() <= maximum_bytes {
        return message.to_owned();
    }
    let mut end = maximum_bytes;
    while !message.is_char_boundary(end) {
        end -= 1;
    }
    message[..end].to_owned()
}

pub(crate) fn transaction_fingerprint(
    request: &crate::transaction::ApplyTransactionRequest,
) -> crate::Result<[u8; 32]> {
    let bytes = serde_json::to_vec(request).map_err(|error| {
        crate::LkError::new(
            crate::ErrorCode::ProtocolMalformed,
            format!("cannot encode transaction fingerprint input: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(TRANSACTION_FINGERPRINT_DOMAIN);
    hasher.update(&bytes);
    Ok(*hasher.finalize().as_bytes())
}

fn encode_bounded<T: Serialize>(value: &T, pretty: bool) -> Result<Vec<u8>, MachineError> {
    encode_with_limit(value, pretty, MAX_JSON_OUTPUT_BYTES)
}

fn encode_with_limit<T: Serialize>(
    value: &T,
    pretty: bool,
    maximum_bytes: usize,
) -> Result<Vec<u8>, MachineError> {
    let mut sink = LimitWriter::new(maximum_bytes);
    let encoded = if pretty {
        let mut serializer = serde_json::Serializer::pretty(&mut sink);
        value.serialize(&mut serializer)
    } else {
        let mut serializer = serde_json::Serializer::new(&mut sink);
        value.serialize(&mut serializer)
    };
    if sink.exceeded {
        return Err(MachineError::new(
            BoundaryErrorKind::Output,
            "JSON response exceeds output byte policy",
        ));
    }
    encoded.map_err(|error| MachineError::new(BoundaryErrorKind::Output, error.to_string()))?;
    Ok(sink.bytes)
}

struct LimitWriter {
    bytes: Vec<u8>,
    maximum_bytes: usize,
    exceeded: bool,
}

impl LimitWriter {
    fn new(maximum_bytes: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(maximum_bytes.min(4096)),
            maximum_bytes,
            exceeded: false,
        }
    }
}

impl Write for LimitWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum_bytes.saturating_sub(self.bytes.len()) {
            self.exceeded = true;
            return Err(io::Error::other("JSON output byte policy exceeded"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
