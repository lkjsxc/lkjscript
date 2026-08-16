//! Strict, bounded JSON transport projection and runtime machine contract description.

pub use crate::machine_contract::*;

use crate::ids::RequestId;
use crate::protocol::{PROTOCOL_VERSION, Request, RequestCode, Response, ResponseCode};
use crate::query::{
    MAX_BATCH_ITEMS, MAX_BATCH_QUERIES, MAX_CONTEXT_ITEMS, MAX_PAGE_ITEMS, QueryCode,
};
use crate::schema::{NodeKind, OperationCode, SemanticType};
use crate::transaction::{MAX_RETURNED_BINDINGS, TransactionOpCode};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::io::{self, Write};

pub const JSON_ENVELOPE_VERSION: u16 = 7;
pub const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MACHINE_SCHEMA_IDENTITY: &str = "lkjscript-machine-schema-v7";
const MACHINE_SCHEMA_DIGEST_DOMAIN: &str = "lkjscript.machine-schema.digest.v2";
const TRANSACTION_FINGERPRINT_DOMAIN: &str = "lkjscript.apply-transaction.fingerprint.v7";
const MAX_BOUNDARY_ERROR_MESSAGE_BYTES: usize = 1024;
const BOUNDARY_ERROR_FALLBACK: &[u8] =
    b"{\"version\":7,\"error\":{\"kind\":\"output\",\"message\":\"cannot encode boundary error\"}}";

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

// The typed response remains direct; boxing only one transport arm would add allocation to every RPC.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DaemonResponseEnvelope {
    Response(ResponseEnvelope),
    BoundaryError(BoundaryErrorEnvelope),
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

pub fn decode_daemon_response(
    bytes: &[u8],
    expected_request_id: RequestId,
) -> Result<DaemonResponseEnvelope, MachineError> {
    require_nonzero_request_id(expected_request_id)?;
    #[allow(clippy::large_enum_variant)]
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum WireEnvelope {
        Response(ResponseEnvelope),
        BoundaryError(BoundaryErrorEnvelope),
    }

    match decode_response_json::<WireEnvelope>(bytes)? {
        WireEnvelope::Response(envelope) => {
            require_response_version(envelope.version)?;
            if envelope.request_id != expected_request_id {
                return Err(MachineError::new(
                    BoundaryErrorKind::InvalidJson,
                    "response request identity does not match request",
                ));
            }
            Ok(DaemonResponseEnvelope::Response(envelope))
        }
        WireEnvelope::BoundaryError(envelope) => {
            require_response_version(envelope.version)?;
            if envelope
                .request_id
                .is_some_and(|request_id| request_id != expected_request_id)
            {
                return Err(MachineError::new(
                    BoundaryErrorKind::InvalidJson,
                    "boundary error request identity does not match request",
                ));
            }
            Ok(DaemonResponseEnvelope::BoundaryError(envelope))
        }
    }
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

fn scalar_types() -> Vec<MachineScalarDescription> {
    let boolean = |name: &str| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Boolean,
        domain: MachineScalarDomain::Boolean,
    };
    let string = |name: &str| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::String,
        domain: MachineScalarDomain::Utf8String,
    };
    let signed = |name: &str, minimum, maximum| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Number,
        domain: MachineScalarDomain::SignedInteger { minimum, maximum },
    };
    let unsigned = |name: &str, minimum, maximum| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::Number,
        domain: MachineScalarDomain::UnsignedInteger { minimum, maximum },
    };
    let hex = |name: &str, encoded_bytes| MachineScalarDescription {
        name: name.into(),
        json_kind: JsonScalarKind::String,
        domain: MachineScalarDomain::LowercaseHex { encoded_bytes },
    };
    vec![
        boolean("bool"),
        string("string"),
        signed("i64", i64::MIN, i64::MAX),
        MachineScalarDescription {
            name: "bytes_string".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::CanonicalUrlSafeBase64 {
                padding: false,
                whitespace: false,
                canonical_trailing_bits: true,
                maximum_decoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_BYTES as u64,
                maximum_encoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_ENCODED_BYTES as u64,
            },
        },
        unsigned("u8", u8::MIN.into(), u8::MAX.into()),
        unsigned("u16", u16::MIN.into(), u16::MAX.into()),
        unsigned("u32", u32::MIN.into(), u32::MAX.into()),
        unsigned("u64", u64::MIN, u64::MAX),
        hex("workspace_id", crate::WorkspaceId::BYTE_LEN as u8),
        hex("idempotency_key", 16),
        MachineScalarDescription {
            name: "node_id".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::NodeId {
                workspace_bytes: crate::WorkspaceId::BYTE_LEN as u8,
                minimum_serial: 1,
                maximum_serial: u64::MAX,
            },
        },
        hex("snapshot_hash", crate::SnapshotHash::BYTE_LEN as u8),
        hex("change_digest", crate::ChangeDigest::BYTE_LEN as u8),
        hex("machine_schema_digest", MachineSchemaDigest::BYTE_LEN as u8),
        unsigned("revision", 0, u64::MAX),
        unsigned("request_id", 1, u64::MAX),
        unsigned("query_id", 0, u64::MAX),
        MachineScalarDescription {
            name: "draft_symbol".into(),
            json_kind: JsonScalarKind::String,
            domain: MachineScalarDomain::CanonicalIdentifier {
                grammar: "[a-z][a-z0-9_]*".into(),
                minimum_utf8_bytes: 1,
                maximum_utf8_bytes: crate::ids::MAX_DRAFT_SYMBOL_BYTES as u64,
            },
        },
    ]
}

fn machine_field(name: &str, type_expression: &str, required: bool) -> MachineFieldDescription {
    MachineFieldDescription {
        name: name.into(),
        type_expression: if required {
            type_expression.into()
        } else {
            format!("optional<{type_expression}>")
        },
        required,
    }
}
fn unit_payload() -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Unit,
        newtype: None,
        fields: Vec::new(),
    }
}
fn newtype_payload(type_expression: &str) -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Newtype,
        newtype: Some(type_expression.into()),
        fields: Vec::new(),
    }
}
fn record_payload(fields: &[(&str, &str, bool)]) -> PayloadShapeDescription {
    PayloadShapeDescription {
        shape: PayloadShapeKind::Record,
        newtype: None,
        fields: fields
            .iter()
            .map(|(name, ty, required)| machine_field(name, ty, *required))
            .collect(),
    }
}
fn variant_payload(name: &str, payload: PayloadShapeDescription) -> VariantPayloadDescription {
    VariantPayloadDescription {
        name: name.into(),
        payload,
    }
}

fn named_variant(name: &str, variants: Vec<VariantPayloadDescription>) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "adjacently_tagged".into(),
        tag_field: Some("kind".into()),
        content_field: Some("data".into()),
        variants,
    }
}

fn external_variant(
    name: &str,
    variants: Vec<VariantPayloadDescription>,
) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "externally_tagged".into(),
        tag_field: None,
        content_field: None,
        variants,
    }
}

fn unit_variants(
    name: &str,
    values: impl IntoIterator<Item = (&'static str, u8)>,
) -> NamedVariantDescription {
    NamedVariantDescription {
        name: name.into(),
        tagging: "string_enum".into(),
        tag_field: None,
        content_field: None,
        variants: values
            .into_iter()
            .map(|(variant, _)| variant_payload(variant, unit_payload()))
            .collect(),
    }
}

fn named_record(name: &str, fields: &[(&str, &str, bool)]) -> NamedPayloadDescription {
    NamedPayloadDescription {
        name: name.into(),
        payload: record_payload(fields),
    }
}

fn draft_field(
    name: &str,
    field_type: DraftFieldType,
    required: bool,
    declares_symbol: bool,
) -> DraftFieldDescription {
    DraftFieldDescription {
        name: name.to_owned(),
        field_type,
        required,
        nullable: !required,
        declares_symbol,
    }
}

fn structured_records() -> Vec<DraftRecordDescription> {
    use DraftFieldType as T;
    vec![
        DraftRecordDescription {
            name: "create_product_type".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("fields", T::ProductFieldList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "product_field".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "create_sum_type".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("variants", T::SumVariantList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "sum_variant".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("payload", T::TypeDraft, false, false),
            ],
        },
        DraftRecordDescription {
            name: "create_function".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("parameters", T::ParameterList, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("body", T::FunctionBody, false, false),
            ],
        },
        DraftRecordDescription {
            name: "function_parameter".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "function_body".into(),
            fields: vec![
                draft_field("operations", T::ExpressionList, true, false),
                draft_field("return_value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "yielding_body".into(),
            fields: vec![
                draft_field("operations", T::ExpressionList, true, false),
                draft_field("yield_value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "product_field_value".into(),
            fields: vec![
                draft_field("field", T::NodeTarget, true, false),
                draft_field("value", T::Value, true, false),
            ],
        },
        DraftRecordDescription {
            name: "operation_match_arm".into(),
            fields: vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("region", T::NodeTarget, true, false),
            ],
        },
        DraftRecordDescription {
            name: "match_arm".into(),
            fields: vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload_symbol", T::DraftSymbol, false, true),
                draft_field("body", T::YieldingBody, true, false),
            ],
        },
        DraftRecordDescription {
            name: "expression".into(),
            fields: vec![
                draft_field("symbol", T::DraftSymbol, false, true),
                draft_field("operation", T::ExpressionKind, true, false),
            ],
        },
        DraftRecordDescription {
            name: "define_function_body".into(),
            fields: vec![
                draft_field("function", T::NodeId, true, false),
                draft_field("body", T::FunctionBody, true, false),
            ],
        },
        DraftRecordDescription {
            name: "insert_expression".into(),
            fields: vec![
                draft_field("block", T::NodeId, true, false),
                draft_field("before", T::NodeId, false, false),
                draft_field("expression", T::Expression, true, false),
            ],
        },
    ]
}

fn expression_variant(code: crate::transaction::ExpressionDraftCode) -> DraftVariantDescription {
    use crate::transaction::ExpressionDraftCode as C;
    use DraftFieldType as T;
    let (shape, newtype, fields) = match code {
        C::ConstUnit => (PayloadShapeKind::Unit, None, vec![]),
        C::ConstBool => (PayloadShapeKind::Newtype, Some(T::Bool), vec![]),
        C::ConstI64 => (PayloadShapeKind::Newtype, Some(T::I64), vec![]),
        C::ConstBytes => (PayloadShapeKind::Newtype, Some(T::Bytes), vec![]),
        C::AddI64 | C::LtI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesEqual => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::BytesAt => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::BytesSlice => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("start", T::Value, true, false),
                draft_field("length", T::Value, true, false),
            ],
        ),
        C::Call => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("function", T::NodeTarget, true, false),
                draft_field("arguments", T::ValueList, true, false),
            ],
        ),
        C::Hole => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("expected", T::TypeDraft, true, false)],
        ),
        C::If => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("condition", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("then_body", T::YieldingBody, true, false),
                draft_field("else_body", T::YieldingBody, true, false),
            ],
        ),
        C::ForI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("start", T::Value, true, false),
                draft_field("end_exclusive", T::Value, true, false),
                draft_field("step", T::I64, true, false),
                draft_field("initial", T::Value, true, false),
                draft_field("carried", T::TypeDraft, true, false),
                draft_field("index_symbol", T::DraftSymbol, true, true),
                draft_field("carried_symbol", T::DraftSymbol, true, true),
                draft_field("body", T::YieldingBody, true, false),
            ],
        ),
        C::ConstructProduct => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("product", T::NodeTarget, true, false),
                draft_field("fields", T::ProductFieldValueList, true, false),
            ],
        ),
        C::ProjectField => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("field", T::NodeTarget, true, false),
            ],
        ),
        C::ConstructVariant => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload", T::Value, false, false),
            ],
        ),
        C::MatchSum => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("scrutinee", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("arms", T::MatchArmList, true, false),
            ],
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn operation_variant(code: OperationCode) -> DraftVariantDescription {
    use DraftFieldType as T;
    use OperationCode as C;
    let (shape, newtype, fields) = match code {
        C::ConstUnit => (PayloadShapeKind::Unit, None, vec![]),
        C::ConstI64 => (PayloadShapeKind::Newtype, Some(T::I64), vec![]),
        C::ConstBool => (PayloadShapeKind::Newtype, Some(T::Bool), vec![]),
        C::ConstBytes => (PayloadShapeKind::Newtype, Some(T::Bytes), vec![]),
        C::AddI64 | C::LtI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesEqual => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
            ],
        ),
        C::BytesLen => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::BytesAt => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("index", T::Value, true, false),
            ],
        ),
        C::BytesSlice => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("start", T::Value, true, false),
                draft_field("length", T::Value, true, false),
            ],
        ),
        C::Call => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("function", T::NodeTarget, true, false),
                draft_field("arguments", T::ValueList, true, false),
            ],
        ),
        C::Hole => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("expected", T::TypeDraft, true, false)],
        ),
        C::If => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("condition", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("then_region", T::NodeTarget, true, false),
                draft_field("else_region", T::NodeTarget, true, false),
            ],
        ),
        C::ForI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("start", T::Value, true, false),
                draft_field("end_exclusive", T::Value, true, false),
                draft_field("step", T::I64, true, false),
                draft_field("initial", T::Value, true, false),
                draft_field("carried", T::TypeDraft, true, false),
                draft_field("body_region", T::NodeTarget, true, false),
            ],
        ),
        C::Return | C::Yield => (
            PayloadShapeKind::Record,
            None,
            vec![draft_field("value", T::Value, true, false)],
        ),
        C::ConstructProduct => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("product", T::NodeTarget, true, false),
                draft_field("fields", T::ProductFieldValueList, true, false),
            ],
        ),
        C::ProjectField => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("value", T::Value, true, false),
                draft_field("field", T::NodeTarget, true, false),
            ],
        ),
        C::ConstructVariant => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("variant", T::NodeTarget, true, false),
                draft_field("payload", T::Value, false, false),
            ],
        ),
        C::MatchSum => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("scrutinee", T::Value, true, false),
                draft_field("result", T::TypeDraft, true, false),
                draft_field("arms", T::OperationMatchArmList, true, false),
            ],
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn type_variants() -> Vec<DraftVariantDescription> {
    use DraftFieldType as T;
    vec![
        DraftVariantDescription {
            name: "unit".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "bool".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "i64".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "bytes".into(),
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "nominal".into(),
            shape: PayloadShapeKind::Newtype,
            newtype: Some(T::NodeTarget),
            fields: Vec::new(),
        },
    ]
}

fn value_variant(code: crate::transaction::ValueDraftCode) -> DraftVariantDescription {
    use crate::transaction::ValueDraftCode as C;
    use DraftFieldType as T;
    let (shape, newtype, fields) = match code {
        C::FunctionParameter | C::BlockArgument => {
            (PayloadShapeKind::Newtype, Some(T::NodeTarget), Vec::new())
        }
        C::OperationResult => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("operation", T::NodeTarget, true, false),
                draft_field("output", T::U8, true, false),
            ],
        ),
        C::InlineExpression => (
            PayloadShapeKind::Newtype,
            Some(T::ExpressionKind),
            Vec::new(),
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        shape,
        newtype,
        fields,
    }
}

fn semantic_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "canonical_product_field_value",
            &[("field", "node_id", true), ("value", "value_ref", true)],
        ),
        named_record(
            "canonical_match_arm",
            &[("variant", "node_id", true), ("region", "node_id", true)],
        ),
    ]
}

fn semantic_variants() -> Vec<NamedVariantDescription> {
    vec![
        external_variant(
            "semantic_type",
            vec![
                variant_payload("unit", unit_payload()),
                variant_payload("bool", unit_payload()),
                variant_payload("i64", unit_payload()),
                variant_payload("bytes", unit_payload()),
                variant_payload("nominal", newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "value_ref",
            vec![
                variant_payload("function_parameter", newtype_payload("node_id")),
                variant_payload("block_argument", newtype_payload("node_id")),
                variant_payload(
                    "operation_result",
                    record_payload(&[("operation", "node_id", true), ("output", "u8", true)]),
                ),
            ],
        ),
        external_variant(
            "region_role",
            vec![
                variant_payload("if_then", unit_payload()),
                variant_payload("if_else", unit_payload()),
                variant_payload("for_body", unit_payload()),
                variant_payload("match_arm", newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "operation_kind",
            vec![
                variant_payload("const_unit", unit_payload()),
                variant_payload("const_i64", newtype_payload("i64")),
                variant_payload("const_bool", newtype_payload("bool")),
                variant_payload(
                    "add_i64",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "lt_i64",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "call",
                    record_payload(&[
                        ("function", "node_id", true),
                        ("arguments", "list<value_ref>", true),
                    ]),
                ),
                variant_payload(
                    "hole",
                    record_payload(&[("expected", "semantic_type", true)]),
                ),
                variant_payload(
                    "if",
                    record_payload(&[
                        ("condition", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("then_region", "node_id", true),
                        ("else_region", "node_id", true),
                    ]),
                ),
                variant_payload(
                    "for_i64",
                    record_payload(&[
                        ("start", "value_ref", true),
                        ("end_exclusive", "value_ref", true),
                        ("step", "i64", true),
                        ("initial", "value_ref", true),
                        ("carried", "semantic_type", true),
                        ("body_region", "node_id", true),
                    ]),
                ),
                variant_payload("return", record_payload(&[("value", "value_ref", true)])),
                variant_payload("yield", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "construct_product",
                    record_payload(&[
                        ("product", "node_id", true),
                        ("fields", "list<canonical_product_field_value>", true),
                    ]),
                ),
                variant_payload(
                    "project_field",
                    record_payload(&[("value", "value_ref", true), ("field", "node_id", true)]),
                ),
                variant_payload(
                    "construct_variant",
                    record_payload(&[
                        ("variant", "node_id", true),
                        ("payload", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "match_sum",
                    record_payload(&[
                        ("scrutinee", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("arms", "list<canonical_match_arm>", true),
                    ]),
                ),
                variant_payload("const_bytes", newtype_payload("bytes_string")),
                variant_payload("bytes_len", record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "bytes_at",
                    record_payload(&[("value", "value_ref", true), ("index", "value_ref", true)]),
                ),
                variant_payload(
                    "bytes_slice",
                    record_payload(&[
                        ("value", "value_ref", true),
                        ("start", "value_ref", true),
                        ("length", "value_ref", true),
                    ]),
                ),
                variant_payload(
                    "bytes_equal",
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
            ],
        ),
        named_variant(
            "node",
            vec![
                variant_payload(
                    "workspace_root",
                    record_payload(&[("packages", "list<node_id>", true)]),
                ),
                variant_payload(
                    "package",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("modules", "list<node_id>", true),
                        ("entry", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "module",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("types", "list<node_id>", true),
                        ("functions", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_type",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("fields", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_field",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "sum_type",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("variants", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "sum_variant",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("payload", "semantic_type", false),
                    ]),
                ),
                variant_payload(
                    "function",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("parameters", "list<node_id>", true),
                        ("result", "semantic_type", true),
                        ("body", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "parameter",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "region",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("blocks", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "block",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("arguments", "list<node_id>", true),
                        ("operations", "list<node_id>", true),
                        ("terminator", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "block_argument",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "operation",
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("operation", "operation_kind", true),
                    ]),
                ),
            ],
        ),
    ]
}

fn transaction_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "apply_transaction_request",
            &[
                ("transaction", "transaction", true),
                ("response", "transaction_response_spec", true),
            ],
        ),
        named_record(
            "transaction",
            &[
                ("workspace", "workspace_id", true),
                ("base_revision", "revision", true),
                ("idempotency_key", "idempotency_key", false),
                ("mode", "transaction_mode", true),
                ("operations", "list<transaction_operation>", true),
            ],
        ),
        named_record(
            "transaction_response_spec",
            &[("return_symbols", "list<draft_symbol>", true)],
        ),
        named_record(
            "transaction_receipt",
            &[
                ("workspace", "workspace_id", true),
                ("base_revision", "revision", true),
                ("revision", "revision", true),
                ("hash", "snapshot_hash", true),
                ("published", "bool", true),
                ("created_count", "u64", true),
                (
                    "returned_bindings",
                    "list<tuple<draft_symbol,node_id>>",
                    true,
                ),
                ("change_count", "u64", true),
                ("change_digest", "change_digest", true),
                ("complete_before", "bool", true),
                ("complete_after", "bool", true),
                ("blocker_count_before", "u64", true),
                ("blocker_count_after", "u64", true),
            ],
        ),
    ]
}

fn transaction_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "transaction_operation",
            TransactionOpCode::ALL
                .into_iter()
                .map(transaction_payload)
                .collect(),
        ),
        named_variant(
            "node_target",
            vec![
                variant_payload("existing", newtype_payload("node_id")),
                variant_payload("draft", newtype_payload("draft_symbol")),
            ],
        ),
        unit_variants("transaction_mode", [("commit", 1), ("validate_only", 2)]),
    ]
}

fn run_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "run_policy",
            &[("fuel", "u64", true), ("maximum_frames", "u32", true)],
        ),
        named_record(
            "run_result",
            &[
                ("value", "runtime_value", true),
                ("compile_nanoseconds", "u64", true),
                ("execute_nanoseconds", "u64", true),
            ],
        ),
        named_record(
            "runtime_field_value",
            &[("field", "node_id", true), ("value", "runtime_value", true)],
        ),
        named_record(
            "runtime_product_data",
            &[
                ("ty", "node_id", true),
                ("fields", "list<runtime_field_value>", true),
            ],
        ),
        named_record(
            "runtime_sum_data",
            &[
                ("ty", "node_id", true),
                ("variant", "node_id", true),
                ("payload", "runtime_value", false),
            ],
        ),
    ]
}

fn run_variants() -> Vec<NamedVariantDescription> {
    vec![named_variant(
        "runtime_value",
        vec![
            variant_payload("unit", unit_payload()),
            variant_payload("bool", newtype_payload("bool")),
            variant_payload("i64", newtype_payload("i64")),
            variant_payload("bytes", newtype_payload("bytes_string")),
            variant_payload("product", newtype_payload("runtime_product_data")),
            variant_payload("sum", newtype_payload("runtime_sum_data")),
        ],
    )]
}

fn error_records() -> Vec<NamedPayloadDescription> {
    vec![named_record(
        "boundary_error",
        &[
            ("kind", "boundary_error_kind", true),
            ("message", "string", true),
        ],
    )]
}

fn error_variants() -> Vec<NamedVariantDescription> {
    vec![unit_variants(
        "boundary_error_kind",
        [
            ("invalid_json", 1),
            ("input_too_large", 2),
            ("transport", 3),
            ("output", 4),
            ("usage", 5),
        ],
    )]
}

fn identity_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "request",
            RequestCode::ALL.into_iter().map(request_payload).collect(),
        ),
        named_variant(
            "response",
            ResponseCode::ALL
                .into_iter()
                .map(response_payload)
                .collect(),
        ),
    ]
}

fn request_payload(code: RequestCode) -> VariantPayloadDescription {
    let payload = match code {
        RequestCode::CreateWorkspace | RequestCode::Shutdown => unit_payload(),
        RequestCode::DescribeSchema => newtype_payload("describe_schema_request"),
        RequestCode::ApplyTransaction => newtype_payload("apply_transaction_request"),
        RequestCode::QueryBatch => newtype_payload("query_batch_request"),
        RequestCode::Run => record_payload(&[
            ("workspace", "workspace_id", true),
            ("revision", "revision", true),
            ("entry", "node_id", true),
            ("arguments", "list<runtime_value>", true),
            ("policy", "run_policy", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn response_payload(code: ResponseCode) -> VariantPayloadDescription {
    let payload = match code {
        ResponseCode::WorkspaceCreated => newtype_payload("workspace_summary"),
        ResponseCode::TransactionReceipt => newtype_payload("transaction_receipt"),
        ResponseCode::QueryBatchResult => newtype_payload("query_batch_result"),
        ResponseCode::Run => newtype_payload("run_result"),
        ResponseCode::Acknowledged => unit_payload(),
        ResponseCode::Error => newtype_payload("error"),
        ResponseCode::DescribeSchema => newtype_payload("describe_schema_result"),
    };
    variant_payload(code.machine_name(), payload)
}

fn transaction_payload(code: TransactionOpCode) -> VariantPayloadDescription {
    let payload = match code {
        TransactionOpCode::CreatePackage => {
            record_payload(&[("symbol", "draft_symbol", true), ("name", "string", true)])
        }
        TransactionOpCode::CreateModule => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("package", "node_target", true),
            ("name", "string", true),
        ]),
        TransactionOpCode::CreateProductType => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("fields", "list<product_field>", true),
        ]),
        TransactionOpCode::CreateSumType => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("variants", "list<sum_variant>", true),
        ]),
        TransactionOpCode::CreateFunction => record_payload(&[
            ("symbol", "draft_symbol", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("parameters", "list<function_parameter>", true),
            ("result", "type_draft", true),
            ("body", "function_body", false),
        ]),
        TransactionOpCode::DefineFunctionBody => record_payload(&[
            ("function", "node_id", true),
            ("body", "function_body", true),
        ]),
        TransactionOpCode::InsertExpression => record_payload(&[
            ("block", "node_id", true),
            ("before", "node_id", false),
            ("expression", "expression", true),
        ]),
        TransactionOpCode::SetEntryFunction => record_payload(&[
            ("package", "node_target", true),
            ("function", "node_target", true),
        ]),
        TransactionOpCode::RenameNode => {
            record_payload(&[("node", "node_target", true), ("name", "string", true)])
        }
        TransactionOpCode::ReplaceOperation => record_payload(&[
            ("operation", "node_target", true),
            ("replacement", "operation_draft", true),
        ]),
        TransactionOpCode::ReplaceOperand => record_payload(&[
            ("operation", "node_target", true),
            ("index", "u64", true),
            ("value", "value_draft", true),
        ]),
        TransactionOpCode::DeleteOwnedSubtree => record_payload(&[("root", "node_target", true)]),
        TransactionOpCode::RefineHole => record_payload(&[
            ("hole", "node_target", true),
            ("replacement", "operation_draft", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn query_payload(code: QueryCode) -> VariantPayloadDescription {
    let payload = match code {
        QueryCode::WorkspaceSummary => unit_payload(),
        QueryCode::Node => record_payload(&[("node", "node_id", true), ("expand", "bool", true)]),
        QueryCode::Blockers => record_payload(&[("page", "page_request", true)]),
        QueryCode::OwnerChain => {
            record_payload(&[("node", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::Body => {
            record_payload(&[("block", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::IncomingUses => {
            record_payload(&[("value", "value_ref", true), ("page", "page_request", true)])
        }
        QueryCode::DefinitionReferences => {
            record_payload(&[("target", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::Dependencies => {
            record_payload(&[("node", "node_id", true), ("page", "page_request", true)])
        }
        QueryCode::VisibleValues => record_payload(&[
            ("purpose", "visible_cursor_purpose", true),
            ("target", "repair_target", true),
            ("include_incompatible", "bool", true),
            ("page", "page_request", true),
        ]),
        QueryCode::LegalConstructors => record_payload(&[
            ("target", "repair_target", true),
            ("include_incompatible", "bool", true),
            ("constructors", "page_request", true),
            ("values", "page_request", true),
        ]),
        QueryCode::SemanticDiff => {
            record_payload(&[("from", "revision", true), ("page", "page_request", true)])
        }
        QueryCode::RepairContext => record_payload(&[
            ("target", "repair_target", true),
            ("budget", "context_budget", true),
        ]),
        QueryCode::NominalType => record_payload(&[
            ("declaration", "node_id", true),
            ("page", "page_request", true),
        ]),
    };
    variant_payload(code.machine_name(), payload)
}

fn query_result_payload(code: QueryCode) -> VariantPayloadDescription {
    let ty = match code {
        QueryCode::WorkspaceSummary => "workspace_summary",
        QueryCode::Node => "node_view",
        QueryCode::Blockers => "page<completeness_blocker>",
        QueryCode::OwnerChain => "page<owner_fact>",
        QueryCode::Body => "page<body_item>",
        QueryCode::IncomingUses => "page<use_site>",
        QueryCode::DefinitionReferences => "page<definition_reference_site>",
        QueryCode::Dependencies => "page<dependency_fact>",
        QueryCode::VisibleValues => "page<visible_value>",
        QueryCode::LegalConstructors => "legal_constructors_result",
        QueryCode::SemanticDiff => "semantic_diff_page",
        QueryCode::RepairContext => "repair_context",
        QueryCode::NominalType => "nominal_type_result",
    };
    variant_payload(code.machine_name(), newtype_payload(ty))
}

fn query_records() -> Vec<NamedPayloadDescription> {
    vec![
        named_record(
            "query_batch_request",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("queries", "list<query_item>", true),
            ],
        ),
        named_record(
            "query_item",
            &[("id", "query_id", true), ("query", "query", true)],
        ),
        named_record(
            "query_batch_result",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("results", "list<query_item_result>", true),
            ],
        ),
        named_record(
            "query_item_result",
            &[("id", "query_id", true), ("outcome", "query_outcome", true)],
        ),
        named_record(
            "workspace_summary",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("hash", "snapshot_hash", true),
                ("root", "node_id", true),
                ("node_count", "u64", true),
                ("complete", "bool", true),
                ("blocker_count", "u64", true),
                ("entry_count", "u64", true),
            ],
        ),
        named_record(
            "function_signature_summary",
            &[
                ("parameter_count", "u64", true),
                ("result", "semantic_type", true),
            ],
        ),
        named_record(
            "name_preview",
            &[("value", "string", true), ("truncated", "bool", true)],
        ),
        named_record(
            "node_summary",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("node", "node_id", true),
                ("kind", "node_kind", true),
                ("owner", "node_id", false),
                ("display_name", "name_preview", false),
                ("signature", "function_signature_summary", false),
                ("value_type", "semantic_type", false),
                ("complete", "bool", true),
                ("blocker_count", "u64", true),
                ("child_count", "u64", true),
                ("outgoing_reference_count", "u64", true),
            ],
        ),
        named_record(
            "node_view",
            &[("summary", "node_summary", true), ("record", "node", false)],
        ),
        named_record(
            "completeness_blocker",
            &[
                ("owner", "node_id", true),
                ("target", "node_id", false),
                ("category", "expected_category", true),
                ("expected_type", "semantic_type", false),
            ],
        ),
        named_record(
            "owner_fact",
            &[
                ("node", "node_id", true),
                ("kind", "node_kind", true),
                ("name", "name_preview", false),
            ],
        ),
        named_record(
            "owned_region_summary",
            &[("region", "node_id", true), ("role", "region_role", true)],
        ),
        named_record(
            "body_item",
            &[
                ("operation", "node_id", true),
                ("ordinal", "u64", true),
                ("code", "operation_code", true),
                ("result_types", "list<semantic_type>", true),
                ("operands", "list<value_ref>", true),
                ("definitions", "list<definition_reference_site>", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
                ("literal", "literal_value", false),
                ("owned_regions", "list<owned_region_summary>", true),
            ],
        ),
        named_record(
            "use_site",
            &[
                ("source", "node_id", true),
                ("operand_index", "u64", true),
                ("target", "value_ref", true),
                ("owner_block", "node_id", true),
                ("owner_function", "node_id", true),
                ("expected_type", "semantic_type", true),
                ("use_mode", "operand_use", true),
            ],
        ),
        named_record(
            "definition_reference_site",
            &[
                ("source", "node_id", true),
                ("slot", "definition_slot", true),
                ("target", "node_id", true),
            ],
        ),
        named_record(
            "visible_value",
            &[
                ("value", "value_ref", true),
                ("ty", "semantic_type", true),
                ("compatible", "bool", true),
                ("producer", "node_id", true),
                ("producer_code", "operation_code", false),
                ("owner_function", "node_id", true),
                ("ordinal", "u64", false),
                ("name", "name_preview", false),
            ],
        ),
        named_record(
            "legal_constructors_result",
            &[
                ("target", "repair_target", true),
                ("expected_type", "semantic_type", true),
                ("constructors", "page<constructor_descriptor>", true),
                ("visible_values", "page<visible_value>", true),
            ],
        ),
        named_record(
            "context_budget",
            &[
                ("body_before", "u32", true),
                ("body_after", "u32", true),
                ("visible_values", "u32", true),
                ("incoming_uses", "u32", true),
                ("include_incompatible", "bool", true),
            ],
        ),
        named_record(
            "semantic_diff_page",
            &[
                ("from", "revision", true),
                ("to", "revision", true),
                ("change_count", "u64", true),
                ("change_digest", "change_digest", true),
                ("page", "page<change>", true),
            ],
        ),
        named_record(
            "block_argument_fact",
            &[
                ("argument", "node_id", true),
                ("block", "node_id", true),
                ("region", "node_id", true),
                ("ordinal", "u32", true),
                ("role", "block_argument_role", true),
                ("ty", "semantic_type", true),
            ],
        ),
        named_record(
            "enclosing_region_fact",
            &[
                ("region", "node_id", true),
                ("owner_operation", "node_id", true),
                ("role", "region_role", true),
            ],
        ),
        named_record(
            "repair_context",
            &[
                ("workspace", "workspace_id", true),
                ("revision", "revision", true),
                ("target", "repair_target", true),
                ("operation", "node_id", true),
                ("operation_code", "operation_code", true),
                ("operand_index", "u64", false),
                ("expected_type", "semantic_type", true),
                ("use_mode", "operand_use", false),
                ("current_value", "value_ref", false),
                ("current_actual_type", "semantic_type", false),
                ("owner_block", "node_id", true),
                ("owner_function", "node_id", true),
                ("ordinal", "u64", true),
                ("function_signature", "function_signature_summary", true),
                ("owner_chain", "list<owner_fact>", true),
                ("enclosing_regions", "list<enclosing_region_fact>", true),
                ("visible_block_arguments", "list<block_argument_fact>", true),
                ("body_window", "list<body_item>", true),
                ("visible_values", "page<visible_value>", true),
                ("incoming_uses", "page<use_site>", true),
                ("legal_constructor_count", "u64", true),
                ("legal_constructors", "list<constructor_descriptor>", true),
                ("nominal_type", "nominal_type_result", false),
                (
                    "nominal_type_continuation",
                    "nominal_type_continuation",
                    false,
                ),
                ("blocker", "completeness_blocker", false),
                ("refinement_operation", "transaction_operation_code", false),
            ],
        ),
        NamedPayloadDescription {
            name: "nominal_type_result".into(),
            payload: record_payload(&[
                ("declaration", "node_id", true),
                ("name", "string", true),
                ("kind", "node_kind", true),
                ("owner", "node_id", true),
                ("layout", "nominal_layout_summary", true),
                ("members", "page<nominal_member_fact>", true),
            ]),
        },
        NamedPayloadDescription {
            name: "nominal_layout_summary".into(),
            payload: record_payload(&[
                ("representable", "bool", true),
                ("failure", "layout_failure", false),
                ("size", "u64", false),
                ("align", "u64", false),
                ("cells", "u64", false),
                ("discriminant_bytes", "u8", false),
                ("payload_offset", "u64", false),
            ]),
        },
        NamedPayloadDescription {
            name: "constructor_descriptor".into(),
            payload: record_payload(&[
                ("code", "operation_code", true),
                ("result_type", "semantic_type", true),
                ("operand_count", "u64", true),
                ("operand_types", "list<semantic_type>", true),
                ("operand_uses", "list<operand_use>", true),
                ("literal_fields", "list<literal_field>", true),
                ("call_target", "node_id", false),
                ("declaration", "node_id", false),
                ("member_count", "u64", true),
                ("members", "list<node_id>", true),
                ("requirements_complete", "bool", true),
                (
                    "nominal_type_continuation",
                    "nominal_type_continuation",
                    false,
                ),
                ("direct_refinement", "bool", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
            ]),
        },
        NamedPayloadDescription {
            name: "nominal_type_continuation".into(),
            payload: record_payload(&[
                ("declaration", "node_id", true),
                ("page", "page_request", true),
            ]),
        },
        NamedPayloadDescription {
            name: "page_request".into(),
            payload: record_payload(&[("after", "page_cursor", false), ("limit", "u32", true)]),
        },
        named_record(
            "page",
            &[
                ("items", "list<type_parameter>", true),
                ("next", "page_cursor", false),
                ("total", "u64", false),
            ],
        ),
        named_record(
            "change",
            &[("node", "node_id", true), ("kind", "change_kind", true)],
        ),
    ]
}

fn query_variants() -> Vec<NamedVariantDescription> {
    vec![
        named_variant(
            "query",
            QueryCode::ALL.into_iter().map(query_payload).collect(),
        ),
        named_variant(
            "query_result",
            QueryCode::ALL
                .into_iter()
                .map(query_result_payload)
                .collect(),
        ),
        named_variant("nominal_member_fact", query_member_payloads()),
        named_variant("page_cursor", query_cursor_payloads()),
        named_variant(
            "query_outcome",
            vec![
                variant_payload("success", newtype_payload("query_result")),
                variant_payload("error", newtype_payload("error")),
            ],
        ),
        named_variant(
            "repair_target",
            vec![
                variant_payload("hole", newtype_payload("node_id")),
                variant_payload(
                    "operand",
                    record_payload(&[("operation", "node_id", true), ("index", "u64", true)]),
                ),
            ],
        ),
        unit_variants(
            "expected_category",
            [
                ("entry_function", 1),
                ("function_body", 2),
                ("expression", 3),
            ],
        ),
        unit_variants(
            "visible_cursor_purpose",
            [
                ("visible_values", 1),
                ("legal_constructors", 2),
                ("repair_context", 3),
            ],
        ),
        unit_variants(
            "layout_failure",
            [
                ("byte_size_overflow", 1),
                ("cell_count_overflow", 2),
                ("invalid_dependency", 3),
            ],
        ),
        unit_variants(
            "definition_slot",
            [
                ("package_entry", 1),
                ("call_target", 2),
                ("function_result_type", 3),
                ("parameter_type", 4),
                ("product_field_type", 5),
                ("sum_variant_payload_type", 6),
                ("block_argument_type", 7),
                ("operation_type", 8),
                ("product_declaration", 9),
                ("product_field", 10),
                ("sum_variant", 11),
                ("match_variant", 12),
            ],
        ),
        named_variant(
            "literal_value",
            vec![
                variant_payload("i64", newtype_payload("i64")),
                variant_payload("bool", newtype_payload("bool")),
                variant_payload("expected_type", newtype_payload("semantic_type")),
                variant_payload("bytes", newtype_payload("bytes_string")),
            ],
        ),
        named_variant(
            "dependency_fact",
            vec![
                variant_payload(
                    "value_operand",
                    record_payload(&[("index", "u64", true), ("value", "value_ref", true)]),
                ),
                variant_payload(
                    "definition",
                    record_payload(&[
                        ("slot", "definition_slot", true),
                        ("target", "node_id", true),
                    ]),
                ),
            ],
        ),
        named_variant(
            "scalar_value",
            vec![
                variant_payload("i64", newtype_payload("i64")),
                variant_payload("bool", newtype_payload("bool")),
                variant_payload("type", newtype_payload("semantic_type")),
                variant_payload("bytes", newtype_payload("bytes_string")),
            ],
        ),
        named_variant(
            "change_kind",
            vec![
                variant_payload("created", record_payload(&[("kind", "node_kind", true)])),
                variant_payload("deleted", record_payload(&[("kind", "node_kind", true)])),
                variant_payload(
                    "renamed",
                    record_payload(&[("before", "string", true), ("after", "string", true)]),
                ),
                variant_payload(
                    "scalar_attribute_changed",
                    record_payload(&[
                        ("before", "scalar_value", true),
                        ("after", "scalar_value", true),
                    ]),
                ),
                variant_payload(
                    "containment_changed",
                    record_payload(&[("before_count", "u64", true), ("after_count", "u64", true)]),
                ),
                variant_payload(
                    "operand_changed",
                    record_payload(&[
                        ("index", "u64", true),
                        ("before", "value_ref", false),
                        ("after", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "definition_changed",
                    record_payload(&[("before", "node_id", true), ("after", "node_id", true)]),
                ),
                variant_payload(
                    "entry_function_changed",
                    record_payload(&[("before", "node_id", false), ("after", "node_id", false)]),
                ),
                variant_payload(
                    "completeness_changed",
                    record_payload(&[("complete", "bool", true)]),
                ),
                variant_payload(
                    "operation_refined",
                    record_payload(&[
                        ("before", "operation_code", true),
                        ("after", "operation_code", true),
                        ("result_type", "semantic_type", true),
                        ("replacement", "operation_kind", true),
                    ]),
                ),
                variant_payload("allocated_and_tombstoned", unit_payload()),
            ],
        ),
    ]
}

fn query_member_payloads() -> Vec<VariantPayloadDescription> {
    vec![
        variant_payload(
            "product_field",
            record_payload(&[
                ("field", "node_id", true),
                ("name", "string", true),
                ("ordinal", "u32", true),
                ("ty", "semantic_type", true),
                ("offset", "u64", false),
                ("cells", "u64", false),
            ]),
        ),
        variant_payload(
            "sum_variant",
            record_payload(&[
                ("variant", "node_id", true),
                ("name", "string", true),
                ("ordinal", "u32", true),
                ("payload", "semantic_type", false),
                ("discriminant", "u64", false),
                ("payload_size", "u64", false),
                ("payload_align", "u64", false),
                ("payload_cells", "u64", false),
            ]),
        ),
    ]
}

fn query_cursor_payloads() -> Vec<VariantPayloadDescription> {
    let common = |extra: &[(&str, &str, bool)]| {
        let mut fields = vec![
            ("workspace", "workspace_id", true),
            ("revision", "revision", true),
        ];
        fields.extend_from_slice(extra);
        record_payload(&fields)
    };
    vec![
        variant_payload("blockers", common(&[("next", "u64", true)])),
        variant_payload(
            "owner_chain",
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "body",
            common(&[("block", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "incoming_uses",
            common(&[("value", "value_ref", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "definition_references",
            common(&[("target", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "dependencies",
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "visible_values",
            common(&[
                ("purpose", "visible_cursor_purpose", true),
                ("target", "repair_target", true),
                ("expected", "semantic_type", true),
                ("include_incompatible", "bool", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "legal_constructors",
            common(&[
                ("target", "repair_target", true),
                ("expected", "semantic_type", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "diff",
            record_payload(&[
                ("workspace", "workspace_id", true),
                ("from", "revision", true),
                ("to", "revision", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "nominal_type",
            common(&[("declaration", "node_id", true), ("next", "u64", true)]),
        ),
    ]
}

fn error_payload() -> PayloadShapeDescription {
    record_payload(&[
        ("code", "error_code", true),
        ("workspace", "workspace_id", false),
        ("revision", "revision", false),
        ("operation_index", "u32", false),
        ("draft_symbol", "draft_symbol", false),
        ("draft_path", "string", false),
        ("target", "node_id", false),
        ("expected_kind", "node_kind", false),
        ("actual_kind", "node_kind", false),
        ("expected_type", "semantic_type", false),
        ("actual_type", "semantic_type", false),
        ("related", "list<node_id>", true),
        ("retryable", "bool", true),
        ("message", "string", true),
    ])
}

fn envelope_payloads() -> Vec<NamedPayloadDescription> {
    vec![
        NamedPayloadDescription {
            name: "request_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", true),
                ("request", "request", true),
            ]),
        },
        NamedPayloadDescription {
            name: "response_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", true),
                ("response", "response", true),
            ]),
        },
        NamedPayloadDescription {
            name: "boundary_error_envelope".into(),
            payload: record_payload(&[
                ("version", "u16", true),
                ("request_id", "request_id", false),
                ("error", "boundary_error", true),
            ]),
        },
    ]
}

fn schema_discovery_records() -> Vec<NamedPayloadDescription> {
    let record = |name: &str, fields: &[(&str, &str, bool)]| NamedPayloadDescription {
        name: name.into(),
        payload: record_payload(fields),
    };
    vec![
        record(
            "machine_field_description",
            &[
                ("name", "string", true),
                ("type_expression", "string", true),
                ("required", "bool", true),
            ],
        ),
        record(
            "payload_shape_description",
            &[
                ("shape", "payload_shape_kind", true),
                ("newtype", "string", false),
                ("fields", "list<machine_field_description>", true),
            ],
        ),
        record(
            "variant_payload_description",
            &[
                ("name", "string", true),
                ("payload", "payload_shape_description", true),
            ],
        ),
        record(
            "named_payload_description",
            &[
                ("name", "string", true),
                ("payload", "payload_shape_description", true),
            ],
        ),
        record(
            "named_variant_description",
            &[
                ("name", "string", true),
                ("tagging", "string", true),
                ("tag_field", "string", false),
                ("content_field", "string", false),
                ("variants", "list<variant_payload_description>", true),
            ],
        ),
        record("code_description", &[("name", "string", true)]),
        record(
            "draft_field_type_description",
            &[
                ("name", "string", true),
                ("type_expression", "string", true),
            ],
        ),
        record(
            "draft_field_description",
            &[
                ("name", "string", true),
                ("field_type", "draft_field_type", true),
                ("required", "bool", true),
                ("nullable", "bool", true),
                ("declares_symbol", "bool", true),
            ],
        ),
        record(
            "draft_record_description",
            &[
                ("name", "string", true),
                ("fields", "list<draft_field_description>", true),
            ],
        ),
        record(
            "draft_variant_description",
            &[
                ("name", "string", true),
                ("shape", "payload_shape_kind", true),
                ("newtype", "draft_field_type", false),
                ("fields", "list<draft_field_description>", true),
            ],
        ),
        record(
            "structured_authoring_description",
            &[
                (
                    "draft_field_types",
                    "list<draft_field_type_description>",
                    true,
                ),
                ("records", "list<draft_record_description>", true),
                (
                    "expression_variants",
                    "list<draft_variant_description>",
                    true,
                ),
                (
                    "operation_variants",
                    "list<draft_variant_description>",
                    true,
                ),
                ("value_variants", "list<draft_variant_description>", true),
                ("type_variants", "list<draft_variant_description>", true),
                ("expression_tagging", "string", true),
                ("operation_tagging", "string", true),
                ("value_tagging", "string", true),
                ("type_tagging", "string", true),
                ("allocation_order", "string", true),
                ("inline_expression_variants", "list<string>", true),
                ("inline_holes_allowed", "bool", true),
                ("inline_region_operations_allowed", "bool", true),
                ("maintenance_accepts_inline_values", "bool", true),
                ("nesting_metric", "string", true),
                ("explicit_symbols_are_selectable", "bool", true),
                ("implicit_symbols_are_selectable", "bool", true),
                ("implicit_node_kinds", "list<node_kind>", true),
                ("maximum_request_depth", "u32", true),
                ("maximum_request_items", "u64", true),
                ("counted_item_categories", "list<string>", true),
            ],
        ),
        record(
            "operand_description",
            &[("ty", "type_rule", true), ("use_mode", "operand_use", true)],
        ),
        record(
            "block_argument_description",
            &[
                ("role", "block_argument_role", true),
                ("ty", "type_rule", true),
            ],
        ),
        record(
            "region_description",
            &[
                ("role", "region_role", true),
                ("block_arguments", "list<block_argument_description>", true),
                ("terminator", "operation_code", true),
                ("yield_type", "type_rule", true),
            ],
        ),
        record(
            "operation_description",
            &[
                ("name", "string", true),
                ("operand_arity", "operand_arity", true),
                ("operands", "list<operand_description>", true),
                ("results", "list<type_rule>", true),
                ("literal_fields", "list<literal_field>", true),
                ("region_arity", "region_arity", true),
                ("regions", "list<region_description>", true),
                ("complete", "bool", true),
                ("terminator", "bool", true),
            ],
        ),
        record(
            "run_field_description",
            &[
                ("name", "string", true),
                ("field_type", "run_field_type", true),
                ("required", "bool", true),
            ],
        ),
        record(
            "runtime_value_description",
            &[
                ("name", "string", true),
                ("payload", "runtime_value_payload", true),
                ("fields", "list<machine_field_description>", true),
                ("invariants", "list<string>", true),
            ],
        ),
        record(
            "run_description",
            &[
                ("fields", "list<run_field_description>", true),
                ("policy_fields", "list<run_field_description>", true),
                ("runtime_values", "list<runtime_value_description>", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                ("limit_scope", "list<string>", true),
            ],
        ),
        record(
            "schema_discovery_description",
            &[
                ("digest_format", "string", true),
                ("digest_domain", "string", true),
                ("request", "payload_shape_description", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                (
                    "projection_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("result_payloads", "list<variant_payload_description>", true),
                ("roots", "list<string>", true),
                ("type_constructors", "list<string>", true),
                ("maximum_roots_per_request", "u8", true),
                ("full_available", "bool", true),
                ("known_digest_match_follows_root_validation", "bool", true),
            ],
        ),
        record(
            "schema_description",
            &[
                ("machine_schema_identity", "string", true),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("schema_discovery", "schema_discovery_description", true),
                ("scalar_types", "list<machine_scalar_description>", true),
                ("semantic_types", "list<code_description>", true),
                ("node_kinds", "list<code_description>", true),
                ("name_contract", "name_contract_description", true),
                ("operations", "list<operation_description>", true),
                ("semantic_records", "list<named_payload_description>", true),
                ("semantic_variants", "list<named_variant_description>", true),
                ("transaction_operations", "list<code_description>", true),
                (
                    "transaction_operation_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                (
                    "transaction_records",
                    "list<named_payload_description>",
                    true,
                ),
                (
                    "transaction_variants",
                    "list<named_variant_description>",
                    true,
                ),
                (
                    "structured_authoring",
                    "structured_authoring_description",
                    true,
                ),
                ("run", "run_description", true),
                ("queries", "list<code_description>", true),
                ("query_payloads", "list<variant_payload_description>", true),
                (
                    "query_result_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("query_records", "list<named_payload_description>", true),
                ("query_variants", "list<named_variant_description>", true),
                (
                    "query_member_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                (
                    "query_cursor_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("errors", "list<code_description>", true),
                ("error_payload", "payload_shape_description", true),
                ("error_records", "list<named_payload_description>", true),
                ("error_variants", "list<named_variant_description>", true),
                ("requests", "list<code_description>", true),
                (
                    "request_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("responses", "list<code_description>", true),
                (
                    "response_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("identity_variants", "list<named_variant_description>", true),
                ("envelopes", "list<named_payload_description>", true),
                ("boundary_error_kinds", "list<string>", true),
                ("limits", "boundary_limits", true),
                ("id_formats", "id_formats_description", true),
                (
                    "nominal_declarations",
                    "nominal_declarations_description",
                    true,
                ),
            ],
        ),
        record(
            "machine_scalar_description",
            &[
                ("name", "string", true),
                ("json_kind", "json_scalar_kind", true),
                ("domain", "machine_scalar_domain", true),
            ],
        ),
        record(
            "name_contract_description",
            &[
                ("named_node_kinds", "list<node_kind>", true),
                ("minimum_utf8_bytes", "u64", true),
                ("maximum_utf8_bytes", "u64", true),
                (
                    "sibling_uniqueness_groups",
                    "list<name_uniqueness_group_description>",
                    true,
                ),
            ],
        ),
        record(
            "name_uniqueness_group_description",
            &[
                ("name", "string", true),
                ("owner_kind", "node_kind", true),
                ("member_kinds", "list<node_kind>", true),
            ],
        ),
        record(
            "boundary_limits",
            &[
                ("maximum_request_frame_bytes", "u64", true),
                ("maximum_response_frame_bytes", "u64", true),
                ("maximum_artifact_bytes", "u64", true),
                ("maximum_artifact_name_bytes", "u64", true),
                ("maximum_json_input_bytes", "u64", true),
                ("maximum_json_output_bytes", "u64", true),
                ("maximum_page_items", "u32", true),
                ("maximum_batch_queries", "u32", true),
                ("maximum_batch_items", "u32", true),
                ("maximum_context_items_per_category", "u32", true),
                ("maximum_returned_bindings", "u32", true),
                ("maximum_run_arguments", "u32", true),
                ("maximum_run_fuel", "u64", true),
                ("maximum_run_frames", "u32", true),
                ("maximum_run_live_cells", "u64", true),
                ("maximum_runtime_value_depth", "u32", true),
                ("maximum_runtime_value_items", "u64", true),
                ("maximum_runtime_value_bytes", "u64", true),
                ("maximum_byte_literal_bytes", "u64", true),
                ("maximum_transaction_byte_literal_bytes", "u64", true),
                ("maximum_runtime_byte_value_bytes", "u64", true),
                ("maximum_run_argument_byte_bytes", "u64", true),
                ("maximum_run_managed_visible_bytes", "u64", true),
                ("maximum_run_retained_backing_bytes", "u64", true),
                ("maximum_run_managed_objects", "u64", true),
                ("maximum_error_related_ids", "u32", true),
                ("maximum_boundary_error_message_bytes", "u64", true),
                ("maximum_persistence_head_bytes", "u64", true),
            ],
        ),
        record(
            "id_formats_description",
            &[
                ("workspace", "string", true),
                ("idempotency_key", "string", true),
                ("node", "string", true),
                ("snapshot_hash", "string", true),
                ("change_digest", "string", true),
                ("revision", "string", true),
                ("request_id", "string", true),
                ("query_id", "string", true),
                ("draft_symbol", "string", true),
                ("machine_schema_digest", "string", true),
            ],
        ),
        record(
            "nominal_declarations_description",
            &[
                ("declaration_kinds", "list<node_kind>", true),
                ("member_kinds", "list<node_kind>", true),
                ("shape_invariants", "list<string>", true),
                ("layout_invariants", "list<string>", true),
            ],
        ),
        record(
            "schema_manifest",
            &[
                ("schema_identity", "string", true),
                ("digest", "machine_schema_digest", true),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("roots", "list<string>", true),
                ("type_constructors", "list<string>", true),
                ("maximum_roots_per_request", "u8", true),
                ("full_available", "bool", true),
                ("maximum_request_frame_bytes", "u64", true),
                ("maximum_response_frame_bytes", "u64", true),
                ("maximum_json_output_bytes", "u64", true),
            ],
        ),
        record(
            "schema_definitions",
            &[
                ("digest", "machine_schema_digest", true),
                ("roots", "list<schema_root>", true),
                ("type_constructors", "list<string>", true),
                ("definitions", "list<schema_definition>", true),
            ],
        ),
        record(
            "schema_definition",
            &[
                ("name", "string", true),
                ("dependencies", "list<string>", true),
                ("body", "schema_definition_body", true),
            ],
        ),
        record(
            "draft_variant_family_description",
            &[
                ("name", "string", true),
                ("tagging", "string", true),
                ("variants", "list<draft_variant_description>", true),
            ],
        ),
        record(
            "endpoint_description",
            &[
                ("name", "string", true),
                ("family", "string", true),
                ("template", "string", true),
                (
                    "bindings",
                    "list<endpoint_variant_binding_description>",
                    true,
                ),
                ("protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("boundary_error_envelope", "string", true),
                ("typed_error", "string", true),
                ("id_formats", "string", true),
                ("limits", "string", true),
            ],
        ),
        record(
            "endpoint_variant_binding_description",
            &[
                ("parameter", "string", true),
                ("variant", "variant_payload_description", true),
            ],
        ),
        record(
            "endpoint_protocol_template_description",
            &[
                ("name", "string", true),
                (
                    "parameters",
                    "list<endpoint_template_parameter_description>",
                    true,
                ),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
            ],
        ),
        record(
            "endpoint_template_parameter_description",
            &[
                ("name", "string", true),
                ("target_variant", "string", true),
                ("semantics", "string", true),
            ],
        ),
        record(
            "code_family_description",
            &[("name", "string", true), ("members", "list<string>", true)],
        ),
        record(
            "structured_authoring_policy_description",
            &[
                ("allocation_order", "string", true),
                ("inline_expression_variants", "list<string>", true),
                ("inline_holes_allowed", "bool", true),
                ("inline_region_operations_allowed", "bool", true),
                ("maintenance_accepts_inline_values", "bool", true),
                ("nesting_metric", "string", true),
                ("explicit_symbols_are_selectable", "bool", true),
                ("implicit_symbols_are_selectable", "bool", true),
                ("implicit_node_kinds", "list<node_kind>", true),
                ("maximum_request_depth", "u32", true),
                ("maximum_request_items", "u64", true),
                ("counted_item_categories", "list<string>", true),
            ],
        ),
    ]
}

fn schema_discovery_variants(
    projection_payloads: &[VariantPayloadDescription],
    result_payloads: &[VariantPayloadDescription],
) -> Vec<NamedVariantDescription> {
    vec![
        named_variant("schema_projection", projection_payloads.to_vec()),
        named_variant("describe_schema_result", result_payloads.to_vec()),
        unit_variants(
            "schema_root",
            SchemaRoot::ALL
                .into_iter()
                .map(|root| (root.machine_name(), 0)),
        ),
        named_variant(
            "schema_definition_body",
            vec![
                variant_payload("scalar", newtype_payload("machine_scalar_description")),
                variant_payload("record", newtype_payload("named_payload_description")),
                variant_payload("variant", newtype_payload("named_variant_description")),
                variant_payload("draft_record", newtype_payload("draft_record_description")),
                variant_payload(
                    "draft_variant",
                    newtype_payload("draft_variant_family_description"),
                ),
                variant_payload("endpoint", newtype_payload("endpoint_description")),
                variant_payload(
                    "endpoint_template",
                    newtype_payload("endpoint_protocol_template_description"),
                ),
                variant_payload("codes", newtype_payload("code_family_description")),
                variant_payload("operations", newtype_payload("list<operation_description>")),
                variant_payload(
                    "structured_authoring",
                    newtype_payload("structured_authoring_policy_description"),
                ),
                variant_payload(
                    "name_contract",
                    newtype_payload("name_contract_description"),
                ),
                variant_payload(
                    "nominal_declarations",
                    newtype_payload("nominal_declarations_description"),
                ),
                variant_payload("id_formats", newtype_payload("id_formats_description")),
                variant_payload("limits", newtype_payload("boundary_limits")),
            ],
        ),
        unit_variants(
            "payload_shape_kind",
            [("unit", 1), ("newtype", 2), ("record", 3)],
        ),
        unit_variants(
            "json_scalar_kind",
            [("boolean", 1), ("number", 2), ("string", 3)],
        ),
        named_variant(
            "machine_scalar_domain",
            vec![
                variant_payload("boolean", unit_payload()),
                variant_payload("utf8_string", unit_payload()),
                variant_payload(
                    "signed_integer",
                    record_payload(&[("minimum", "i64", true), ("maximum", "i64", true)]),
                ),
                variant_payload(
                    "unsigned_integer",
                    record_payload(&[("minimum", "u64", true), ("maximum", "u64", true)]),
                ),
                variant_payload(
                    "lowercase_hex",
                    record_payload(&[("encoded_bytes", "u8", true)]),
                ),
                variant_payload(
                    "canonical_url_safe_base64",
                    record_payload(&[
                        ("padding", "bool", true),
                        ("whitespace", "bool", true),
                        ("canonical_trailing_bits", "bool", true),
                        ("maximum_decoded_bytes", "u64", true),
                        ("maximum_encoded_bytes", "u64", true),
                    ]),
                ),
                variant_payload(
                    "node_id",
                    record_payload(&[
                        ("workspace_bytes", "u8", true),
                        ("minimum_serial", "u64", true),
                        ("maximum_serial", "u64", true),
                    ]),
                ),
                variant_payload(
                    "canonical_identifier",
                    record_payload(&[
                        ("grammar", "string", true),
                        ("minimum_utf8_bytes", "u64", true),
                        ("maximum_utf8_bytes", "u64", true),
                    ]),
                ),
            ],
        ),
        unit_variants(
            "run_field_type",
            [
                ("workspace", 1),
                ("revision", 2),
                ("node", 3),
                ("runtime_value_list", 4),
                ("run_policy", 5),
                ("u64", 6),
                ("u32", 7),
            ],
        ),
        unit_variants(
            "runtime_value_payload",
            [
                ("none", 1),
                ("bool", 2),
                ("i64", 3),
                ("bytes", 4),
                ("product", 5),
                ("sum", 6),
            ],
        ),
        unit_variants(
            "draft_field_type",
            DraftFieldType::ALL
                .into_iter()
                .map(|field_type| (field_type.machine_name(), 0)),
        ),
        named_variant(
            "operand_arity",
            vec![
                variant_payload("fixed", newtype_payload("u8")),
                variant_payload("call_target_parameters", unit_payload()),
                variant_payload("product_fields", unit_payload()),
                variant_payload("variant_payload", unit_payload()),
            ],
        ),
        named_variant(
            "region_arity",
            vec![
                variant_payload("fixed", newtype_payload("u8")),
                variant_payload(
                    "match_variants",
                    record_payload(&[
                        ("payload_type", "type_rule", true),
                        ("terminator", "operation_code", true),
                        ("yield_type", "type_rule", true),
                    ]),
                ),
            ],
        ),
        unit_variants("operand_use", [("read", 1)]),
        unit_variants(
            "literal_field",
            [
                ("i64_value", 1),
                ("bool_value", 2),
                ("expected_type", 3),
                ("result_type", 4),
                ("carried_type", 5),
                ("positive_step", 6),
                ("bytes_value", 7),
            ],
        ),
        unit_variants(
            "block_argument_role",
            [("loop_index", 1), ("loop_carried", 2), ("match_payload", 3)],
        ),
        named_variant(
            "type_rule",
            vec![
                variant_payload("fixed", newtype_payload("semantic_type")),
                variant_payload("payload_expected", unit_payload()),
                variant_payload("owner_function_result", unit_payload()),
                variant_payload("payload_result", unit_payload()),
                variant_payload("payload_carried", unit_payload()),
                variant_payload("call_target_parameter", unit_payload()),
                variant_payload("call_target_result", unit_payload()),
                variant_payload("owning_region_yield", unit_payload()),
                variant_payload("product_field_type", unit_payload()),
                variant_payload("product_declaration_result", unit_payload()),
                variant_payload("projection_owner", unit_payload()),
                variant_payload("projected_field_result", unit_payload()),
                variant_payload("variant_payload", unit_payload()),
                variant_payload("variant_owner_result", unit_payload()),
                variant_payload("match_scrutinee", unit_payload()),
                variant_payload("match_result", unit_payload()),
            ],
        ),
    ]
}

fn schema_type_constructors() -> Vec<String> {
    ["list<T>", "optional<T>", "tuple<T,...>", "page<T>"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

fn schema_discovery_description() -> SchemaDiscoveryDescription {
    let projection_payloads = vec![
        variant_payload("manifest", unit_payload()),
        variant_payload(
            "roots",
            record_payload(&[("roots", "list<schema_root>", true)]),
        ),
        variant_payload("full", unit_payload()),
    ];
    let result_payloads = vec![
        variant_payload(
            "unchanged",
            record_payload(&[("digest", "machine_schema_digest", true)]),
        ),
        variant_payload("manifest", newtype_payload("schema_manifest")),
        variant_payload("roots", newtype_payload("schema_definitions")),
        variant_payload(
            "full",
            record_payload(&[
                ("digest", "machine_schema_digest", true),
                ("description", "schema_description", true),
            ]),
        ),
    ];
    SchemaDiscoveryDescription {
        digest_format: "64 lowercase hexadecimal characters encoding 32 bytes".into(),
        digest_domain: MACHINE_SCHEMA_DIGEST_DOMAIN.into(),
        request: record_payload(&[
            ("projection", "schema_projection", true),
            ("known_digest", "machine_schema_digest", false),
        ]),
        records: schema_discovery_records(),
        variants: schema_discovery_variants(&projection_payloads, &result_payloads),
        projection_payloads,
        result_payloads,
        roots: SchemaRoot::ALL
            .into_iter()
            .map(|root| root.machine_name().to_owned())
            .collect(),
        type_constructors: schema_type_constructors(),
        maximum_roots_per_request: MAX_SCHEMA_ROOTS as u8,
        full_available: true,
        known_digest_match_follows_root_validation: true,
    }
}

fn name_contract_description() -> NameContractDescription {
    let sibling_uniqueness_groups = crate::schema::NameUniquenessGroup::ALL
        .into_iter()
        .map(|group| NameUniquenessGroupDescription {
            name: group.machine_name().into(),
            owner_kind: group.owner_kind(),
            member_kinds: group.member_kinds().to_vec(),
        })
        .collect::<Vec<_>>();
    let mut named_node_kinds = Vec::new();
    for kind in sibling_uniqueness_groups
        .iter()
        .flat_map(|group| group.member_kinds.iter().copied())
    {
        if !named_node_kinds.contains(&kind) {
            named_node_kinds.push(kind);
        }
    }
    NameContractDescription {
        named_node_kinds,
        minimum_utf8_bytes: crate::schema::MINIMUM_NAME_UTF8_BYTES as u64,
        maximum_utf8_bytes: crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64,
        sibling_uniqueness_groups,
    }
}

pub fn schema_description() -> SchemaDescription {
    SchemaDescription {
        machine_schema_identity: MACHINE_SCHEMA_IDENTITY.into(),
        protocol_version: PROTOCOL_VERSION,
        json_envelope_version: JSON_ENVELOPE_VERSION,
        artifact_format_version: crate::artifact::FORMAT_VERSION.0,
        artifact_magic_hex: crate::artifact::MAGIC
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        semantic_schema_identity: String::from_utf8_lossy(&crate::artifact::SCHEMA_ID.0).into_owned(),
        schema_discovery: schema_discovery_description(),
        scalar_types: scalar_types(),
        semantic_types: SemanticType::PRIMITIVES
            .into_iter()
            .map(|code| described(code.machine_name()))
            .chain(std::iter::once(described("nominal")))
            .collect(),
        node_kinds: NodeKind::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        name_contract: name_contract_description(),
        operations: OperationCode::ALL
            .into_iter()
            .map(|code| {
                let descriptor = code.descriptor();
                OperationDescription {
                    name: descriptor.machine_name.to_owned(),
                    operand_arity: descriptor.operand_arity,
                    operands: descriptor
                        .operands
                        .iter()
                        .map(|operand| OperandDescription {
                            ty: operand.ty,
                            use_mode: operand.use_mode,
                        })
                        .collect(),
                    results: descriptor.results.to_vec(),
                    literal_fields: descriptor.literal_fields.to_vec(),
                    region_arity: descriptor.region_arity,
                    regions: descriptor
                        .regions
                        .iter()
                        .map(|region| RegionDescription {
                            role: region.role,
                            block_arguments: region
                                .block_arguments
                                .iter()
                                .map(|argument| BlockArgumentDescription {
                                    role: argument.role,
                                    ty: argument.ty,
                                })
                                .collect(),
                            terminator: region.terminator,
                            yield_type: region.yield_type,
                        })
                        .collect(),
                    complete: descriptor.complete,
                    terminator: descriptor.terminator,
                }
            })
            .collect(),
        semantic_records: semantic_records(),
        semantic_variants: semantic_variants(),
        transaction_operations: TransactionOpCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        transaction_operation_payloads: TransactionOpCode::ALL
            .into_iter()
            .map(transaction_payload)
            .collect(),
        transaction_records: transaction_records(),
        transaction_variants: transaction_variants(),
        structured_authoring: StructuredAuthoringDescription {
            draft_field_types: DraftFieldType::ALL
                .into_iter()
                .map(|field_type| DraftFieldTypeDescription {
                    name: field_type.machine_name().into(),
                    type_expression: field_type.type_expression().into(),
                })
                .collect(),
            records: structured_records(),
            expression_variants: crate::transaction::ExpressionDraftCode::ALL
                .into_iter()
                .map(expression_variant)
                .collect(),
            operation_variants: OperationCode::ALL
                .into_iter()
                .map(operation_variant)
                .collect(),
            value_variants: crate::transaction::ValueDraftCode::ALL
                .into_iter()
                .map(value_variant)
                .collect(),
            type_variants: type_variants(),
            expression_tagging: "adjacently_tagged(kind,data)".into(),
            operation_tagging: "adjacently_tagged(kind,data)".into(),
            value_tagging: "adjacently_tagged(kind,data)".into(),
            type_tagging: "externally_tagged; unit variants are strings and nominal is an object keyed by nominal".into(),
            allocation_order: "transaction_order; structured bodies preserve expression order; inline value children are normalized depth-first and left-to-right before their parent; product fields and match arms use declaration order".to_owned(),
            inline_expression_variants: crate::transaction::ExpressionDraftCode::ALL
                .into_iter()
                .filter(|code| code.is_inline_eligible())
                .map(|code| code.machine_name().to_owned())
                .collect(),
            inline_holes_allowed: false,
            inline_region_operations_allowed: false,
            maintenance_accepts_inline_values: false,
            nesting_metric: "maximum number of inline-expression or operation-owned-body edges on one structured proposal path; list wrappers, call arguments, product fields, match-arm labels, and variant payload wrappers do not add depth".to_owned(),
            explicit_symbols_are_selectable: true,
            implicit_symbols_are_selectable: false,
            implicit_node_kinds: vec![
                NodeKind::Region,
                NodeKind::Block,
                NodeKind::BlockArgument,
                NodeKind::Operation,
            ],
            maximum_request_depth: crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH as u32,
            maximum_request_items: crate::transaction::MAX_STRUCTURED_DRAFT_ITEMS as u64,
            counted_item_categories: vec![
                "transaction_operation".into(),
                "function_parameter".into(),
                "product_field".into(),
                "sum_variant".into(),
                "function_body".into(),
                "yielding_body".into(),
                "explicit_or_inline_expression".into(),
                "call_argument".into(),
                "product_binding".into(),
                "match_arm".into(),
            ],
        },
        run: RunDescription {
            fields: [
                ("workspace", RunFieldType::Workspace),
                ("revision", RunFieldType::Revision),
                ("entry", RunFieldType::Node),
                ("arguments", RunFieldType::RuntimeValueList),
                ("policy", RunFieldType::RunPolicy),
            ]
            .into_iter()
            .map(|(name, field_type)| RunFieldDescription {
                name: name.into(),
                field_type,
                required: true,
            })
            .collect(),
            policy_fields: [
                ("fuel", RunFieldType::U64),
                ("maximum_frames", RunFieldType::U32),
            ]
            .into_iter()
            .map(|(name, field_type)| RunFieldDescription {
                name: name.into(),
                field_type,
                required: true,
            })
            .collect(),
            runtime_values: crate::interpret::RuntimeValueCode::ALL
                .into_iter()
                .map(|code| RuntimeValueDescription {
                    name: code.machine_name().into(),
                    payload: match code {
                        crate::interpret::RuntimeValueCode::Unit => RuntimeValuePayload::None,
                        crate::interpret::RuntimeValueCode::Bool => RuntimeValuePayload::Bool,
                        crate::interpret::RuntimeValueCode::I64 => RuntimeValuePayload::I64,
                        crate::interpret::RuntimeValueCode::Bytes => RuntimeValuePayload::Bytes,
                        crate::interpret::RuntimeValueCode::Product => RuntimeValuePayload::Product,
                        crate::interpret::RuntimeValueCode::Sum => RuntimeValuePayload::Sum,
                    },
                    fields: match code {
                        crate::interpret::RuntimeValueCode::Unit => vec![],
                        crate::interpret::RuntimeValueCode::Bool => vec![MachineFieldDescription { name: "data".into(), type_expression: "bool".into(), required: true }],
                        crate::interpret::RuntimeValueCode::I64 => vec![MachineFieldDescription { name: "data".into(), type_expression: "i64".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Bytes => vec![MachineFieldDescription { name: "data".into(), type_expression: "bytes_string".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Product => vec![MachineFieldDescription { name: "data".into(), type_expression: "runtime_product_data".into(), required: true }],
                        crate::interpret::RuntimeValueCode::Sum => vec![MachineFieldDescription { name: "data".into(), type_expression: "runtime_sum_data".into(), required: true }],
                    },
                    invariants: match code {
                        crate::interpret::RuntimeValueCode::Product => vec![
                            "ty is a semantic product declaration Node ID; fields name every exact owned field identity once".into(),
                            "input field order is arbitrary and normalized; output field order is canonical declaration order".into(),
                            "each field value has the field's exact semantic type; compiler indexes and layout offsets are forbidden".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Sum => vec![
                            "ty is a semantic sum declaration Node ID and variant is one exact owned semantic variant Node ID".into(),
                            "payload is absent for nullary variants and present with the exact payload type otherwise".into(),
                            "compiler discriminants and dense type or variant indexes are forbidden".into(),
                        ],
                        crate::interpret::RuntimeValueCode::Bytes => vec![
                            "equality and behavior depend only on visible ordered octets; backing, view, sharing, and runtime handles are unobservable".into(),
                            "data is canonical unpadded URL-safe base64 with no whitespace and canonical trailing bits".into(),
                        ],
                        _ => vec!["value must have the exact primitive semantic type".into()],
                    },
                })
                .collect(),
            records: run_records(),
            variants: run_variants(),
            limit_scope: vec![
                "argument count applies to the complete Run arguments list".into(),
                "runtime value depth applies per nested value root; item and structural-byte limits aggregate across all Run arguments".into(),
                "live-cell policy applies to peak frame arrays plus argument, edge, return, and public flatten scratch before allocation or cell transfer".into(),
                "fuel charges before work: one base per instruction or transfer plus max(1, materialized cells) for every logical value transfer; variant construction charges its full canonical sum cells".into(),
                "bytes_slice additionally charges one logical view unit without charging per visible octet".into(),
                "bytes_equal additionally charges one fuel unit per compared octet and stops at the first mismatch; differing lengths compare no octets".into(),
                "decoded byte values, invocation visible bytes, distinct retained backing bytes, and managed object count have independent limits".into(),
            ],
        },
        queries: QueryCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        query_payloads: QueryCode::ALL.into_iter().map(query_payload).collect(),
        query_result_payloads: QueryCode::ALL
            .into_iter()
            .map(query_result_payload)
            .collect(),
        query_records: query_records(),
        query_variants: query_variants(),
        query_member_payloads: query_member_payloads(),
        query_cursor_payloads: query_cursor_payloads(),
        errors: crate::ErrorCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        error_payload: error_payload(),
        error_records: error_records(),
        error_variants: error_variants(),
        requests: RequestCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        request_payloads: RequestCode::ALL.into_iter().map(request_payload).collect(),
        responses: ResponseCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name()))
            .collect(),
        response_payloads: ResponseCode::ALL
            .into_iter()
            .map(response_payload)
            .collect(),
        identity_variants: identity_variants(),
        envelopes: envelope_payloads(),
        boundary_error_kinds: [
            BoundaryErrorKind::InvalidJson,
            BoundaryErrorKind::InputTooLarge,
            BoundaryErrorKind::Transport,
            BoundaryErrorKind::Output,
            BoundaryErrorKind::Usage,
        ]
        .into_iter()
        .map(|kind| kind.machine_name().into())
        .collect(),
        limits: BoundaryLimits {
            maximum_request_frame_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_response_frame_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_artifact_bytes: crate::artifact::MAXIMUM_ARTIFACT_BYTES as u64,
            maximum_artifact_name_bytes: crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64,
            maximum_json_input_bytes: MAX_JSON_INPUT_BYTES as u64,
            maximum_json_output_bytes: MAX_JSON_OUTPUT_BYTES as u64,
            maximum_page_items: MAX_PAGE_ITEMS,
            maximum_batch_queries: MAX_BATCH_QUERIES as u32,
            maximum_batch_items: MAX_BATCH_ITEMS,
            maximum_context_items_per_category: MAX_CONTEXT_ITEMS,
            maximum_returned_bindings: MAX_RETURNED_BINDINGS as u32,
            maximum_run_arguments: crate::interpret::MAX_RUN_ARGUMENTS as u32,
            maximum_run_fuel: crate::interpret::MAX_RUN_FUEL,
            maximum_run_frames: crate::interpret::MAX_RUN_FRAMES,
            maximum_run_live_cells: crate::interpret::MAX_RUN_LIVE_CELLS as u64,
            maximum_runtime_value_depth: crate::interpret::MAX_RUNTIME_VALUE_DEPTH as u32,
            maximum_runtime_value_items: crate::interpret::MAX_RUNTIME_VALUE_ITEMS as u64,
            maximum_runtime_value_bytes: crate::interpret::MAX_RUNTIME_VALUE_BYTES as u64,
            maximum_byte_literal_bytes: crate::schema::MAXIMUM_BYTE_LITERAL_BYTES as u64,
            maximum_transaction_byte_literal_bytes: crate::schema::MAXIMUM_TRANSACTION_BYTE_LITERAL_BYTES as u64,
            maximum_runtime_byte_value_bytes: crate::schema::MAXIMUM_BYTE_STRING_BYTES as u64,
            maximum_run_argument_byte_bytes: crate::interpret::MAX_RUN_ARGUMENT_BYTE_BYTES as u64,
            maximum_run_managed_visible_bytes: crate::interpret::MAX_RUN_MANAGED_VISIBLE_BYTES as u64,
            maximum_run_retained_backing_bytes: crate::interpret::MAX_RUN_RETAINED_BACKING_BYTES as u64,
            maximum_run_managed_objects: crate::interpret::MAX_RUN_MANAGED_OBJECTS as u64,
            maximum_error_related_ids: crate::error::MAX_ERROR_RELATED_IDS as u32,
            maximum_boundary_error_message_bytes: MAX_BOUNDARY_ERROR_MESSAGE_BYTES as u64,
            maximum_persistence_head_bytes: crate::persistence::MAXIMUM_HEAD_BYTES as u64,
        },
        id_formats: IdFormats {
            workspace: "32 lowercase hexadecimal characters".to_owned(),
            idempotency_key: "32 lowercase hexadecimal characters".to_owned(),
            node: "WORKSPACE:nonzero canonical decimal serial".to_owned(),
            snapshot_hash: "64 lowercase hexadecimal characters".to_owned(),
            change_digest: "64 lowercase hexadecimal characters".to_owned(),
            revision: "JSON unsigned 64-bit integer".to_owned(),
            request_id: "JSON nonzero unsigned 64-bit integer".to_owned(),
            query_id: "JSON unsigned 64-bit integer".to_owned(),
            draft_symbol: "1 to 64 ASCII bytes matching [a-z][a-z0-9_]*".to_owned(),
            machine_schema_digest: "64 lowercase hexadecimal characters".to_owned(),
        },
        nominal_declarations: NominalDeclarationsDescription {
            declaration_kinds: vec![NodeKind::ProductType, NodeKind::SumType],
            member_kinds: vec![NodeKind::ProductField, NodeKind::SumVariant],
            shape_invariants: vec![
                "nominal type identity is its declaration Node ID; member identity is its field or variant Node ID".into(),
                "one declaration identity has immutable owner, ordered member identities, ordinals, field types, and variant payload contracts".into(),
                "product construction names every exact owned field once; sum construction names one exact owned variant and its exact optional payload".into(),
                "closed-sum match has exactly one identity-keyed arm per variant and canonical storage follows declaration order".into(),
                "direct and indirect by-value nominal cycles reject atomically".into(),
            ],
            layout_invariants: vec![
                "layout is deterministic derived state and is absent from semantic artifacts".into(),
                "product fields use declaration order with checked alignment; sum discriminants use variant ordinals".into(),
                "runtime aggregate accounting uses materialized cells rather than one scalar per aggregate".into(),
            ],
        },
    }
}

fn canonicalize_schema(mut schema: SchemaDescription) -> SchemaDescription {
    schema
        .scalar_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .node_kinds
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .name_contract
        .named_node_kinds
        .sort_by_key(|item| item.machine_name());
    schema
        .name_contract
        .sibling_uniqueness_groups
        .sort_by(|left, right| left.name.cmp(&right.name));
    for group in &mut schema.name_contract.sibling_uniqueness_groups {
        group.member_kinds.sort_by_key(|item| item.machine_name());
    }
    schema
        .operations
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.semantic_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .transaction_operations
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_operation_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.transaction_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .structured_authoring
        .draft_field_types
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .expression_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .operation_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .value_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .type_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .implicit_node_kinds
        .sort_by_key(|item| item.machine_name());
    schema.structured_authoring.counted_item_categories.sort();
    schema
        .structured_authoring
        .inline_expression_variants
        .sort();
    schema
        .run
        .runtime_values
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .run
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .run
        .variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.run.variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .queries
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_result_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.query_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .query_member_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_cursor_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .errors
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .error_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .error_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.error_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .requests
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .request_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .responses
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .response_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .identity_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.identity_variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .envelopes
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema.boundary_error_kinds.sort();
    schema
        .schema_discovery
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .schema_discovery
        .variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.schema_discovery.variants {
        variant
            .variants
            .sort_by(|left, right| left.name.cmp(&right.name));
    }
    schema
        .schema_discovery
        .projection_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .schema_discovery
        .result_payloads
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema.schema_discovery.roots.sort();
    schema.schema_discovery.type_constructors.sort();
    schema
        .nominal_declarations
        .declaration_kinds
        .sort_by_key(|item| item.machine_name());
    schema
        .nominal_declarations
        .member_kinds
        .sort_by_key(|item| item.machine_name());
    schema
}

pub fn machine_schema_digest(
    description: &SchemaDescription,
) -> crate::Result<MachineSchemaDigest> {
    let canonical = canonicalize_schema(description.clone());
    let catalogue = schema_definition_catalogue(&canonical).map_err(|error| {
        crate::LkError::new(
            crate::ErrorCode::ProtocolMalformed,
            format!("cannot derive machine schema digest catalogue: {error}"),
        )
    })?;
    let bytes = serde_json::to_vec(&(canonical, catalogue)).map_err(|error| {
        crate::LkError::new(
            crate::ErrorCode::ProtocolMalformed,
            format!("cannot encode machine schema digest input: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(MACHINE_SCHEMA_DIGEST_DOMAIN);
    hasher.update(&bytes);
    Ok(MachineSchemaDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

pub fn active_machine_schema_digest() -> crate::Result<MachineSchemaDigest> {
    machine_schema_digest(&schema_description())
}

pub fn describe_schema(request: &DescribeSchemaRequest) -> Result<DescribeSchemaResult, String> {
    request.validate().map_err(str::to_owned)?;
    let description = schema_description();
    let catalogue = schema_definition_catalogue(&description)?;
    let projected = match &request.projection {
        SchemaProjection::Roots { roots } => Some(project_schema_roots(&catalogue, roots)?),
        SchemaProjection::Manifest | SchemaProjection::Full => None,
    };
    let digest = machine_schema_digest(&description).map_err(|error| error.to_string())?;
    if request.known_digest == Some(digest) {
        return Ok(DescribeSchemaResult::Unchanged { digest });
    }
    match &request.projection {
        SchemaProjection::Manifest => Ok(DescribeSchemaResult::Manifest(schema_manifest(
            &description,
            digest,
        ))),
        SchemaProjection::Roots { .. } => {
            let Some((roots, definitions)) = projected else {
                return Err("schema root projection was not preflighted".to_owned());
            };
            Ok(DescribeSchemaResult::Roots(SchemaDefinitions {
                digest,
                roots,
                type_constructors: schema_type_constructors(),
                definitions,
            }))
        }
        SchemaProjection::Full => Ok(DescribeSchemaResult::Full {
            digest,
            description: Box::new(description),
        }),
    }
}

fn schema_manifest(description: &SchemaDescription, digest: MachineSchemaDigest) -> SchemaManifest {
    SchemaManifest {
        schema_identity: description.machine_schema_identity.clone(),
        digest,
        protocol_version: description.protocol_version,
        json_envelope_version: description.json_envelope_version,
        artifact_format_version: description.artifact_format_version,
        artifact_magic_hex: description.artifact_magic_hex.clone(),
        semantic_schema_identity: description.semantic_schema_identity.clone(),
        roots: SchemaRoot::ALL
            .into_iter()
            .map(|root| root.machine_name().to_owned())
            .collect(),
        type_constructors: schema_type_constructors(),
        maximum_roots_per_request: MAX_SCHEMA_ROOTS as u8,
        full_available: true,
        maximum_request_frame_bytes: description.limits.maximum_request_frame_bytes,
        maximum_response_frame_bytes: description.limits.maximum_response_frame_bytes,
        maximum_json_output_bytes: description.limits.maximum_json_output_bytes,
    }
}

fn project_schema_roots(
    catalogue: &BTreeMap<String, SchemaDefinition>,
    roots: &[SchemaRoot],
) -> Result<(Vec<SchemaRoot>, Vec<SchemaDefinition>), String> {
    let mut canonical_roots = roots.to_vec();
    canonical_roots.sort_unstable();
    let mut pending = VecDeque::new();
    for root in &canonical_roots {
        pending.push_back(root.machine_name().to_owned());
    }
    let mut selected = BTreeSet::new();
    while let Some(name) = pending.pop_front() {
        if !selected.insert(name.clone()) {
            continue;
        }
        let definition = catalogue
            .get(&name)
            .ok_or_else(|| format!("unknown schema root or dependency: {name}"))?;
        for dependency in &definition.dependencies {
            if !selected.contains(dependency) {
                pending.push_back(dependency.clone());
            }
        }
    }
    let definitions = selected
        .into_iter()
        .map(|name| {
            catalogue
                .get(&name)
                .cloned()
                .ok_or_else(|| format!("missing selected schema definition: {name}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok((canonical_roots, definitions))
}

fn lookup_named_record<'a>(
    records: &'a [NamedPayloadDescription],
    name: &str,
) -> Result<&'a NamedPayloadDescription, String> {
    records
        .iter()
        .find(|record| record.name == name)
        .ok_or_else(|| format!("missing executable record descriptor: {name}"))
}

fn named_variant_family<'a>(
    variants: &'a [NamedVariantDescription],
    name: &str,
) -> Result<&'a NamedVariantDescription, String> {
    variants
        .iter()
        .find(|variant| variant.name == name)
        .ok_or_else(|| format!("missing executable variant descriptor: {name}"))
}

fn variant_payload_by_name(
    variants: &[VariantPayloadDescription],
    name: &str,
) -> Result<VariantPayloadDescription, String> {
    variants
        .iter()
        .find(|variant| variant.name == name)
        .cloned()
        .ok_or_else(|| format!("missing executable variant payload: {name}"))
}

fn projected_variant_family(
    variants: &[NamedVariantDescription],
    name: &str,
    retained_variants: &[&str],
) -> Result<NamedVariantDescription, String> {
    let source = named_variant_family(variants, name)?;
    let mut selected = Vec::with_capacity(retained_variants.len());
    for retained in retained_variants {
        selected.push(
            source
                .variants
                .iter()
                .find(|variant| variant.name == *retained)
                .cloned()
                .ok_or_else(|| format!("missing executable {name} variant: {retained}"))?,
        );
    }
    Ok(NamedVariantDescription {
        name: source.name.clone(),
        tagging: source.tagging.clone(),
        tag_field: source.tag_field.clone(),
        content_field: source.content_field.clone(),
        variants: selected,
    })
}

fn endpoint_protocol_templates(
    description: &SchemaDescription,
) -> Result<Vec<EndpointProtocolTemplateDescription>, String> {
    let records = |sources: &[(&[NamedPayloadDescription], &str)]| {
        sources
            .iter()
            .map(|(records, name)| lookup_named_record(records, name).cloned())
            .collect::<Result<Vec<_>, _>>()
    };
    let parameter =
        |name: &str, target_variant: &str, semantics: &str| EndpointTemplateParameterDescription {
            name: name.to_owned(),
            target_variant: target_variant.to_owned(),
            semantics: semantics.to_owned(),
        };

    let control = EndpointProtocolTemplateDescription {
        name: "control_endpoint_protocol".to_owned(),
        parameters: vec![
            parameter(
                "request_variant",
                "request",
                "the endpoint binding supplies exactly one top-level request variant and its leaf payload",
            ),
            parameter(
                "success_response_variant",
                "response",
                "the endpoint binding supplies exactly one successful top-level response variant and its leaf payload",
            ),
        ],
        records: records(&[
            (&description.envelopes, "request_envelope"),
            (&description.envelopes, "response_envelope"),
        ])?,
        variants: vec![
            projected_variant_family(&description.identity_variants, "request", &[])?,
            projected_variant_family(
                &description.identity_variants,
                "response",
                &[ResponseCode::Error.machine_name()],
            )?,
        ],
    };
    let query = EndpointProtocolTemplateDescription {
        name: "query_endpoint_protocol".to_owned(),
        parameters: vec![
            parameter(
                "query_variant",
                "query",
                "the endpoint binding supplies exactly one selected inner query variant and its leaf payload",
            ),
            parameter(
                "query_result_variant",
                "query_result",
                "the endpoint binding supplies the matching inner success-result variant and its leaf payload",
            ),
        ],
        records: records(&[
            (&description.envelopes, "request_envelope"),
            (&description.envelopes, "response_envelope"),
            (&description.query_records, "query_batch_request"),
            (&description.query_records, "query_item"),
            (&description.query_records, "query_batch_result"),
            (&description.query_records, "query_item_result"),
        ])?,
        variants: vec![
            projected_variant_family(
                &description.identity_variants,
                "request",
                &[RequestCode::QueryBatch.machine_name()],
            )?,
            projected_variant_family(
                &description.identity_variants,
                "response",
                &[
                    ResponseCode::QueryBatchResult.machine_name(),
                    ResponseCode::Error.machine_name(),
                ],
            )?,
            projected_variant_family(&description.query_variants, "query", &[])?,
            projected_variant_family(&description.query_variants, "query_result", &[])?,
            projected_variant_family(
                &description.query_variants,
                "query_outcome",
                &["success", "error"],
            )?,
        ],
    };
    Ok(vec![control, query])
}

fn endpoint_definition(
    description: &SchemaDescription,
    endpoint_name: &str,
    family: &str,
    template: &str,
    bindings: Vec<EndpointVariantBindingDescription>,
) -> (String, SchemaDefinitionBody) {
    (
        endpoint_name.to_owned(),
        SchemaDefinitionBody::Endpoint(EndpointDescription {
            name: endpoint_name.to_owned(),
            family: family.to_owned(),
            template: template.to_owned(),
            bindings,
            protocol_version: description.protocol_version,
            json_envelope_version: description.json_envelope_version,
            boundary_error_envelope: "boundary_error_envelope".to_owned(),
            typed_error: "error".to_owned(),
            id_formats: "id_formats".to_owned(),
            limits: "limits".to_owned(),
        }),
    )
}

fn endpoint_binding(
    parameter: &str,
    variant: VariantPayloadDescription,
) -> EndpointVariantBindingDescription {
    EndpointVariantBindingDescription {
        parameter: parameter.to_owned(),
        variant,
    }
}

fn schema_definition_catalogue(
    description: &SchemaDescription,
) -> Result<BTreeMap<String, SchemaDefinition>, String> {
    let mut bodies = BTreeMap::<String, SchemaDefinitionBody>::new();
    let mut insert = |name: String, body: SchemaDefinitionBody| -> Result<(), String> {
        if bodies.insert(name.clone(), body).is_some() {
            return Err(format!("duplicate machine schema definition: {name}"));
        }
        Ok(())
    };

    for scalar in &description.scalar_types {
        insert(
            scalar.name.clone(),
            SchemaDefinitionBody::Scalar(scalar.clone()),
        )?;
    }
    for record in description
        .semantic_records
        .iter()
        .chain(description.transaction_records.iter())
        .chain(description.query_records.iter())
        .chain(description.run.records.iter())
        .chain(description.error_records.iter())
        .chain(description.envelopes.iter())
        .chain(description.schema_discovery.records.iter())
    {
        insert(
            record.name.clone(),
            SchemaDefinitionBody::Record(record.clone()),
        )?;
    }
    insert(
        "describe_schema_request".to_owned(),
        SchemaDefinitionBody::Record(NamedPayloadDescription {
            name: "describe_schema_request".to_owned(),
            payload: description.schema_discovery.request.clone(),
        }),
    )?;
    insert(
        "error".to_owned(),
        SchemaDefinitionBody::Record(NamedPayloadDescription {
            name: "error".to_owned(),
            payload: description.error_payload.clone(),
        }),
    )?;
    for variant in description
        .semantic_variants
        .iter()
        .chain(description.transaction_variants.iter())
        .chain(description.query_variants.iter())
        .chain(description.run.variants.iter())
        .chain(description.error_variants.iter())
        .chain(description.identity_variants.iter())
        .chain(description.schema_discovery.variants.iter())
    {
        insert(
            variant.name.clone(),
            SchemaDefinitionBody::Variant(variant.clone()),
        )?;
    }
    for template in endpoint_protocol_templates(description)? {
        insert(
            template.name.clone(),
            SchemaDefinitionBody::EndpointTemplate(template),
        )?;
    }
    for (request_code, response_code) in [
        (RequestCode::CreateWorkspace, ResponseCode::WorkspaceCreated),
        (
            RequestCode::ApplyTransaction,
            ResponseCode::TransactionReceipt,
        ),
        (RequestCode::Run, ResponseCode::Run),
        (RequestCode::Shutdown, ResponseCode::Acknowledged),
        (RequestCode::DescribeSchema, ResponseCode::DescribeSchema),
    ] {
        let (name, body) = endpoint_definition(
            description,
            request_code.machine_name(),
            "control",
            "control_endpoint_protocol",
            vec![
                endpoint_binding(
                    "request_variant",
                    variant_payload_by_name(
                        &description.request_payloads,
                        request_code.machine_name(),
                    )?,
                ),
                endpoint_binding(
                    "success_response_variant",
                    variant_payload_by_name(
                        &description.response_payloads,
                        response_code.machine_name(),
                    )?,
                ),
            ],
        );
        insert(name, body)?;
    }
    for query_code in QueryCode::ALL {
        let endpoint_name = format!("query_{}", query_code.machine_name());
        let (name, body) = endpoint_definition(
            description,
            &endpoint_name,
            "query",
            "query_endpoint_protocol",
            vec![
                endpoint_binding(
                    "query_variant",
                    variant_payload_by_name(
                        &description.query_payloads,
                        query_code.machine_name(),
                    )?,
                ),
                endpoint_binding(
                    "query_result_variant",
                    variant_payload_by_name(
                        &description.query_result_payloads,
                        query_code.machine_name(),
                    )?,
                ),
            ],
        );
        insert(name, body)?;
    }
    for record in &description.structured_authoring.records {
        insert(
            record.name.clone(),
            SchemaDefinitionBody::DraftRecord(record.clone()),
        )?;
    }
    for family in [
        DraftVariantFamilyDescription {
            name: "expression_kind_draft".to_owned(),
            tagging: description.structured_authoring.expression_tagging.clone(),
            variants: description.structured_authoring.expression_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "operation_draft".to_owned(),
            tagging: description.structured_authoring.operation_tagging.clone(),
            variants: description.structured_authoring.operation_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "value_draft".to_owned(),
            tagging: description.structured_authoring.value_tagging.clone(),
            variants: description.structured_authoring.value_variants.clone(),
        },
        DraftVariantFamilyDescription {
            name: "type_draft".to_owned(),
            tagging: description.structured_authoring.type_tagging.clone(),
            variants: description.structured_authoring.type_variants.clone(),
        },
    ] {
        insert(
            family.name.clone(),
            SchemaDefinitionBody::DraftVariant(family),
        )?;
    }

    let code_family = |name: &str, codes: &[CodeDescription]| CodeFamilyDescription {
        name: name.to_owned(),
        members: codes.iter().map(|code| code.name.clone()).collect(),
    };
    for family in [
        code_family("node_kind", &description.node_kinds),
        CodeFamilyDescription {
            name: "operation_code".to_owned(),
            members: description
                .operations
                .iter()
                .map(|operation| operation.name.clone())
                .collect(),
        },
        code_family(
            "transaction_operation_code",
            &description.transaction_operations,
        ),
        code_family("error_code", &description.errors),
    ] {
        insert(family.name.clone(), SchemaDefinitionBody::Codes(family))?;
    }
    insert(
        "operations".to_owned(),
        SchemaDefinitionBody::Operations(description.operations.clone()),
    )?;
    insert(
        "structured_authoring".to_owned(),
        SchemaDefinitionBody::StructuredAuthoring(StructuredAuthoringPolicyDescription {
            allocation_order: description.structured_authoring.allocation_order.clone(),
            inline_expression_variants: description
                .structured_authoring
                .inline_expression_variants
                .clone(),
            inline_holes_allowed: description.structured_authoring.inline_holes_allowed,
            inline_region_operations_allowed: description
                .structured_authoring
                .inline_region_operations_allowed,
            maintenance_accepts_inline_values: description
                .structured_authoring
                .maintenance_accepts_inline_values,
            nesting_metric: description.structured_authoring.nesting_metric.clone(),
            explicit_symbols_are_selectable: description
                .structured_authoring
                .explicit_symbols_are_selectable,
            implicit_symbols_are_selectable: description
                .structured_authoring
                .implicit_symbols_are_selectable,
            implicit_node_kinds: description.structured_authoring.implicit_node_kinds.clone(),
            maximum_request_depth: description.structured_authoring.maximum_request_depth,
            maximum_request_items: description.structured_authoring.maximum_request_items,
            counted_item_categories: description
                .structured_authoring
                .counted_item_categories
                .clone(),
        }),
    )?;
    insert(
        "name_contract".to_owned(),
        SchemaDefinitionBody::NameContract(description.name_contract.clone()),
    )?;
    insert(
        "nominal_declarations".to_owned(),
        SchemaDefinitionBody::NominalDeclarations(description.nominal_declarations.clone()),
    )?;
    insert(
        "id_formats".to_owned(),
        SchemaDefinitionBody::IdFormats(description.id_formats.clone()),
    )?;
    insert(
        "limits".to_owned(),
        SchemaDefinitionBody::Limits(description.limits.clone()),
    )?;

    for (name, body) in &bodies {
        let SchemaDefinitionBody::Endpoint(endpoint) = body else {
            continue;
        };
        let Some(SchemaDefinitionBody::EndpointTemplate(template)) = bodies.get(&endpoint.template)
        else {
            return Err(format!(
                "endpoint {name} references unknown template {}",
                endpoint.template
            ));
        };
        let expected = template
            .parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<BTreeSet<_>>();
        let actual = endpoint
            .bindings
            .iter()
            .map(|binding| binding.parameter.as_str())
            .collect::<BTreeSet<_>>();
        if actual.len() != endpoint.bindings.len() || actual != expected {
            return Err(format!(
                "endpoint {name} must bind every template parameter exactly once"
            ));
        }
    }

    let names = bodies.keys().cloned().collect::<BTreeSet<_>>();
    let mut catalogue = BTreeMap::new();
    for (name, body) in bodies {
        let dependencies = definition_dependencies(&body)?;
        for dependency in &dependencies {
            if !names.contains(dependency) {
                return Err(format!(
                    "machine schema definition {name} references unknown definition {dependency}"
                ));
            }
        }
        catalogue.insert(
            name.clone(),
            SchemaDefinition {
                name,
                dependencies: dependencies.into_iter().collect(),
                body,
            },
        );
    }
    for root in SchemaRoot::ALL {
        if !catalogue.contains_key(root.machine_name()) {
            return Err(format!(
                "machine schema root {} has no definition",
                root.machine_name()
            ));
        }
    }
    Ok(catalogue)
}

fn definition_dependencies(body: &SchemaDefinitionBody) -> Result<BTreeSet<String>, String> {
    let mut dependencies = BTreeSet::new();
    match body {
        SchemaDefinitionBody::Scalar(_)
        | SchemaDefinitionBody::Codes(_)
        | SchemaDefinitionBody::IdFormats(_)
        | SchemaDefinitionBody::Limits(_) => {}
        SchemaDefinitionBody::Record(record) => {
            payload_dependencies(&record.payload, &mut dependencies)?;
        }
        SchemaDefinitionBody::Variant(variant) => {
            for payload in &variant.variants {
                payload_dependencies(&payload.payload, &mut dependencies)?;
            }
        }
        SchemaDefinitionBody::DraftRecord(record) => {
            for field in &record.fields {
                dependencies.extend(type_expression_dependencies(
                    field.field_type.type_expression(),
                )?);
            }
        }
        SchemaDefinitionBody::Endpoint(endpoint) => {
            dependencies.extend(
                [
                    &endpoint.template,
                    &endpoint.boundary_error_envelope,
                    &endpoint.typed_error,
                    &endpoint.id_formats,
                    &endpoint.limits,
                ]
                .into_iter()
                .cloned(),
            );
            for binding in &endpoint.bindings {
                payload_dependencies(&binding.variant.payload, &mut dependencies)?;
            }
        }
        SchemaDefinitionBody::EndpointTemplate(template) => {
            let mut local_names = BTreeSet::new();
            for name in template
                .records
                .iter()
                .map(|record| &record.name)
                .chain(template.variants.iter().map(|variant| &variant.name))
            {
                if !local_names.insert(name.clone()) {
                    return Err(format!(
                        "duplicate endpoint template local definition: {name}"
                    ));
                }
            }
            let mut parameter_names = BTreeSet::new();
            for parameter in &template.parameters {
                if !parameter_names.insert(parameter.name.clone()) {
                    return Err(format!(
                        "duplicate endpoint template parameter: {}",
                        parameter.name
                    ));
                }
                if !template
                    .variants
                    .iter()
                    .any(|variant| variant.name == parameter.target_variant)
                {
                    return Err(format!(
                        "endpoint template parameter {} targets unknown local variant {}",
                        parameter.name, parameter.target_variant
                    ));
                }
            }
            for record in &template.records {
                payload_dependencies(&record.payload, &mut dependencies)?;
            }
            for variant in &template.variants {
                for payload in &variant.variants {
                    payload_dependencies(&payload.payload, &mut dependencies)?;
                }
            }
            dependencies.retain(|dependency| !local_names.contains(dependency));
        }
        SchemaDefinitionBody::DraftVariant(family) => {
            for variant in &family.variants {
                if let Some(field_type) = variant.newtype {
                    dependencies
                        .extend(type_expression_dependencies(field_type.type_expression())?);
                }
                for field in &variant.fields {
                    dependencies.extend(type_expression_dependencies(
                        field.field_type.type_expression(),
                    )?);
                }
            }
            dependencies.insert("structured_authoring".to_owned());
        }
        SchemaDefinitionBody::Operations(_) => {
            dependencies.extend(
                [
                    "operation_code",
                    "operand_arity",
                    "operand_use",
                    "literal_field",
                    "region_arity",
                    "region_role",
                    "block_argument_role",
                    "type_rule",
                ]
                .into_iter()
                .map(str::to_owned),
            );
        }
        SchemaDefinitionBody::StructuredAuthoring(_) => {
            dependencies.insert("node_kind".to_owned());
        }
        SchemaDefinitionBody::NameContract(_) | SchemaDefinitionBody::NominalDeclarations(_) => {
            dependencies.insert("node_kind".to_owned());
        }
    }
    Ok(dependencies)
}

fn payload_dependencies(
    payload: &PayloadShapeDescription,
    dependencies: &mut BTreeSet<String>,
) -> Result<(), String> {
    if let Some(newtype) = &payload.newtype {
        dependencies.extend(type_expression_dependencies(newtype)?);
    }
    for field in &payload.fields {
        dependencies.extend(type_expression_dependencies(&field.type_expression)?);
    }
    Ok(())
}

fn type_expression_dependencies(expression: &str) -> Result<BTreeSet<String>, String> {
    #[derive(Clone, Copy)]
    struct Frame {
        constructor: &'static str,
        items: usize,
    }
    fn constructor(name: &str) -> Option<&'static str> {
        match name {
            "list" => Some("list"),
            "optional" => Some("optional"),
            "tuple" => Some("tuple"),
            "page" => Some("page"),
            _ => None,
        }
    }

    let bytes = expression.as_bytes();
    if bytes.is_empty() || !bytes.is_ascii() {
        return Err(format!("invalid machine type expression: {expression}"));
    }
    let mut dependencies = BTreeSet::new();
    let mut stack = Vec::<Frame>::new();
    let mut index = 0;
    let mut expect_type = true;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_lowercase() {
            if !expect_type {
                return Err(format!("invalid machine type expression: {expression}"));
            }
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_lowercase()
                    || bytes[index].is_ascii_digit()
                    || bytes[index] == b'_')
            {
                index += 1;
            }
            let name = &expression[start..index];
            if let Some(constructor) = constructor(name) {
                if index >= bytes.len() || bytes[index] != b'<' {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                if constructor == "page" {
                    dependencies.insert("page".to_owned());
                }
                stack.push(Frame {
                    constructor,
                    items: 0,
                });
                index += 1;
                expect_type = true;
            } else {
                if name != "type_parameter" {
                    dependencies.insert(name.to_owned());
                }
                expect_type = false;
            }
            continue;
        }
        match byte {
            b',' if !expect_type && !stack.is_empty() => {
                let Some(frame) = stack.last_mut() else {
                    return Err(format!("invalid machine type expression: {expression}"));
                };
                frame.items += 1;
                if frame.constructor != "tuple" {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                expect_type = true;
                index += 1;
            }
            b'>' if !expect_type && !stack.is_empty() => {
                let Some(mut frame) = stack.pop() else {
                    return Err(format!("invalid machine type expression: {expression}"));
                };
                frame.items += 1;
                if frame.constructor != "tuple" && frame.items != 1 {
                    return Err(format!("invalid machine type expression: {expression}"));
                }
                expect_type = false;
                index += 1;
            }
            _ => return Err(format!("invalid machine type expression: {expression}")),
        }
    }
    if expect_type || !stack.is_empty() {
        return Err(format!("invalid machine type expression: {expression}"));
    }
    Ok(dependencies)
}

fn described(name: &str) -> CodeDescription {
    CodeDescription {
        name: name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{
        ContextBudget, PageRequest, Query, QueryBatchRequest, QueryBatchResult, QueryItem,
        QueryItemResult, QueryOutcome, QueryResult, VisibleCursorPurpose, WorkspaceSummary,
    };
    use crate::schema::{
        BlockArgumentRole, LiteralField, OperandArity, OperandUse, RegionArity, RegionRole,
        TypeRule,
    };
    use crate::transaction::{
        ExpressionDraft, ExpressionKindDraft, Transaction, TransactionMode, TransactionOp,
        TransactionReceipt, TransactionResponseSpec, YieldingBodyDraft,
    };
    use crate::{
        ApplyTransactionRequest, DraftSymbol, ErrorCode, NodeId, NodeTarget, QueryId, Revision,
        ValueDraft, WorkspaceId,
    };

    #[test]
    fn strict_envelope_and_canonical_id_rejections() {
        let workspace = WorkspaceId::from_bytes([0xab; 16]);
        let request = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(1),
            request: Request::QueryBatch(QueryBatchRequest {
                workspace,
                revision: Revision::INITIAL,
                queries: vec![QueryItem {
                    id: QueryId::new(1),
                    query: Query::Blockers {
                        page: PageRequest {
                            after: None,
                            limit: 1,
                        },
                    },
                }],
            }),
        };
        let bytes = serde_json::to_vec(&request).expect("request JSON");
        assert_eq!(decode_request(&bytes).expect("decode"), request);
        let text = String::from_utf8(bytes).expect("UTF-8");
        for invalid in [
            text.replacen("\"version\":7", "\"version\":6", 1),
            text.replacen("\"request_id\":1", "\"request_id\":0", 1),
            text.replacen("{\"version\":7", "{\"unknown\":0,\"version\":7", 1),
            format!("{text} {{}}"),
            text.replacen(
                &workspace.to_string(),
                &workspace.to_string().to_uppercase(),
                1,
            ),
        ] {
            assert!(decode_request(invalid.as_bytes()).is_err(), "{invalid}");
        }
    }

    #[test]
    fn scalar_contracts_enforce_exact_id_and_integer_domains() {
        let schema = schema_description();
        let workspace = crate::WorkspaceId::from_bytes([0x42; 16]);
        let node = crate::NodeId::new(workspace, 1).expect("node").to_string();
        for (expression, value) in [
            ("node_id", serde_json::json!(node)),
            ("u8", serde_json::json!(u8::MAX)),
            ("u32", serde_json::json!(u32::MAX)),
            ("u64", serde_json::json!(u64::MAX)),
            ("i64", serde_json::json!(i64::MIN)),
            ("i64", serde_json::json!(i64::MAX)),
        ] {
            validate_exact_type_expression(&schema, expression, &value, None)
                .unwrap_or_else(|error| panic!("{expression} witness: {error}"));
        }
        for (expression, value) in [
            ("node_id", serde_json::json!("arbitrary string")),
            ("node_id", serde_json::json!(workspace.to_string())),
            (
                "node_id",
                serde_json::json!(format!(
                    "{}:1",
                    WorkspaceId::from_bytes([0xab; 16])
                        .to_string()
                        .to_uppercase()
                )),
            ),
            ("node_id", serde_json::json!(format!("{workspace}:01"))),
            ("u8", serde_json::json!(256)),
            ("u8", serde_json::json!(-1)),
            ("u32", serde_json::json!(u64::from(u32::MAX) + 1)),
            ("u64", serde_json::json!(-1)),
            ("i64", serde_json::json!(u64::MAX)),
            ("request_id", serde_json::json!(0)),
        ] {
            assert!(
                validate_exact_type_expression(&schema, expression, &value, None).is_err(),
                "{expression} accepted {value}"
            );
        }
        assert_eq!(schema.scalar_types.len(), 18);
        assert_eq!(
            schema
                .scalar_types
                .iter()
                .map(|scalar| scalar.name.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            schema.scalar_types.len()
        );
    }

    #[test]
    fn semantic_transaction_and_query_variant_samples_are_exhaustive() {
        use crate::diff::{ChangeKind, ScalarValue};
        use crate::query::{
            DefinitionSlot, DependencyFact, ExpectedCategory, LiteralValue, QueryOutcome,
            QueryResult, RepairTarget, WorkspaceSummary,
        };
        use crate::schema::{MatchArm, Node, OperationKind, ProductFieldValue, ValueRef};

        let schema = schema_description();
        let workspace = WorkspaceId::from_bytes([0x43; 16]);
        let node = NodeId::new(workspace, 1).expect("node");
        let other = NodeId::new(workspace, 2).expect("other");
        let value = ValueRef::FunctionParameter(node);

        assert_family_samples(
            &schema,
            "semantic_type",
            &[
                SemanticType::Unit,
                SemanticType::Bool,
                SemanticType::I64,
                SemanticType::Bytes,
                SemanticType::Nominal(node),
            ],
        );
        assert_family_samples(
            &schema,
            "value_ref",
            &[
                ValueRef::FunctionParameter(node),
                ValueRef::BlockArgument(node),
                ValueRef::OperationResult {
                    operation: node,
                    output: 0,
                },
            ],
        );
        assert_family_samples(
            &schema,
            "region_role",
            &[
                RegionRole::IfThen,
                RegionRole::IfElse,
                RegionRole::ForBody,
                RegionRole::MatchArm(node),
            ],
        );
        let operations = vec![
            OperationKind::ConstUnit,
            OperationKind::ConstI64(1),
            OperationKind::ConstBool(true),
            OperationKind::AddI64 {
                lhs: value,
                rhs: value,
            },
            OperationKind::LtI64 {
                lhs: value,
                rhs: value,
            },
            OperationKind::Call {
                function: node,
                arguments: vec![value],
            },
            OperationKind::Hole {
                expected: SemanticType::I64,
            },
            OperationKind::If {
                condition: value,
                result: SemanticType::I64,
                then_region: node,
                else_region: other,
            },
            OperationKind::ForI64 {
                start: value,
                end_exclusive: value,
                step: 1,
                initial: value,
                carried: SemanticType::I64,
                body_region: node,
            },
            OperationKind::Return { value },
            OperationKind::Yield { value },
            OperationKind::ConstructProduct {
                product: node,
                fields: vec![ProductFieldValue {
                    field: other,
                    value,
                }],
            },
            OperationKind::ProjectField {
                value,
                field: other,
            },
            OperationKind::ConstructVariant {
                variant: other,
                payload: Some(value),
            },
            OperationKind::MatchSum {
                scrutinee: value,
                result: SemanticType::I64,
                arms: vec![MatchArm {
                    variant: other,
                    region: node,
                }],
            },
            OperationKind::ConstBytes(crate::schema::ByteString::from_slice(b"LKJM").unwrap()),
            OperationKind::BytesLen { value },
            OperationKind::BytesAt {
                value,
                index: value,
            },
            OperationKind::BytesSlice {
                value,
                start: value,
                length: value,
            },
            OperationKind::BytesEqual {
                lhs: value,
                rhs: value,
            },
        ];
        assert_eq!(operations.len(), OperationCode::ALL.len());
        assert_family_samples(&schema, "operation_kind", &operations);

        let nodes = vec![
            Node::WorkspaceRoot {
                packages: vec![node],
            },
            Node::Package {
                owner: node,
                name: "p".into(),
                modules: vec![other],
                entry: Some(other),
            },
            Node::Module {
                owner: node,
                name: "m".into(),
                types: vec![other],
                functions: vec![other],
            },
            Node::ProductType {
                owner: node,
                name: "product".into(),
                fields: vec![other],
            },
            Node::ProductField {
                owner: node,
                ordinal: 0,
                name: "field".into(),
                ty: SemanticType::I64,
            },
            Node::SumType {
                owner: node,
                name: "sum".into(),
                variants: vec![other],
            },
            Node::SumVariant {
                owner: node,
                ordinal: 0,
                name: "variant".into(),
                payload: Some(SemanticType::I64),
            },
            Node::Function {
                owner: node,
                name: "f".into(),
                parameters: vec![other],
                result: SemanticType::I64,
                body: Some(other),
            },
            Node::Parameter {
                owner: node,
                ordinal: 0,
                name: "x".into(),
                ty: SemanticType::I64,
            },
            Node::Region {
                owner: node,
                blocks: vec![other],
            },
            Node::Block {
                owner: node,
                arguments: vec![other],
                operations: vec![other],
                terminator: Some(other),
            },
            Node::BlockArgument {
                owner: node,
                ordinal: 0,
                ty: SemanticType::I64,
            },
            Node::Operation {
                owner: node,
                operation: OperationKind::ConstUnit,
            },
        ];
        assert_eq!(nodes.len(), NodeKind::ALL.len());
        assert_family_samples(&schema, "node", &nodes);

        assert_family_samples(
            &schema,
            "node_target",
            &[
                NodeTarget::Existing(node),
                NodeTarget::Draft(DraftSymbol::generated(1)),
            ],
        );
        assert_family_samples(
            &schema,
            "transaction_mode",
            &[TransactionMode::Commit, TransactionMode::ValidateOnly],
        );
        assert_family_samples(
            &schema,
            "repair_target",
            &[
                RepairTarget::Hole(node),
                RepairTarget::Operand {
                    operation: node,
                    index: 0,
                },
            ],
        );
        assert_family_samples(
            &schema,
            "expected_category",
            &[
                ExpectedCategory::EntryFunction,
                ExpectedCategory::FunctionBody,
                ExpectedCategory::Expression,
            ],
        );
        assert_family_samples(
            &schema,
            "layout_failure",
            &[
                crate::type_layout::LayoutFailure::ByteSizeOverflow,
                crate::type_layout::LayoutFailure::CellCountOverflow,
                crate::type_layout::LayoutFailure::InvalidDependency,
            ],
        );
        let workspace_summary = WorkspaceSummary {
            workspace,
            revision: Revision::INITIAL,
            hash: crate::SnapshotHash::from_bytes([1; 32]),
            root: node,
            node_count: 1,
            complete: true,
            blocker_count: 0,
            entry_count: 0,
        };
        assert_family_samples(
            &schema,
            "query_outcome",
            &[
                QueryOutcome::Success(Box::new(QueryResult::WorkspaceSummary(workspace_summary))),
                QueryOutcome::Error(crate::error::LkError::new(ErrorCode::InvalidQuery, "bad")),
            ],
        );
        assert_family_samples(
            &schema,
            "visible_cursor_purpose",
            &VisibleCursorPurpose::ALL,
        );
        let definition_slots = [
            DefinitionSlot::PackageEntry,
            DefinitionSlot::CallTarget,
            DefinitionSlot::FunctionResultType,
            DefinitionSlot::ParameterType,
            DefinitionSlot::ProductFieldType,
            DefinitionSlot::SumVariantPayloadType,
            DefinitionSlot::BlockArgumentType,
            DefinitionSlot::OperationType,
            DefinitionSlot::ProductDeclaration,
            DefinitionSlot::ProductField,
            DefinitionSlot::SumVariant,
            DefinitionSlot::MatchVariant,
        ];
        assert_family_samples(&schema, "definition_slot", &definition_slots);
        assert_family_samples(
            &schema,
            "block_argument_role",
            &[
                BlockArgumentRole::LoopIndex,
                BlockArgumentRole::LoopCarried,
                BlockArgumentRole::MatchPayload,
            ],
        );
        assert_family_samples(
            &schema,
            "literal_value",
            &[
                LiteralValue::I64(1),
                LiteralValue::Bool(true),
                LiteralValue::ExpectedType(SemanticType::I64),
                LiteralValue::Bytes(crate::schema::ByteString::from_slice(b"x").unwrap()),
            ],
        );
        assert_family_samples(
            &schema,
            "dependency_fact",
            &[
                DependencyFact::ValueOperand { index: 0, value },
                DependencyFact::Definition {
                    slot: DefinitionSlot::CallTarget,
                    target: node,
                },
            ],
        );
        assert_family_samples(
            &schema,
            "scalar_value",
            &[
                ScalarValue::I64(1),
                ScalarValue::Bool(true),
                ScalarValue::Type(SemanticType::I64),
                ScalarValue::Bytes(crate::schema::ByteString::from_slice(b"x").unwrap()),
            ],
        );
        let changes = vec![
            ChangeKind::Created {
                kind: NodeKind::Function,
            },
            ChangeKind::Deleted {
                kind: NodeKind::Function,
            },
            ChangeKind::Renamed {
                before: "a".into(),
                after: "b".into(),
            },
            ChangeKind::ScalarAttributeChanged {
                before: ScalarValue::I64(1),
                after: ScalarValue::I64(2),
            },
            ChangeKind::ContainmentChanged {
                before_count: 1,
                after_count: 2,
            },
            ChangeKind::OperandChanged {
                index: 0,
                before: Some(value),
                after: Some(value),
            },
            ChangeKind::DefinitionChanged {
                before: node,
                after: other,
            },
            ChangeKind::EntryFunctionChanged {
                before: Some(node),
                after: Some(other),
            },
            ChangeKind::CompletenessChanged { complete: true },
            ChangeKind::OperationRefined {
                before: OperationCode::Hole,
                after: OperationCode::ConstUnit,
                result_type: SemanticType::Unit,
                replacement: OperationKind::ConstUnit,
            },
            ChangeKind::AllocatedAndTombstoned,
        ];
        assert_family_samples(&schema, "change_kind", &changes);
    }

    #[test]
    fn schema_meta_variant_samples_are_exhaustive() {
        let schema = schema_description();
        let digest = machine_schema_digest(&schema).expect("digest");
        assert_family_samples(
            &schema,
            "schema_projection",
            &[
                SchemaProjection::Manifest,
                SchemaProjection::Roots {
                    roots: vec![SchemaRoot::Request],
                },
                SchemaProjection::Full,
            ],
        );
        assert_family_samples(
            &schema,
            "describe_schema_result",
            &[
                DescribeSchemaResult::Unchanged { digest },
                DescribeSchemaResult::Manifest(schema_manifest(&schema, digest)),
                DescribeSchemaResult::Roots(SchemaDefinitions {
                    digest,
                    roots: vec![SchemaRoot::Limits],
                    type_constructors: schema_type_constructors(),
                    definitions: vec![
                        schema_definition_catalogue(&schema)
                            .expect("catalogue")
                            .get("limits")
                            .expect("limits")
                            .clone(),
                    ],
                }),
                DescribeSchemaResult::Full {
                    digest,
                    description: Box::new(schema.clone()),
                },
            ],
        );
        assert_family_samples(&schema, "schema_root", &SchemaRoot::ALL);
        let catalogue = schema_definition_catalogue(&schema).expect("catalogue");
        let definition_bodies = [
            "bool",
            "transaction",
            "node",
            "expression",
            "expression_kind_draft",
            "query_node",
            "query_endpoint_protocol",
            "node_kind",
            "operations",
            "structured_authoring",
            "name_contract",
            "nominal_declarations",
            "id_formats",
            "limits",
        ]
        .map(|name| catalogue.get(name).expect("definition body").body.clone());
        assert_family_samples(&schema, "schema_definition_body", &definition_bodies);
        assert_family_samples(
            &schema,
            "payload_shape_kind",
            &[
                PayloadShapeKind::Unit,
                PayloadShapeKind::Newtype,
                PayloadShapeKind::Record,
            ],
        );
        assert_family_samples(
            &schema,
            "json_scalar_kind",
            &[
                JsonScalarKind::Boolean,
                JsonScalarKind::Number,
                JsonScalarKind::String,
            ],
        );
        assert_family_samples(
            &schema,
            "machine_scalar_domain",
            &[
                MachineScalarDomain::Boolean,
                MachineScalarDomain::Utf8String,
                MachineScalarDomain::SignedInteger {
                    minimum: i64::MIN,
                    maximum: i64::MAX,
                },
                MachineScalarDomain::UnsignedInteger {
                    minimum: 0,
                    maximum: u64::MAX,
                },
                MachineScalarDomain::LowercaseHex { encoded_bytes: 16 },
                MachineScalarDomain::CanonicalUrlSafeBase64 {
                    padding: false,
                    whitespace: false,
                    canonical_trailing_bits: true,
                    maximum_decoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_BYTES as u64,
                    maximum_encoded_bytes: crate::schema::MAXIMUM_BYTE_STRING_ENCODED_BYTES as u64,
                },
                MachineScalarDomain::NodeId {
                    workspace_bytes: 16,
                    minimum_serial: 1,
                    maximum_serial: u64::MAX,
                },
                MachineScalarDomain::CanonicalIdentifier {
                    grammar: "[a-z][a-z0-9_]*".into(),
                    minimum_utf8_bytes: 1,
                    maximum_utf8_bytes: crate::ids::MAX_DRAFT_SYMBOL_BYTES as u64,
                },
            ],
        );
        assert_family_samples(
            &schema,
            "run_field_type",
            &[
                RunFieldType::Workspace,
                RunFieldType::Revision,
                RunFieldType::Node,
                RunFieldType::RuntimeValueList,
                RunFieldType::RunPolicy,
                RunFieldType::U64,
                RunFieldType::U32,
            ],
        );
        assert_family_samples(
            &schema,
            "runtime_value_payload",
            &[
                RuntimeValuePayload::None,
                RuntimeValuePayload::Bool,
                RuntimeValuePayload::I64,
                RuntimeValuePayload::Bytes,
                RuntimeValuePayload::Product,
                RuntimeValuePayload::Sum,
            ],
        );
        assert_family_samples(&schema, "draft_field_type", &DraftFieldType::ALL);
        assert_family_samples(
            &schema,
            "boundary_error_kind",
            &[
                BoundaryErrorKind::InvalidJson,
                BoundaryErrorKind::InputTooLarge,
                BoundaryErrorKind::Transport,
                BoundaryErrorKind::Output,
                BoundaryErrorKind::Usage,
            ],
        );
        assert_family_samples(
            &schema,
            "operand_arity",
            &[
                OperandArity::Fixed(1),
                OperandArity::CallTargetParameters,
                OperandArity::ProductFields,
                OperandArity::VariantPayload,
            ],
        );
        assert_family_samples(
            &schema,
            "region_arity",
            &[
                RegionArity::Fixed(1),
                RegionArity::MatchVariants {
                    payload_type: TypeRule::VariantPayload,
                    terminator: OperationCode::Yield,
                    yield_type: TypeRule::MatchResult,
                },
            ],
        );
        assert_family_samples(&schema, "operand_use", &[OperandUse::Read]);
        assert_family_samples(
            &schema,
            "literal_field",
            &[
                LiteralField::I64Value,
                LiteralField::BoolValue,
                LiteralField::ExpectedType,
                LiteralField::ResultType,
                LiteralField::CarriedType,
                LiteralField::PositiveStep,
                LiteralField::BytesValue,
            ],
        );
        assert_family_samples(
            &schema,
            "block_argument_role",
            &[
                BlockArgumentRole::LoopIndex,
                BlockArgumentRole::LoopCarried,
                BlockArgumentRole::MatchPayload,
            ],
        );
        assert_family_samples(
            &schema,
            "type_rule",
            &[
                TypeRule::Fixed(SemanticType::I64),
                TypeRule::PayloadExpected,
                TypeRule::OwnerFunctionResult,
                TypeRule::PayloadResult,
                TypeRule::PayloadCarried,
                TypeRule::CallTargetParameter,
                TypeRule::CallTargetResult,
                TypeRule::OwningRegionYield,
                TypeRule::ProductFieldType,
                TypeRule::ProductDeclarationResult,
                TypeRule::ProjectionOwner,
                TypeRule::ProjectedFieldResult,
                TypeRule::VariantPayload,
                TypeRule::VariantOwnerResult,
                TypeRule::MatchScrutinee,
                TypeRule::MatchResult,
            ],
        );
        for root in SchemaRoot::ALL {
            validate_exact_type_expression(
                &schema,
                "schema_root",
                &serde_json::to_value(root).expect("root JSON"),
                None,
            )
            .expect("schema root contract");
        }
    }

    #[test]
    fn schema_json_is_strict_and_rejects_invalid_root_contracts() {
        let envelope = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(2),
            request: Request::DescribeSchema(DescribeSchemaRequest {
                projection: SchemaProjection::Roots {
                    roots: vec![SchemaRoot::RuntimeValue],
                },
                known_digest: None,
            }),
        };
        let valid = serde_json::to_string(&envelope).expect("schema request JSON");
        assert_eq!(
            decode_request(valid.as_bytes()).expect("schema JSON"),
            envelope
        );
        for invalid in [
            valid.replace("runtime_value", "unknown_root"),
            valid.replace(
                "\"runtime_value\"",
                "\"runtime_value\",\"runtime_value\"",
            ),
            valid.replace("[\"runtime_value\"]", "[]"),
            valid.replace(
                "[\"runtime_value\"]",
                "[\"runtime_value\",\"request\",\"response\",\"apply_transaction_request\",\"transaction_receipt\",\"transaction_operation\",\"expression_kind_draft\",\"operation_draft\",\"value_draft\",\"type_draft\",\"query\",\"query_result\",\"error\",\"node\",\"operations\",\"nominal_declarations\",\"id_formats\",\"limits\",\"describe_schema_request\",\"describe_schema_result\",\"runtime_value\"]",
            ),
        ] {
            assert!(decode_request(invalid.as_bytes()).is_err(), "{invalid}");
        }
        let digest = active_machine_schema_digest().expect("digest").to_string();
        let known = serde_json::to_vec(&RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(3),
            request: Request::DescribeSchema(DescribeSchemaRequest {
                projection: SchemaProjection::Manifest,
                known_digest: Some(digest.parse().expect("digest parse")),
            }),
        })
        .expect("known digest JSON");
        let known = String::from_utf8(known).expect("UTF-8");
        let known_roots = serde_json::to_string(&RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(4),
            request: Request::DescribeSchema(DescribeSchemaRequest {
                projection: SchemaProjection::Roots {
                    roots: vec![SchemaRoot::RuntimeValue],
                },
                known_digest: Some(digest.parse().expect("digest parse")),
            }),
        })
        .expect("known root JSON");
        assert!(
            decode_request(
                known_roots
                    .replace("runtime_value", "unknown_root")
                    .as_bytes()
            )
            .is_err()
        );
        let uppercase = known.replace(&digest, &digest.to_uppercase());
        assert!(decode_request(uppercase.as_bytes()).is_err());
        let null_digest = known.replace(&format!("\"{digest}\""), "null");
        let decoded = decode_request(null_digest.as_bytes()).expect("nullable known digest");
        let Request::DescribeSchema(request) = decoded.request else {
            panic!("schema request")
        };
        assert_eq!(request.known_digest, None);
    }

    #[test]
    fn advertised_runtime_value_depth_is_usable_through_the_complete_json_run_envelope() {
        let workspace = WorkspaceId::from_bytes([0xac; 16]);
        let node = NodeId::new(workspace, 2).expect("node");
        let nested_sum = |depth: usize| {
            let mut value = crate::RuntimeValue::Unit;
            for _ in 1..depth {
                value = crate::RuntimeValue::Sum {
                    ty: node,
                    variant: node,
                    payload: Some(Box::new(value)),
                };
            }
            value
        };
        let nested_product = |depth: usize| {
            let mut value = crate::RuntimeValue::Unit;
            for _ in 1..depth {
                value = crate::RuntimeValue::Product {
                    ty: node,
                    fields: vec![crate::RuntimeFieldValue { field: node, value }],
                };
            }
            value
        };
        let envelope = |value| RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(11),
            request: Request::Run {
                workspace,
                revision: Revision::INITIAL,
                entry: node,
                arguments: vec![value],
                policy: crate::RunPolicy {
                    fuel: 1,
                    maximum_frames: 1,
                },
            },
        };
        for (shape, maximum, excessive) in [
            (
                "one-field product",
                nested_product(crate::interpret::MAX_RUNTIME_VALUE_DEPTH),
                nested_product(crate::interpret::MAX_RUNTIME_VALUE_DEPTH + 1),
            ),
            (
                "payload sum",
                nested_sum(crate::interpret::MAX_RUNTIME_VALUE_DEPTH),
                nested_sum(crate::interpret::MAX_RUNTIME_VALUE_DEPTH + 1),
            ),
        ] {
            let maximum = envelope(maximum);
            let bytes = serde_json::to_vec(&maximum).expect("maximum runtime envelope JSON");
            assert_eq!(
                decode_request(&bytes).unwrap_or_else(|error| panic!("{shape}: {error:?}")),
                maximum
            );

            let excessive = envelope(excessive);
            let bytes = serde_json::to_vec(&excessive).expect("excessive runtime envelope JSON");
            assert_eq!(
                decode_request(&bytes)
                    .expect_err("runtime depth policy")
                    .kind,
                BoundaryErrorKind::InputTooLarge,
                "{shape}"
            );
            assert_eq!(request_id_hint(&bytes), Some(RequestId::new(11)), "{shape}");
        }
    }

    #[test]
    fn semantic_query_policy_is_not_part_of_json_decoding() {
        let workspace = WorkspaceId::from_bytes([0xcd; 16]);
        let request = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(2),
            request: Request::QueryBatch(QueryBatchRequest {
                workspace,
                revision: Revision::INITIAL,
                queries: vec![
                    QueryItem {
                        id: QueryId::new(1),
                        query: Query::Blockers {
                            page: PageRequest {
                                after: None,
                                limit: 0,
                            },
                        },
                    },
                    QueryItem {
                        id: QueryId::new(1),
                        query: Query::RepairContext {
                            target: crate::query::RepairTarget::Hole(
                                crate::NodeId::new(workspace, 1).expect("node"),
                            ),
                            budget: ContextBudget {
                                body_before: MAX_CONTEXT_ITEMS + 1,
                                body_after: 0,
                                visible_values: 0,
                                incoming_uses: 0,
                                include_incompatible: false,
                            },
                        },
                    },
                ],
            }),
        };
        let bytes = serde_json::to_vec(&request).expect("request JSON");
        assert_eq!(decode_request(&bytes).expect("typed JSON decode"), request);
    }

    #[test]
    fn bounded_writer_stops_before_materializing_oversized_json() {
        let value = "x".repeat(256);
        for pretty in [false, true] {
            let error = encode_with_limit(&value, pretty, 32).expect_err("output limit");
            assert_eq!(error.kind, BoundaryErrorKind::Output);
        }
        let expected = serde_json::to_vec(&value).expect("small expected JSON");
        assert_eq!(
            encode_with_limit(&value, false, expected.len()).expect("exact output limit"),
            expected
        );
        let error = encode_with_limit(&value, false, expected.len() - 1)
            .expect_err("one byte below output limit");
        assert_eq!(error.kind, BoundaryErrorKind::Output);

        let boundary = encode_boundary_error(
            Some(RequestId::new(9)),
            BoundaryErrorKind::InvalidJson,
            "é".repeat(10_000),
        );
        assert!(boundary.len() < 2048);
        let decoded: BoundaryErrorEnvelope =
            serde_json::from_slice(&boundary).expect("boundary error JSON");
        assert_eq!(decoded.request_id, Some(RequestId::new(9)));
        assert!(decoded.error.message.len() <= MAX_BOUNDARY_ERROR_MESSAGE_BYTES);
        assert!(
            decoded
                .error
                .message
                .is_char_boundary(decoded.error.message.len())
        );
        assert!(BOUNDARY_ERROR_FALLBACK.len() < MAX_JSON_OUTPUT_BYTES);
    }

    #[test]
    fn advertised_structured_depth_is_accepted_by_strict_json_and_above_rejects_semantically() {
        let workspace = WorkspaceId::from_bytes([0xce; 16]);
        let block = NodeId::new(workspace, 2).expect("block");
        let local = NodeTarget::Draft;
        let nested = |levels: usize| {
            let mut expression = ExpressionDraft {
                symbol: Some(DraftSymbol::generated(1)),
                operation: ExpressionKindDraft::ConstI64(1),
            };
            for level in 0..levels {
                let inner = expression.symbol;
                let else_handle = DraftSymbol::generated(100 + level as u32);
                expression = ExpressionDraft {
                    symbol: Some(DraftSymbol::generated(1000 + level as u32)),
                    operation: ExpressionKindDraft::If {
                        condition: ValueDraft::FunctionParameter(NodeTarget::Existing(
                            NodeId::new(workspace, 3).expect("parameter"),
                        )),
                        result: SemanticType::I64.into(),
                        then_body: YieldingBodyDraft {
                            operations: vec![expression],
                            yield_value: ValueDraft::OperationResult {
                                operation: local(inner.expect("bound expression")),
                                output: 0,
                            },
                        },
                        else_body: YieldingBodyDraft {
                            operations: vec![ExpressionDraft {
                                symbol: Some(else_handle),
                                operation: ExpressionKindDraft::ConstI64(0),
                            }],
                            yield_value: ValueDraft::OperationResult {
                                operation: local(else_handle),
                                output: 0,
                            },
                        },
                    },
                };
            }
            RequestEnvelope {
                version: JSON_ENVELOPE_VERSION,
                request_id: RequestId::new(77),
                request: Request::ApplyTransaction(ApplyTransactionRequest {
                    transaction: Transaction {
                        workspace,
                        base_revision: Revision::INITIAL,
                        idempotency_key: None,
                        mode: TransactionMode::Commit,
                        operations: vec![TransactionOp::InsertExpression {
                            block,
                            before: None,
                            expression,
                        }],
                    },
                    response: TransactionResponseSpec::default(),
                }),
            }
        };
        let maximum = nested(crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH);
        let bytes = serde_json::to_vec(&maximum).expect("maximum JSON encode");
        let decoded = decode_request(&bytes).expect("maximum strict JSON decode");
        let Request::ApplyTransaction(request) = decoded.request else {
            panic!("transaction")
        };
        crate::transaction::validate_structured_request(&request.transaction.operations)
            .expect("advertised maximum accepted");

        let above = nested(crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH + 1);
        let bytes = serde_json::to_vec(&above).expect("above JSON encode");
        let decoded = decode_request(&bytes).expect("above remains parser-safe");
        let Request::ApplyTransaction(request) = decoded.request else {
            panic!("transaction")
        };
        assert_eq!(
            crate::transaction::validate_structured_request(&request.transaction.operations)
                .expect_err("above structured depth")
                .code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn described_optional_and_required_fields_match_strict_serde() {
        let schema = schema_description();
        let optional_draft_fields = schema
            .structured_authoring
            .records
            .iter()
            .map(|record| (record.name.as_str(), &record.fields))
            .chain(
                schema
                    .structured_authoring
                    .expression_variants
                    .iter()
                    .map(|variant| (variant.name.as_str(), &variant.fields)),
            )
            .chain(
                schema
                    .structured_authoring
                    .operation_variants
                    .iter()
                    .map(|variant| (variant.name.as_str(), &variant.fields)),
            )
            .flat_map(|(owner, fields)| {
                fields
                    .iter()
                    .filter(|field| !field.required)
                    .map(move |field| (owner, field.name.as_str(), field.nullable))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            optional_draft_fields,
            vec![
                ("sum_variant", "payload", true),
                ("create_function", "body", true),
                ("match_arm", "payload_symbol", true),
                ("expression", "symbol", true),
                ("insert_expression", "before", true),
                ("construct_variant", "payload", true),
                ("construct_variant", "payload", true),
            ]
        );
        let workspace = WorkspaceId::from_bytes([0x71; 16]);
        let node = NodeId::new(workspace, 7).expect("node");
        let symbol = DraftSymbol::generated(1);
        let body = crate::transaction::FunctionBodyDraft {
            operations: vec![],
            return_value: ValueDraft::FunctionParameter(NodeTarget::Existing(node)),
        };

        let create_function = TransactionOp::CreateFunction {
            symbol,
            module: NodeTarget::Existing(node),
            name: "f".into(),
            parameters: vec![],
            result: crate::TypeDraft::I64,
            body: Some(body.clone()),
        };
        assert_draft_record_serde_contract::<TransactionOp>(
            serde_json::to_value(create_function).expect("create function JSON"),
            &schema,
            "create_function",
            true,
        );

        let insert = TransactionOp::InsertExpression {
            block: node,
            before: Some(node),
            expression: ExpressionDraft {
                symbol: Some(symbol),
                operation: ExpressionKindDraft::ConstI64(1),
            },
        };
        assert_draft_record_serde_contract::<TransactionOp>(
            serde_json::to_value(insert).expect("insert expression JSON"),
            &schema,
            "insert_expression",
            true,
        );

        let sum_variant = crate::transaction::SumVariantDraft {
            symbol,
            name: "some".into(),
            payload: Some(crate::TypeDraft::I64),
        };
        assert_draft_record_serde_contract::<crate::transaction::SumVariantDraft>(
            serde_json::to_value(sum_variant).expect("sum variant JSON"),
            &schema,
            "sum_variant",
            false,
        );
        let match_arm = crate::transaction::MatchArmDraft {
            variant: NodeTarget::Existing(node),
            payload_symbol: Some(symbol),
            body: crate::transaction::YieldingBodyDraft {
                operations: vec![],
                yield_value: ValueDraft::FunctionParameter(NodeTarget::Existing(node)),
            },
        };
        assert_draft_record_serde_contract::<crate::transaction::MatchArmDraft>(
            serde_json::to_value(match_arm).expect("match arm JSON"),
            &schema,
            "match_arm",
            false,
        );
        let construct_expression = ExpressionKindDraft::ConstructVariant {
            variant: NodeTarget::Existing(node),
            payload: Some(ValueDraft::FunctionParameter(NodeTarget::Existing(node))),
        };
        let expression_fields = &schema
            .structured_authoring
            .expression_variants
            .iter()
            .find(|variant| variant.name == "construct_variant")
            .expect("construct variant expression description")
            .fields;
        assert_draft_fields_serde_contract::<ExpressionKindDraft>(
            serde_json::to_value(construct_expression).expect("construct expression JSON"),
            &schema,
            expression_fields,
            true,
        );
        let construct_operation = crate::OperationDraft::ConstructVariant {
            variant: NodeTarget::Existing(node),
            payload: Some(ValueDraft::FunctionParameter(NodeTarget::Existing(node))),
        };
        let operation_fields = &schema
            .structured_authoring
            .operation_variants
            .iter()
            .find(|variant| variant.name == "construct_variant")
            .expect("construct variant operation description")
            .fields;
        assert_draft_fields_serde_contract::<crate::OperationDraft>(
            serde_json::to_value(construct_operation).expect("construct operation JSON"),
            &schema,
            operation_fields,
            true,
        );

        let runtime_value = crate::RuntimeValue::Sum {
            ty: node,
            variant: node,
            payload: Some(Box::new(crate::RuntimeValue::Unit)),
        };
        let runtime_sum = schema
            .run
            .variants
            .iter()
            .find(|description| description.name == "runtime_value")
            .and_then(|description| {
                description
                    .variants
                    .iter()
                    .find(|variant| variant.name == "sum")
            })
            .expect("runtime sum description");
        assert_eq!(
            runtime_sum.payload.newtype.as_deref(),
            Some("runtime_sum_data")
        );
        assert_machine_record_serde_contract::<crate::RuntimeValue>(
            serde_json::to_value(runtime_value).expect("runtime sum JSON"),
            named_payload(&schema.run.records, "runtime_sum_data"),
            true,
        );

        let transaction = Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: Some(crate::IdempotencyKey::from_bytes([5; 16])),
            mode: TransactionMode::Commit,
            operations: vec![],
        };
        assert_machine_record_serde_contract::<Transaction>(
            serde_json::to_value(transaction).expect("transaction JSON"),
            named_payload(&schema.transaction_records, "transaction"),
            false,
        );

        let cursor = crate::query::PageCursor::Blockers {
            workspace,
            revision: Revision::INITIAL,
            next: 1,
        };
        let page_request = PageRequest {
            after: Some(cursor),
            limit: 1,
        };
        assert_machine_record_serde_contract::<PageRequest>(
            serde_json::to_value(page_request).expect("page request JSON"),
            named_payload(&schema.query_records, "page_request"),
            false,
        );
        let page = crate::query::Page::<u64> {
            items: vec![1],
            next: Some(cursor),
            total: Some(1),
        };
        assert_machine_record_serde_contract_with_parameter::<crate::query::Page<u64>>(
            serde_json::to_value(page).expect("page JSON"),
            named_payload(&schema.query_records, "page"),
            false,
            Some("u64"),
        );

        let receipt = TransactionReceipt {
            workspace,
            base_revision: Revision::INITIAL,
            revision: Revision::new(1),
            hash: crate::SnapshotHash::from_bytes([2; 32]),
            published: true,
            created_count: 1,
            returned_bindings: vec![(symbol, node)],
            change_count: 1,
            change_digest: crate::ChangeDigest::from_bytes([3; 32]),
            complete_before: false,
            complete_after: true,
            blocker_count_before: 1,
            blocker_count_after: 0,
        };
        assert_machine_record_serde_contract::<TransactionReceipt>(
            serde_json::to_value(receipt).expect("receipt JSON"),
            named_payload(&schema.transaction_records, "transaction_receipt"),
            false,
        );
        let result = crate::query::QueryBatchResult {
            workspace,
            revision: Revision::INITIAL,
            results: vec![],
        };
        assert_machine_record_serde_contract::<crate::query::QueryBatchResult>(
            serde_json::to_value(result).expect("query result JSON"),
            named_payload(&schema.query_records, "query_batch_result"),
            false,
        );

        let error = crate::error::LkError::new(ErrorCode::InvalidQuery, "bad")
            .for_workspace(workspace)
            .at_revision(Revision::INITIAL)
            .for_node(node);
        assert_machine_record_serde_contract::<crate::error::LkError>(
            serde_json::to_value(error).expect("error JSON"),
            &schema.error_payload,
            false,
        );

        let schema_request = DescribeSchemaRequest {
            projection: SchemaProjection::Full,
            known_digest: Some(active_machine_schema_digest().expect("digest")),
        };
        assert_machine_record_serde_contract::<DescribeSchemaRequest>(
            serde_json::to_value(schema_request).expect("schema request JSON"),
            &schema.schema_discovery.request,
            false,
        );
    }

    #[test]
    fn every_advertised_public_variant_matches_strict_serde() {
        use crate::diff::{Change, ChangeKind};
        use crate::interpret::{RunResult, RuntimeFieldValue, RuntimeValue, RuntimeValueCode};
        use crate::query::{
            BlockArgumentFact, BodyItem, CompletenessBlocker, ConstructorDescriptor,
            DefinitionReferenceSite, DefinitionSlot, DependencyFact, EnclosingRegionFact,
            ExpectedCategory, FunctionSignatureSummary, LegalConstructorsResult, NamePreview,
            NodeSummary, NodeView, NominalLayoutSummary, NominalMemberFact, NominalTypeResult,
            OwnedRegionSummary, OwnerFact, Page, PageCursor, QueryCode, QueryResult, RepairContext,
            RepairTarget, SemanticDiffPage, UseSite, VisibleCursorPurpose, VisibleValue,
            WorkspaceSummary,
        };
        use crate::schema::{
            BlockArgumentRole, MatchArmOperationDraft, OperationDraft, ProductFieldValueDraft,
            RegionRole, TypeDraft, ValueRef,
        };
        use crate::transaction::{
            ExpressionDraftCode, FunctionBodyDraft, FunctionParameterDraft, MatchArmDraft,
            ProductFieldDraft, SumVariantDraft, ValueDraftCode,
        };

        let schema = schema_description();
        let workspace = WorkspaceId::from_bytes([9; 16]);
        let node = NodeId::new(workspace, 9).expect("node");
        let other = NodeId::new(workspace, 10).expect("other node");
        let target = NodeTarget::Existing(node);
        let value = ValueDraft::FunctionParameter(target);
        let value_ref = ValueRef::FunctionParameter(node);
        let page_request = PageRequest {
            after: None,
            limit: 1,
        };
        let yielding = YieldingBodyDraft {
            operations: vec![],
            yield_value: value.clone(),
        };
        let function_body = FunctionBodyDraft {
            operations: vec![],
            return_value: value.clone(),
        };
        let expression = ExpressionDraft {
            symbol: Some(DraftSymbol::generated(20)),
            operation: ExpressionKindDraft::ConstI64(1),
        };
        let transaction_samples = vec![
            TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "p".into(),
            },
            TransactionOp::CreateModule {
                symbol: DraftSymbol::generated(2),
                package: target,
                name: "m".into(),
            },
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: target,
                name: "f".into(),
                parameters: vec![FunctionParameterDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "x".into(),
                    ty: TypeDraft::I64,
                }],
                result: TypeDraft::I64,
                body: Some(function_body.clone()),
            },
            TransactionOp::DefineFunctionBody {
                function: node,
                body: function_body.clone(),
            },
            TransactionOp::InsertExpression {
                block: node,
                before: Some(other),
                expression: expression.clone(),
            },
            TransactionOp::SetEntryFunction {
                package: target,
                function: target,
            },
            TransactionOp::RenameNode {
                node: target,
                name: "renamed".into(),
            },
            TransactionOp::ReplaceOperation {
                operation: target,
                replacement: OperationDraft::ConstI64(1),
            },
            TransactionOp::ReplaceOperand {
                operation: target,
                index: 0,
                value: value.clone(),
            },
            TransactionOp::DeleteOwnedSubtree { root: target },
            TransactionOp::RefineHole {
                hole: target,
                replacement: OperationDraft::ConstI64(1),
            },
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(5),
                module: target,
                name: "pair".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(6),
                    name: "value".into(),
                    ty: TypeDraft::I64,
                }],
            },
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(7),
                module: target,
                name: "maybe".into(),
                variants: vec![SumVariantDraft {
                    symbol: DraftSymbol::generated(8),
                    name: "some".into(),
                    payload: Some(TypeDraft::I64),
                }],
            },
        ];
        assert_eq!(transaction_samples.len(), TransactionOpCode::ALL.len());
        for (sample, code) in transaction_samples.iter().zip(TransactionOpCode::ALL) {
            assert_eq!(sample.code(), code);
            assert_machine_variant_serde_contract(
                sample,
                &schema.transaction_operation_payloads,
                code.machine_name(),
            );
        }

        let query_samples = vec![
            Query::WorkspaceSummary,
            Query::Node { node, expand: true },
            Query::Blockers { page: page_request },
            Query::OwnerChain {
                node,
                page: page_request,
            },
            Query::Body {
                block: node,
                page: page_request,
            },
            Query::IncomingUses {
                value: value_ref,
                page: page_request,
            },
            Query::DefinitionReferences {
                target: node,
                page: page_request,
            },
            Query::Dependencies {
                node,
                page: page_request,
            },
            Query::VisibleValues {
                purpose: VisibleCursorPurpose::VisibleValues,
                target: RepairTarget::Hole(node),
                include_incompatible: true,
                page: page_request,
            },
            Query::LegalConstructors {
                target: RepairTarget::Hole(node),
                include_incompatible: true,
                constructors: page_request,
                values: page_request,
            },
            Query::SemanticDiff {
                from: Revision::INITIAL,
                page: page_request,
            },
            Query::RepairContext {
                target: RepairTarget::Hole(node),
                budget: ContextBudget {
                    body_before: 1,
                    body_after: 1,
                    visible_values: 1,
                    incoming_uses: 1,
                    include_incompatible: true,
                },
            },
            Query::NominalType {
                declaration: node,
                page: page_request,
            },
        ];
        assert_eq!(query_samples.len(), QueryCode::ALL.len());
        for (sample, code) in query_samples.iter().zip(QueryCode::ALL) {
            assert_eq!(sample.code(), code);
            assert_machine_variant_serde_contract(
                sample,
                &schema.query_payloads,
                code.machine_name(),
            );
        }

        let name = NamePreview {
            value: "name".into(),
            truncated: false,
        };
        let signature = FunctionSignatureSummary {
            parameter_count: 1,
            result: SemanticType::I64,
        };
        let summary = WorkspaceSummary {
            workspace,
            revision: Revision::INITIAL,
            hash: crate::SnapshotHash::from_bytes([1; 32]),
            root: node,
            node_count: 2,
            complete: true,
            blocker_count: 0,
            entry_count: 1,
        };
        let node_summary = NodeSummary {
            workspace,
            revision: Revision::INITIAL,
            node,
            kind: NodeKind::Function,
            owner: Some(other),
            display_name: Some(name.clone()),
            signature: Some(signature),
            value_type: Some(SemanticType::I64),
            complete: true,
            blocker_count: 0,
            child_count: 1,
            outgoing_reference_count: 1,
        };
        let blocker = CompletenessBlocker {
            owner: node,
            target: Some(other),
            category: ExpectedCategory::Expression,
            expected_type: Some(SemanticType::I64),
        };
        let owner = OwnerFact {
            node,
            kind: NodeKind::Function,
            name: Some(name.clone()),
        };
        let definition = DefinitionReferenceSite {
            source: node,
            slot: DefinitionSlot::CallTarget,
            target: other,
        };
        let body_item = BodyItem {
            operation: node,
            ordinal: 0,
            code: OperationCode::ConstI64,
            result_types: vec![SemanticType::I64],
            operands: vec![value_ref],
            definitions: vec![definition],
            complete: true,
            terminator: false,
            literal: Some(crate::query::LiteralValue::I64(1)),
            owned_regions: vec![OwnedRegionSummary {
                region: other,
                role: RegionRole::IfThen,
            }],
        };
        let use_site = UseSite {
            source: node,
            operand_index: 0,
            target: value_ref,
            owner_block: other,
            owner_function: node,
            expected_type: SemanticType::I64,
            use_mode: OperandUse::Read,
        };
        let visible = VisibleValue {
            value: value_ref,
            ty: SemanticType::I64,
            compatible: true,
            producer: node,
            producer_code: Some(OperationCode::ConstI64),
            owner_function: other,
            ordinal: Some(0),
            name: Some(name.clone()),
        };
        let continuation = crate::query::NominalTypeContinuation {
            declaration: node,
            page: page_request,
        };
        let constructor = ConstructorDescriptor {
            code: OperationCode::ConstructProduct,
            result_type: SemanticType::Nominal(node),
            operand_count: 1,
            operand_types: vec![SemanticType::I64],
            operand_uses: vec![OperandUse::Read],
            literal_fields: vec![],
            call_target: Some(other),
            declaration: Some(node),
            member_count: 1,
            members: vec![other],
            requirements_complete: true,
            nominal_type_continuation: Some(continuation.clone()),
            direct_refinement: true,
            complete: true,
            terminator: false,
        };
        let cursor = PageCursor::NominalType {
            workspace,
            revision: Revision::INITIAL,
            declaration: node,
            next: 1,
        };
        fn page<T>(cursor: PageCursor, items: Vec<T>) -> Page<T> {
            Page {
                items,
                next: Some(cursor),
                total: Some(1),
            }
        }
        let layout = NominalLayoutSummary {
            representable: true,
            failure: None,
            size: Some(8),
            align: Some(8),
            cells: Some(1),
            discriminant_bytes: Some(1),
            payload_offset: Some(8),
        };
        let member = NominalMemberFact::ProductField {
            field: other,
            name: "value".into(),
            ordinal: 0,
            ty: SemanticType::I64,
            offset: Some(0),
            cells: Some(1),
        };
        let nominal = NominalTypeResult {
            declaration: node,
            name: "pair".into(),
            kind: NodeKind::ProductType,
            owner: other,
            layout: layout.clone(),
            members: page(cursor, vec![member.clone()]),
        };
        let legal = LegalConstructorsResult {
            target: RepairTarget::Hole(node),
            expected_type: SemanticType::Nominal(node),
            constructors: page(cursor, vec![constructor.clone()]),
            visible_values: page(cursor, vec![visible.clone()]),
        };
        let member_samples = [
            member.clone(),
            NominalMemberFact::SumVariant {
                variant: other,
                name: "some".into(),
                ordinal: 0,
                payload: Some(SemanticType::I64),
                discriminant: Some(0),
                payload_size: Some(8),
                payload_align: Some(8),
                payload_cells: Some(1),
            },
        ];
        assert_eq!(member_samples.len(), schema.query_member_payloads.len());
        for (sample, description) in member_samples.iter().zip(&schema.query_member_payloads) {
            assert_machine_variant_serde_contract(
                sample,
                &schema.query_member_payloads,
                &description.name,
            );
        }
        let repair = RepairContext {
            workspace,
            revision: Revision::INITIAL,
            target: RepairTarget::Hole(node),
            operation: node,
            operation_code: OperationCode::Hole,
            operand_index: Some(0),
            expected_type: SemanticType::I64,
            use_mode: Some(OperandUse::Read),
            current_value: Some(value_ref),
            current_actual_type: Some(SemanticType::I64),
            owner_block: other,
            owner_function: node,
            ordinal: 0,
            function_signature: signature,
            owner_chain: vec![owner.clone()],
            enclosing_regions: vec![EnclosingRegionFact {
                region: other,
                owner_operation: node,
                role: RegionRole::IfThen,
            }],
            visible_block_arguments: vec![BlockArgumentFact {
                argument: other,
                block: other,
                region: other,
                ordinal: 0,
                role: BlockArgumentRole::LoopIndex,
                ty: SemanticType::I64,
            }],
            body_window: vec![body_item.clone()],
            visible_values: page(cursor, vec![visible.clone()]),
            incoming_uses: page(cursor, vec![use_site]),
            legal_constructor_count: 1,
            legal_constructors: vec![constructor.clone()],
            nominal_type: Some(nominal.clone()),
            nominal_type_continuation: Some(continuation),
            blocker: Some(blocker.clone()),
            refinement_operation: Some(TransactionOpCode::RefineHole),
        };
        let query_result_samples = vec![
            QueryResult::WorkspaceSummary(summary.clone()),
            QueryResult::Node(NodeView {
                summary: node_summary.clone(),
                record: None,
            }),
            QueryResult::Blockers(page(cursor, vec![blocker.clone()])),
            QueryResult::OwnerChain(page(cursor, vec![owner.clone()])),
            QueryResult::Body(page(cursor, vec![body_item.clone()])),
            QueryResult::IncomingUses(page(cursor, vec![use_site])),
            QueryResult::DefinitionReferences(page(cursor, vec![definition])),
            QueryResult::Dependencies(page(
                cursor,
                vec![DependencyFact::ValueOperand {
                    index: 0,
                    value: value_ref,
                }],
            )),
            QueryResult::VisibleValues(page(cursor, vec![visible.clone()])),
            QueryResult::LegalConstructors(legal.clone()),
            QueryResult::SemanticDiff(SemanticDiffPage {
                from: Revision::INITIAL,
                to: Revision::new(1),
                change_count: 1,
                change_digest: crate::ChangeDigest::from_bytes([2; 32]),
                page: page(
                    cursor,
                    vec![Change {
                        node,
                        kind: ChangeKind::CompletenessChanged { complete: true },
                    }],
                ),
            }),
            QueryResult::RepairContext(Box::new(repair.clone())),
            QueryResult::NominalType(nominal.clone()),
        ];
        assert_eq!(query_result_samples.len(), QueryCode::ALL.len());
        for (sample, code) in query_result_samples.iter().zip(QueryCode::ALL) {
            assert_machine_variant_serde_contract(
                sample,
                &schema.query_result_payloads,
                code.machine_name(),
            );
        }

        let cursor_samples = vec![
            PageCursor::Blockers {
                workspace,
                revision: Revision::INITIAL,
                next: 1,
            },
            PageCursor::OwnerChain {
                workspace,
                revision: Revision::INITIAL,
                node,
                next: 1,
            },
            PageCursor::Body {
                workspace,
                revision: Revision::INITIAL,
                block: node,
                next: 1,
            },
            PageCursor::IncomingUses {
                workspace,
                revision: Revision::INITIAL,
                value: value_ref,
                next: 1,
            },
            PageCursor::DefinitionReferences {
                workspace,
                revision: Revision::INITIAL,
                target: node,
                next: 1,
            },
            PageCursor::Dependencies {
                workspace,
                revision: Revision::INITIAL,
                node,
                next: 1,
            },
            PageCursor::VisibleValues {
                workspace,
                revision: Revision::INITIAL,
                purpose: VisibleCursorPurpose::VisibleValues,
                target: RepairTarget::Hole(node),
                expected: SemanticType::I64,
                include_incompatible: true,
                next: 1,
            },
            PageCursor::LegalConstructors {
                workspace,
                revision: Revision::INITIAL,
                target: RepairTarget::Hole(node),
                expected: SemanticType::I64,
                next: 1,
            },
            PageCursor::Diff {
                workspace,
                from: Revision::INITIAL,
                to: Revision::new(1),
                next: 1,
            },
            PageCursor::NominalType {
                workspace,
                revision: Revision::INITIAL,
                declaration: node,
                next: 1,
            },
        ];
        assert_eq!(cursor_samples.len(), schema.query_cursor_payloads.len());
        for (sample, description) in cursor_samples.iter().zip(&schema.query_cursor_payloads) {
            assert_machine_variant_serde_contract(
                sample,
                &schema.query_cursor_payloads,
                &description.name,
            );
        }

        let expression_samples = vec![
            ExpressionKindDraft::ConstUnit,
            ExpressionKindDraft::ConstBool(true),
            ExpressionKindDraft::ConstI64(1),
            ExpressionKindDraft::AddI64 {
                lhs: value.clone(),
                rhs: value.clone(),
            },
            ExpressionKindDraft::LtI64 {
                lhs: value.clone(),
                rhs: value.clone(),
            },
            ExpressionKindDraft::Call {
                function: target,
                arguments: vec![value.clone()],
            },
            ExpressionKindDraft::Hole {
                expected: TypeDraft::I64,
            },
            ExpressionKindDraft::If {
                condition: value.clone(),
                result: TypeDraft::I64,
                then_body: yielding.clone(),
                else_body: yielding.clone(),
            },
            ExpressionKindDraft::ForI64 {
                start: value.clone(),
                end_exclusive: value.clone(),
                step: 1,
                initial: value.clone(),
                carried: TypeDraft::I64,
                index_symbol: DraftSymbol::generated(30),
                carried_symbol: DraftSymbol::generated(31),
                body: yielding.clone(),
            },
            ExpressionKindDraft::ConstructProduct {
                product: target,
                fields: vec![ProductFieldValueDraft {
                    field: target,
                    value: value.clone(),
                }],
            },
            ExpressionKindDraft::ProjectField {
                value: value.clone(),
                field: target,
            },
            ExpressionKindDraft::ConstructVariant {
                variant: target,
                payload: Some(value.clone()),
            },
            ExpressionKindDraft::MatchSum {
                scrutinee: value.clone(),
                result: TypeDraft::I64,
                arms: vec![MatchArmDraft {
                    variant: target,
                    payload_symbol: Some(DraftSymbol::generated(32)),
                    body: yielding.clone(),
                }],
            },
            ExpressionKindDraft::ConstBytes(crate::schema::ByteString::from_slice(b"x").unwrap()),
            ExpressionKindDraft::BytesLen {
                value: value.clone(),
            },
            ExpressionKindDraft::BytesAt {
                value: value.clone(),
                index: value.clone(),
            },
            ExpressionKindDraft::BytesSlice {
                value: value.clone(),
                start: value.clone(),
                length: value.clone(),
            },
            ExpressionKindDraft::BytesEqual {
                lhs: value.clone(),
                rhs: value.clone(),
            },
        ];
        assert_eq!(expression_samples.len(), ExpressionDraftCode::ALL.len());
        for (sample, code) in expression_samples.iter().zip(ExpressionDraftCode::ALL) {
            assert_draft_variant_serde_contract(
                sample,
                &schema,
                &schema.structured_authoring.expression_variants,
                code.machine_name(),
            );
        }

        let operation_samples = vec![
            OperationDraft::ConstUnit,
            OperationDraft::ConstI64(1),
            OperationDraft::ConstBool(true),
            OperationDraft::AddI64 {
                lhs: value.clone(),
                rhs: value.clone(),
            },
            OperationDraft::LtI64 {
                lhs: value.clone(),
                rhs: value.clone(),
            },
            OperationDraft::Call {
                function: target,
                arguments: vec![value.clone()],
            },
            OperationDraft::Hole {
                expected: TypeDraft::I64,
            },
            OperationDraft::If {
                condition: value.clone(),
                result: TypeDraft::I64,
                then_region: target,
                else_region: target,
            },
            OperationDraft::ForI64 {
                start: value.clone(),
                end_exclusive: value.clone(),
                step: 1,
                initial: value.clone(),
                carried: TypeDraft::I64,
                body_region: target,
            },
            OperationDraft::Return {
                value: value.clone(),
            },
            OperationDraft::Yield {
                value: value.clone(),
            },
            OperationDraft::ConstructProduct {
                product: target,
                fields: vec![ProductFieldValueDraft {
                    field: target,
                    value: value.clone(),
                }],
            },
            OperationDraft::ProjectField {
                value: value.clone(),
                field: target,
            },
            OperationDraft::ConstructVariant {
                variant: target,
                payload: Some(value.clone()),
            },
            OperationDraft::MatchSum {
                scrutinee: value.clone(),
                result: TypeDraft::I64,
                arms: vec![MatchArmOperationDraft {
                    variant: target,
                    region: target,
                }],
            },
            OperationDraft::ConstBytes(crate::schema::ByteString::from_slice(b"x").unwrap()),
            OperationDraft::BytesLen {
                value: value.clone(),
            },
            OperationDraft::BytesAt {
                value: value.clone(),
                index: value.clone(),
            },
            OperationDraft::BytesSlice {
                value: value.clone(),
                start: value.clone(),
                length: value.clone(),
            },
            OperationDraft::BytesEqual {
                lhs: value.clone(),
                rhs: value.clone(),
            },
        ];
        assert_eq!(operation_samples.len(), OperationCode::ALL.len());
        for (sample, code) in operation_samples.iter().zip(OperationCode::ALL) {
            assert_eq!(sample.code(), code);
            assert_draft_variant_serde_contract(
                sample,
                &schema,
                &schema.structured_authoring.operation_variants,
                code.machine_name(),
            );
        }

        let value_samples = [
            ValueDraft::FunctionParameter(target),
            ValueDraft::OperationResult {
                operation: target,
                output: 0,
            },
            ValueDraft::BlockArgument(target),
            ValueDraft::InlineExpression(Box::new(ExpressionKindDraft::ConstI64(1))),
        ];
        assert_eq!(value_samples.len(), ValueDraftCode::ALL.len());
        for (sample, code) in value_samples.iter().zip(ValueDraftCode::ALL) {
            assert_eq!(sample.code(), code);
            assert_draft_variant_serde_contract(
                sample,
                &schema,
                &schema.structured_authoring.value_variants,
                code.machine_name(),
            );
        }
        let runtime_samples = vec![
            RuntimeValue::Unit,
            RuntimeValue::Bool(true),
            RuntimeValue::I64(1),
            RuntimeValue::Bytes(crate::schema::ByteString::from_slice(b"x").unwrap()),
            RuntimeValue::Product {
                ty: node,
                fields: vec![RuntimeFieldValue {
                    field: other,
                    value: RuntimeValue::I64(1),
                }],
            },
            RuntimeValue::Sum {
                ty: node,
                variant: other,
                payload: Some(Box::new(RuntimeValue::I64(1))),
            },
        ];
        assert_eq!(runtime_samples.len(), RuntimeValueCode::ALL.len());
        let runtime_variants = &schema
            .run
            .variants
            .iter()
            .find(|family| family.name == "runtime_value")
            .expect("runtime family")
            .variants;
        assert_eq!(runtime_samples.len(), schema.run.runtime_values.len());
        for ((sample, code), advertised) in runtime_samples
            .iter()
            .zip(RuntimeValueCode::ALL)
            .zip(&schema.run.runtime_values)
        {
            assert_eq!(sample.code(), code);
            assert_eq!(advertised.name, code.machine_name());
            assert_machine_variant_serde_contract(sample, runtime_variants, code.machine_name());
        }

        let type_samples = [
            TypeDraft::Unit,
            TypeDraft::Bool,
            TypeDraft::I64,
            TypeDraft::Bytes,
            TypeDraft::Nominal(target),
        ];
        assert_eq!(
            type_samples.len(),
            schema.structured_authoring.type_variants.len()
        );
        for (sample, description) in type_samples
            .iter()
            .zip(&schema.structured_authoring.type_variants)
        {
            assert_type_draft_serde_contract(*sample, &schema, description);
        }

        macro_rules! check_draft_record {
            ($name:literal, $sample:expr, $ty:ty, $tagged:literal) => {{
                assert_draft_record_serde_contract::<$ty>(
                    serde_json::to_value($sample).expect("draft record sample"),
                    &schema,
                    $name,
                    $tagged,
                );
                1_usize
            }};
        }
        let mut structured_record_count = 0_usize;
        structured_record_count += check_draft_record!(
            "create_product_type",
            TransactionOp::CreateProductType {
                symbol: DraftSymbol::generated(5),
                module: target,
                name: "pair".into(),
                fields: vec![ProductFieldDraft {
                    symbol: DraftSymbol::generated(6),
                    name: "value".into(),
                    ty: TypeDraft::I64
                }]
            },
            TransactionOp,
            true
        );
        structured_record_count += check_draft_record!(
            "product_field",
            ProductFieldDraft {
                symbol: DraftSymbol::generated(6),
                name: "value".into(),
                ty: TypeDraft::I64
            },
            ProductFieldDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "create_sum_type",
            TransactionOp::CreateSumType {
                symbol: DraftSymbol::generated(7),
                module: target,
                name: "maybe".into(),
                variants: vec![SumVariantDraft {
                    symbol: DraftSymbol::generated(8),
                    name: "some".into(),
                    payload: Some(TypeDraft::I64)
                }]
            },
            TransactionOp,
            true
        );
        structured_record_count += check_draft_record!(
            "sum_variant",
            SumVariantDraft {
                symbol: DraftSymbol::generated(8),
                name: "some".into(),
                payload: Some(TypeDraft::I64)
            },
            SumVariantDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "create_function",
            TransactionOp::CreateFunction {
                symbol: DraftSymbol::generated(3),
                module: target,
                name: "f".into(),
                parameters: vec![FunctionParameterDraft {
                    symbol: DraftSymbol::generated(4),
                    name: "x".into(),
                    ty: TypeDraft::I64
                }],
                result: TypeDraft::I64,
                body: Some(function_body.clone())
            },
            TransactionOp,
            true
        );
        structured_record_count += check_draft_record!(
            "function_parameter",
            FunctionParameterDraft {
                symbol: DraftSymbol::generated(4),
                name: "x".into(),
                ty: TypeDraft::I64
            },
            FunctionParameterDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "function_body",
            function_body.clone(),
            FunctionBodyDraft,
            false
        );
        structured_record_count +=
            check_draft_record!("yielding_body", yielding.clone(), YieldingBodyDraft, false);
        structured_record_count += check_draft_record!(
            "product_field_value",
            ProductFieldValueDraft {
                field: target,
                value: value.clone()
            },
            ProductFieldValueDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "operation_match_arm",
            MatchArmOperationDraft {
                variant: target,
                region: target
            },
            MatchArmOperationDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "match_arm",
            MatchArmDraft {
                variant: target,
                payload_symbol: Some(DraftSymbol::generated(32)),
                body: yielding.clone()
            },
            MatchArmDraft,
            false
        );
        structured_record_count +=
            check_draft_record!("expression", expression.clone(), ExpressionDraft, false);
        structured_record_count += check_draft_record!(
            "define_function_body",
            TransactionOp::DefineFunctionBody {
                function: node,
                body: function_body.clone()
            },
            TransactionOp,
            true
        );
        structured_record_count += check_draft_record!(
            "insert_expression",
            TransactionOp::InsertExpression {
                block: node,
                before: Some(other),
                expression: expression.clone()
            },
            TransactionOp,
            true
        );
        assert_eq!(
            structured_record_count,
            schema.structured_authoring.records.len()
        );

        let transaction = Transaction {
            workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: Some(crate::IdempotencyKey::from_bytes([3; 16])),
            mode: TransactionMode::Commit,
            operations: transaction_samples,
        };
        let response_spec = TransactionResponseSpec {
            return_symbols: vec![DraftSymbol::generated(1)],
        };
        let apply = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: response_spec.clone(),
        };
        let query_batch = QueryBatchRequest {
            workspace,
            revision: Revision::INITIAL,
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::WorkspaceSummary,
            }],
        };
        let run_policy = crate::RunPolicy {
            fuel: 10,
            maximum_frames: 2,
        };
        let run_result = RunResult {
            value: RuntimeValue::I64(1),
            compile_nanoseconds: 1,
            execute_nanoseconds: 2,
        };
        let receipt = TransactionReceipt {
            workspace,
            base_revision: Revision::INITIAL,
            revision: Revision::new(1),
            hash: crate::SnapshotHash::from_bytes([4; 32]),
            published: true,
            created_count: 1,
            returned_bindings: vec![(DraftSymbol::generated(1), node)],
            change_count: 1,
            change_digest: crate::ChangeDigest::from_bytes([5; 32]),
            complete_before: false,
            complete_after: true,
            blocker_count_before: 1,
            blocker_count_after: 0,
        };
        let batch_result = crate::query::QueryBatchResult {
            workspace,
            revision: Revision::INITIAL,
            results: vec![crate::query::QueryItemResult {
                id: QueryId::new(1),
                outcome: crate::query::QueryOutcome::Success(Box::new(QueryResult::NominalType(
                    nominal.clone(),
                ))),
            }],
        };
        macro_rules! check_named_records {
            ($catalog:expr; $($name:literal => $sample:expr),+ $(,)?) => {{
                let mut count = 0_usize;
                $(
                    assert_named_machine_record_serde_contract(&$sample, $catalog, $name);
                    count += 1;
                )+
                count
            }};
        }
        let semantic_record_count = check_named_records!(
            &schema.semantic_records;
            "canonical_product_field_value" => crate::schema::ProductFieldValue { field: node, value: value_ref },
            "canonical_match_arm" => crate::schema::MatchArm { variant: node, region: other },
        );
        assert_eq!(semantic_record_count, schema.semantic_records.len());
        let transaction_record_count = check_named_records!(
            &schema.transaction_records;
            "apply_transaction_request" => ApplyTransactionRequest { transaction: transaction.clone(), response: response_spec.clone() },
            "transaction" => transaction.clone(),
            "transaction_response_spec" => response_spec.clone(),
            "transaction_receipt" => receipt.clone(),
        );
        assert_eq!(transaction_record_count, schema.transaction_records.len());
        let run_record_count = check_named_records!(
            &schema.run.records;
            "run_policy" => run_policy,
            "run_result" => run_result.clone(),
            "runtime_field_value" => RuntimeFieldValue { field: node, value: RuntimeValue::I64(1) },
        );
        assert_machine_record_serde_contract::<RuntimeValue>(
            serde_json::to_value(RuntimeValue::Product {
                ty: node,
                fields: vec![RuntimeFieldValue {
                    field: other,
                    value: RuntimeValue::I64(1),
                }],
            })
            .expect("runtime product data"),
            named_payload(&schema.run.records, "runtime_product_data"),
            true,
        );
        assert_machine_record_serde_contract::<RuntimeValue>(
            serde_json::to_value(RuntimeValue::Sum {
                ty: node,
                variant: other,
                payload: Some(Box::new(RuntimeValue::I64(1))),
            })
            .expect("runtime sum data"),
            named_payload(&schema.run.records, "runtime_sum_data"),
            true,
        );
        assert_eq!(run_record_count + 2, schema.run.records.len());
        let query_item_result = crate::query::QueryItemResult {
            id: QueryId::new(1),
            outcome: crate::query::QueryOutcome::Success(Box::new(QueryResult::WorkspaceSummary(
                summary.clone(),
            ))),
        };
        let layout_with_failure = NominalLayoutSummary {
            representable: false,
            failure: Some(crate::type_layout::LayoutFailure::ByteSizeOverflow),
            size: Some(8),
            align: Some(8),
            cells: Some(1),
            discriminant_bytes: Some(1),
            payload_offset: Some(8),
        };
        let query_record_count = check_named_records!(
            &schema.query_records;
            "query_batch_request" => query_batch.clone(),
            "query_item" => query_batch.queries[0].clone(),
            "query_batch_result" => batch_result.clone(),
            "query_item_result" => query_item_result,
            "workspace_summary" => summary.clone(),
            "function_signature_summary" => signature,
            "name_preview" => name.clone(),
            "node_summary" => node_summary.clone(),
            "node_view" => NodeView { summary: node_summary.clone(), record: Some(crate::schema::Node::WorkspaceRoot { packages: vec![node] }) },
            "completeness_blocker" => blocker.clone(),
            "owner_fact" => owner.clone(),
            "owned_region_summary" => OwnedRegionSummary { region: other, role: RegionRole::IfThen },
            "body_item" => body_item.clone(),
            "use_site" => use_site,
            "definition_reference_site" => definition,
            "visible_value" => visible.clone(),
            "legal_constructors_result" => legal.clone(),
            "context_budget" => ContextBudget { body_before: 1, body_after: 1, visible_values: 1, incoming_uses: 1, include_incompatible: true },
            "semantic_diff_page" => SemanticDiffPage { from: Revision::INITIAL, to: Revision::new(1), change_count: 1, change_digest: crate::ChangeDigest::from_bytes([2; 32]), page: page(cursor, vec![Change { node, kind: ChangeKind::CompletenessChanged { complete: true } }]) },
            "block_argument_fact" => BlockArgumentFact { argument: other, block: other, region: other, ordinal: 0, role: BlockArgumentRole::LoopIndex, ty: SemanticType::I64 },
            "enclosing_region_fact" => EnclosingRegionFact { region: other, owner_operation: node, role: RegionRole::IfThen },
            "repair_context" => repair.clone(),
            "nominal_type_result" => nominal.clone(),
            "nominal_layout_summary" => layout_with_failure,
            "constructor_descriptor" => constructor.clone(),
            "nominal_type_continuation" => crate::query::NominalTypeContinuation { declaration: node, page: PageRequest { after: Some(cursor), limit: 1 } },
            "page_request" => PageRequest { after: Some(cursor), limit: 1 },
            "change" => Change { node, kind: ChangeKind::OperandChanged { index: 0, before: Some(value_ref), after: Some(value_ref) } },
        );
        let generic_page = Page::<u64> {
            items: vec![1],
            next: Some(cursor),
            total: Some(1),
        };
        assert_machine_record_serde_contract_with_parameter::<Page<u64>>(
            serde_json::to_value(generic_page).expect("generic page sample"),
            named_payload(&schema.query_records, "page"),
            false,
            Some("u64"),
        );
        assert_eq!(query_record_count + 1, schema.query_records.len());
        let error_record_count = check_named_records!(
            &schema.error_records;
            "boundary_error" => BoundaryError { kind: BoundaryErrorKind::InvalidJson, message: "bad".into() },
        );
        assert_eq!(error_record_count, schema.error_records.len());

        let manifest = match describe_schema(&DescribeSchemaRequest::manifest()).expect("manifest")
        {
            DescribeSchemaResult::Manifest(value) => value,
            _ => unreachable!(),
        };
        let request_samples = [
            Request::CreateWorkspace,
            Request::ApplyTransaction(apply),
            Request::QueryBatch(query_batch),
            Request::Run {
                workspace,
                revision: Revision::INITIAL,
                entry: node,
                arguments: runtime_samples,
                policy: run_policy,
            },
            Request::Shutdown,
            Request::DescribeSchema(DescribeSchemaRequest {
                projection: SchemaProjection::Full,
                known_digest: Some(active_machine_schema_digest().expect("digest")),
            }),
        ];
        assert_eq!(request_samples.len(), RequestCode::ALL.len());
        for (sample, code) in request_samples.iter().zip(RequestCode::ALL) {
            assert_machine_variant_serde_contract(
                sample,
                &schema.request_payloads,
                code.machine_name(),
            );
        }
        let response_samples = [
            Response::WorkspaceCreated(summary),
            Response::TransactionReceipt(receipt),
            Response::QueryBatchResult(batch_result),
            Response::Run(run_result),
            Response::Acknowledged,
            Response::Error(
                crate::error::LkError::new(ErrorCode::InvalidQuery, "bad")
                    .for_workspace(workspace)
                    .at_revision(Revision::INITIAL)
                    .for_node(node),
            ),
            Response::DescribeSchema(Box::new(DescribeSchemaResult::Manifest(manifest))),
        ];
        assert_eq!(response_samples.len(), ResponseCode::ALL.len());
        for (sample, code) in response_samples.iter().zip(ResponseCode::ALL) {
            assert_machine_variant_serde_contract(
                sample,
                &schema.response_payloads,
                code.machine_name(),
            );
        }
        let envelope_count = check_named_records!(
            &schema.envelopes;
            "request_envelope" => RequestEnvelope { version: JSON_ENVELOPE_VERSION, request_id: RequestId::new(1), request: Request::CreateWorkspace },
            "response_envelope" => ResponseEnvelope { version: JSON_ENVELOPE_VERSION, request_id: RequestId::new(1), response: Response::Acknowledged },
            "boundary_error_envelope" => BoundaryErrorEnvelope { version: JSON_ENVELOPE_VERSION, request_id: Some(RequestId::new(1)), error: BoundaryError { kind: BoundaryErrorKind::InvalidJson, message: "bad".into() } },
        );
        assert_eq!(envelope_count, schema.envelopes.len());
    }

    #[test]
    fn every_advertised_schema_metarecord_matches_strict_serde() {
        let schema = schema_description();
        let digest = machine_schema_digest(&schema).expect("digest");
        let manifest = match describe_schema(&DescribeSchemaRequest::manifest()).expect("manifest")
        {
            DescribeSchemaResult::Manifest(value) => value,
            _ => unreachable!(),
        };
        let catalogue = schema_definition_catalogue(&schema).expect("catalogue");
        let (roots, definitions) =
            project_schema_roots(&catalogue, &[SchemaRoot::Limits]).expect("roots");
        let projected = SchemaDefinitions {
            digest,
            roots,
            type_constructors: schema_type_constructors(),
            definitions,
        };
        macro_rules! check {
            ($name:literal, $sample:expr) => {{
                assert_named_machine_record_serde_contract(
                    &$sample,
                    &schema.schema_discovery.records,
                    $name,
                );
                1_usize
            }};
        }
        let operation = schema
            .operations
            .iter()
            .find(|operation| operation.name == "if")
            .expect("if operation");
        let region = schema
            .operations
            .iter()
            .find(|operation| operation.name == "for_i64")
            .and_then(|operation| operation.regions.first())
            .expect("for region");
        let mut count = 0_usize;
        count += check!(
            "machine_field_description",
            schema.error_payload.fields[0].clone()
        );
        count += check!("payload_shape_description", newtype_payload("node_id"));
        count += check!(
            "variant_payload_description",
            schema.request_payloads[0].clone()
        );
        count += check!("named_payload_description", schema.envelopes[0].clone());
        count += check!(
            "named_variant_description",
            schema.identity_variants[0].clone()
        );
        count += check!("code_description", schema.requests[0].clone());
        count += check!(
            "draft_field_type_description",
            schema.structured_authoring.draft_field_types[0].clone()
        );
        count += check!(
            "draft_field_description",
            schema.structured_authoring.records[0].fields[0].clone()
        );
        count += check!(
            "draft_record_description",
            schema.structured_authoring.records[0].clone()
        );
        count += check!(
            "draft_variant_description",
            schema.structured_authoring.expression_variants[1].clone()
        );
        count += check!(
            "structured_authoring_description",
            schema.structured_authoring.clone()
        );
        count += check!("operand_description", operation.operands[0].clone());
        count += check!(
            "block_argument_description",
            region.block_arguments[0].clone()
        );
        count += check!("region_description", region.clone());
        count += check!("operation_description", operation.clone());
        count += check!("run_field_description", schema.run.fields[0].clone());
        count += check!(
            "runtime_value_description",
            schema.run.runtime_values[3].clone()
        );
        count += check!("run_description", schema.run.clone());
        count += check!(
            "schema_discovery_description",
            schema.schema_discovery.clone()
        );
        count += check!("schema_description", schema.clone());
        count += check!("machine_scalar_description", schema.scalar_types[0].clone());
        count += check!("name_contract_description", schema.name_contract.clone());
        count += check!(
            "name_uniqueness_group_description",
            schema.name_contract.sibling_uniqueness_groups[0].clone()
        );
        count += check!("boundary_limits", schema.limits.clone());
        count += check!("id_formats_description", schema.id_formats.clone());
        count += check!(
            "nominal_declarations_description",
            schema.nominal_declarations.clone()
        );
        count += check!("schema_manifest", manifest);
        count += check!("schema_definitions", projected.clone());
        count += check!("schema_definition", projected.definitions[0].clone());
        count += check!(
            "draft_variant_family_description",
            match catalogue
                .get("expression_kind_draft")
                .expect("expression draft")
                .body
                .clone()
            {
                SchemaDefinitionBody::DraftVariant(value) => value,
                _ => panic!("draft variant body"),
            }
        );
        let endpoint = match catalogue
            .get("query_node")
            .expect("query node")
            .body
            .clone()
        {
            SchemaDefinitionBody::Endpoint(value) => value,
            _ => panic!("endpoint body"),
        };
        count += check!("endpoint_description", endpoint.clone());
        count += check!(
            "endpoint_variant_binding_description",
            endpoint.bindings[0].clone()
        );
        let template = match catalogue
            .get("query_endpoint_protocol")
            .expect("query endpoint template")
            .body
            .clone()
        {
            SchemaDefinitionBody::EndpointTemplate(value) => value,
            _ => panic!("endpoint template body"),
        };
        count += check!("endpoint_protocol_template_description", template.clone());
        count += check!(
            "endpoint_template_parameter_description",
            template.parameters[0].clone()
        );
        count += check!(
            "code_family_description",
            match catalogue.get("node_kind").expect("node kind").body.clone() {
                SchemaDefinitionBody::Codes(value) => value,
                _ => panic!("code family body"),
            }
        );
        count += check!(
            "structured_authoring_policy_description",
            match catalogue
                .get("structured_authoring")
                .expect("structured authoring")
                .body
                .clone()
            {
                SchemaDefinitionBody::StructuredAuthoring(value) => value,
                _ => panic!("structured authoring body"),
            }
        );
        assert_eq!(count, schema.schema_discovery.records.len());
    }

    fn assert_named_machine_record_serde_contract<T>(
        sample: &T,
        records: &[NamedPayloadDescription],
        name: &str,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let payload = named_payload(records, name);
        assert_eq!(payload.shape, PayloadShapeKind::Record, "{name}");
        let value = serde_json::to_value(sample)
            .unwrap_or_else(|error| panic!("{name} serialize: {error}"));
        assert_eq!(
            serde_json::to_value(
                serde_json::from_value::<T>(value.clone())
                    .unwrap_or_else(|error| panic!("{name} decode: {error}"))
            )
            .expect("record reencode"),
            value,
            "{name} round trip"
        );
        assert_machine_record_serde_contract::<T>(value, payload, false);
    }

    fn assert_machine_variant_serde_contract<T>(
        sample: &T,
        variants: &[VariantPayloadDescription],
        name: &str,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let schema = schema_description();
        let description = variants
            .iter()
            .find(|variant| variant.name == name)
            .unwrap_or_else(|| panic!("missing variant {name}"));
        let value = serde_json::to_value(sample)
            .unwrap_or_else(|error| panic!("{name} serialize: {error}"));
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some(name)
        );
        assert_eq!(
            serde_json::to_value(
                serde_json::from_value::<T>(value.clone())
                    .unwrap_or_else(|error| panic!("{name} decode: {error}"))
            )
            .expect("round trip encode"),
            value,
            "{name} round trip"
        );
        let mut unknown = value.clone();
        unknown
            .as_object_mut()
            .expect("tagged object")
            .insert("unknown".into(), serde_json::json!(0));
        assert!(
            serde_json::from_value::<T>(unknown).is_err(),
            "{name} accepted unknown field"
        );
        match description.payload.shape {
            PayloadShapeKind::Unit => assert!(value.get("data").is_none(), "{name}"),
            PayloadShapeKind::Newtype => {
                let data = value.get("data").unwrap_or_else(|| panic!("{name} data"));
                assert_exact_type_expression(
                    &schema,
                    description
                        .payload
                        .newtype
                        .as_deref()
                        .expect("newtype expression"),
                    data,
                    name,
                );
            }
            PayloadShapeKind::Record => {
                assert_machine_record_serde_contract::<T>(value, &description.payload, true)
            }
        }
    }

    fn assert_draft_variant_serde_contract<T>(
        sample: &T,
        schema: &SchemaDescription,
        variants: &[DraftVariantDescription],
        name: &str,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let description = variants
            .iter()
            .find(|variant| variant.name == name)
            .unwrap_or_else(|| panic!("missing draft variant {name}"));
        let value = serde_json::to_value(sample).expect("draft serialize");
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some(name)
        );
        assert_eq!(
            serde_json::to_value(serde_json::from_value::<T>(value.clone()).expect("draft decode"))
                .expect("draft reencode"),
            value,
            "{name} round trip"
        );
        let mut unknown = value.clone();
        unknown
            .as_object_mut()
            .expect("draft tagged object")
            .insert("unknown".into(), serde_json::json!(0));
        assert!(
            serde_json::from_value::<T>(unknown).is_err(),
            "{name} accepted unknown field"
        );
        match description.shape {
            PayloadShapeKind::Unit => assert!(value.get("data").is_none(), "{name}"),
            PayloadShapeKind::Newtype => {
                let expression = &draft_field_type_description(
                    schema,
                    description.newtype.expect("newtype field type"),
                )
                .type_expression;
                assert_exact_type_expression(
                    schema,
                    expression,
                    value.get("data").expect("newtype data"),
                    name,
                );
            }
            PayloadShapeKind::Record => {
                assert_draft_fields_serde_contract::<T>(value, schema, &description.fields, true)
            }
        }
    }

    fn assert_type_draft_serde_contract(
        sample: crate::TypeDraft,
        schema: &SchemaDescription,
        description: &DraftVariantDescription,
    ) {
        let value = serde_json::to_value(sample).expect("type draft serialize");
        let decoded: crate::TypeDraft =
            serde_json::from_value(value.clone()).expect("type draft decode");
        assert_eq!(decoded, sample);
        match description.shape {
            PayloadShapeKind::Unit => {
                assert_eq!(value.as_str(), Some(description.name.as_str()));
                let invalid = serde_json::json!({ description.name.clone(): { "unknown": 0 } });
                assert!(
                    serde_json::from_value::<crate::TypeDraft>(invalid).is_err(),
                    "{} accepted a payload with an unknown field",
                    description.name
                );
            }
            PayloadShapeKind::Newtype => {
                let object = value.as_object().expect("external type object");
                assert_eq!(object.len(), 1);
                let data = object.get(&description.name).expect("external type data");
                let expression = &draft_field_type_description(
                    schema,
                    description.newtype.expect("type newtype"),
                )
                .type_expression;
                assert_exact_type_expression(schema, expression, data, &description.name);
                let mut unknown = value.clone();
                unknown
                    .as_object_mut()
                    .expect("external type object")
                    .insert("unknown".into(), serde_json::json!(0));
                assert!(serde_json::from_value::<crate::TypeDraft>(unknown).is_err());
            }
            PayloadShapeKind::Record => panic!("type draft has no record variant"),
        }
    }

    fn named_payload<'a>(
        records: &'a [NamedPayloadDescription],
        name: &str,
    ) -> &'a PayloadShapeDescription {
        &records
            .iter()
            .find(|record| record.name == name)
            .unwrap_or_else(|| panic!("missing record {name}"))
            .payload
    }

    fn assert_machine_record_serde_contract<T>(
        sample: serde_json::Value,
        payload: &PayloadShapeDescription,
        tagged: bool,
    ) where
        T: serde::de::DeserializeOwned,
    {
        assert_machine_record_serde_contract_with_parameter::<T>(sample, payload, tagged, None);
    }

    fn assert_machine_record_serde_contract_with_parameter<T>(
        sample: serde_json::Value,
        payload: &PayloadShapeDescription,
        tagged: bool,
        type_parameter: Option<&str>,
    ) where
        T: serde::de::DeserializeOwned,
    {
        let schema = schema_description();
        let actual = record_object(&sample, tagged)
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let advertised = payload
            .fields
            .iter()
            .filter(|field| field.required || actual.contains(field.name.as_str()))
            .map(|field| field.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, advertised, "sample contains an unadvertised field");
        let mut unknown = sample.clone();
        record_object_mut(&mut unknown, tagged).insert("unknown".into(), serde_json::json!(0));
        assert!(
            serde_json::from_value::<T>(unknown).is_err(),
            "record accepted an unknown field"
        );
        for field in &payload.fields {
            let mut omitted = sample.clone();
            record_object_mut(&mut omitted, tagged).remove(&field.name);
            if field.required {
                assert!(
                    serde_json::from_value::<T>(omitted).is_err(),
                    "required field {} accepted omission",
                    field.name
                );
            } else {
                assert!(field.type_expression.starts_with("optional<"));
                serde_json::from_value::<T>(omitted)
                    .unwrap_or_else(|error| panic!("optional {} omission: {error}", field.name));
                let mut null = sample.clone();
                record_object_mut(&mut null, tagged)
                    .insert(field.name.clone(), serde_json::Value::Null);
                serde_json::from_value::<T>(null).unwrap_or_else(|error| {
                    panic!("optional {} explicit null: {error}", field.name)
                });
            }
            if let Some(value) = record_object(&sample, tagged).get(&field.name) {
                validate_exact_type_expression(
                    &schema,
                    &field.type_expression,
                    value,
                    type_parameter,
                )
                .unwrap_or_else(|error| {
                    panic!(
                        "{} does not match {}: {error}; {value}",
                        field.name, field.type_expression
                    )
                });
            }
        }
    }

    fn assert_draft_record_serde_contract<T>(
        sample: serde_json::Value,
        schema: &SchemaDescription,
        record_name: &str,
        tagged: bool,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let record = schema
            .structured_authoring
            .records
            .iter()
            .find(|record| record.name == record_name)
            .unwrap_or_else(|| panic!("missing draft record {record_name}"));
        let decoded = serde_json::from_value::<T>(sample.clone())
            .unwrap_or_else(|error| panic!("{record_name} decode: {error}"));
        assert_eq!(
            serde_json::to_value(decoded).expect("draft record reencode"),
            sample,
            "{record_name} round trip"
        );
        assert_draft_fields_serde_contract::<T>(sample, schema, &record.fields, tagged);
    }

    fn assert_draft_fields_serde_contract<T>(
        sample: serde_json::Value,
        schema: &SchemaDescription,
        fields: &[DraftFieldDescription],
        tagged: bool,
    ) where
        T: serde::de::DeserializeOwned,
    {
        let actual = record_object(&sample, tagged)
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let advertised = fields
            .iter()
            .filter(|field| field.required || actual.contains(field.name.as_str()))
            .map(|field| field.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            actual, advertised,
            "draft sample contains an unadvertised field"
        );
        let mut unknown = sample.clone();
        record_object_mut(&mut unknown, tagged).insert("unknown".into(), serde_json::json!(0));
        assert!(
            serde_json::from_value::<T>(unknown).is_err(),
            "draft record accepted an unknown field"
        );
        for field in fields {
            let mapping = draft_field_type_description(schema, field.field_type);
            let mut omitted = sample.clone();
            record_object_mut(&mut omitted, tagged).remove(&field.name);
            if field.required {
                assert!(!field.nullable);
                assert!(
                    serde_json::from_value::<T>(omitted).is_err(),
                    "required draft field {} accepted omission",
                    field.name
                );
            } else {
                assert!(field.nullable);
                serde_json::from_value::<T>(omitted).unwrap_or_else(|error| {
                    panic!("optional draft {} omission: {error}", field.name)
                });
                let mut null = sample.clone();
                record_object_mut(&mut null, tagged)
                    .insert(field.name.clone(), serde_json::Value::Null);
                serde_json::from_value::<T>(null).unwrap_or_else(|error| {
                    panic!("optional draft {} explicit null: {error}", field.name)
                });
            }
            if let Some(value) = record_object(&sample, tagged).get(&field.name) {
                assert_exact_type_expression(schema, &mapping.type_expression, value, &field.name);
            }
        }
    }

    fn record_object(
        value: &serde_json::Value,
        tagged: bool,
    ) -> &serde_json::Map<String, serde_json::Value> {
        let value = if tagged {
            value.get("data").expect("tagged record data")
        } else {
            value
        };
        value.as_object().expect("record object")
    }

    fn record_object_mut(
        value: &mut serde_json::Value,
        tagged: bool,
    ) -> &mut serde_json::Map<String, serde_json::Value> {
        let value = if tagged {
            value.get_mut("data").expect("tagged record data")
        } else {
            value
        };
        value.as_object_mut().expect("record object")
    }

    fn assert_family_samples<T: serde::de::DeserializeOwned + Serialize>(
        schema: &SchemaDescription,
        family_name: &str,
        samples: &[T],
    ) {
        let family = all_variant_families(schema)
            .find(|family| family.name == family_name)
            .unwrap_or_else(|| panic!("missing family {family_name}"));
        assert_eq!(samples.len(), family.variants.len(), "{family_name}");
        let mut names = std::collections::BTreeSet::new();
        for sample in samples {
            let value = serde_json::to_value(sample).expect("variant sample serialization");
            validate_exact_type_expression(schema, family_name, &value, None)
                .unwrap_or_else(|error| panic!("{family_name} sample {value}: {error}"));
            let name = match family.tagging.as_str() {
                "string_enum" => value.as_str(),
                "adjacently_tagged" => value
                    .as_object()
                    .and_then(|object| object.get(family.tag_field.as_deref().expect("tag field")))
                    .and_then(serde_json::Value::as_str),
                "externally_tagged" => value.as_str().or_else(|| {
                    value
                        .as_object()
                        .and_then(|object| object.keys().next().map(String::as_str))
                }),
                other => panic!("unknown family tagging {other}"),
            }
            .unwrap_or_else(|| panic!("cannot extract {family_name} sample name from {value}"));
            let _description = family
                .variants
                .iter()
                .find(|variant| variant.name == name)
                .unwrap_or_else(|| panic!("missing {family_name} descriptor {name}"));
            if family.tagging == "adjacently_tagged" {
                assert_machine_variant_serde_contract(sample, &family.variants, name);
            } else {
                serde_json::from_value::<T>(value.clone())
                    .unwrap_or_else(|error| panic!("{family_name} {name} decode: {error}"));
            }
            assert!(
                names.insert(name.to_owned()),
                "duplicate {family_name} sample {name}"
            );
        }
        assert_eq!(
            names,
            family
                .variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect(),
            "{family_name} sample/descriptor mismatch"
        );
    }

    fn assert_exact_type_expression(
        schema: &SchemaDescription,
        type_expression: &str,
        value: &serde_json::Value,
        field: &str,
    ) {
        validate_exact_type_expression(schema, type_expression, value, None).unwrap_or_else(
            |error| panic!("{field} does not match {type_expression}: {error}; {value}"),
        );
    }

    fn validate_exact_type_expression(
        schema: &SchemaDescription,
        expression: &str,
        value: &serde_json::Value,
        type_parameter: Option<&str>,
    ) -> Result<(), String> {
        if let Some(inner) = wrapped_type(expression, "optional") {
            return if value.is_null() {
                Ok(())
            } else {
                validate_exact_type_expression(schema, inner, value, type_parameter)
            };
        }
        if value.is_null() {
            return Err("non-optional value is null".into());
        }
        if let Some(inner) = wrapped_type(expression, "list") {
            let values = value
                .as_array()
                .ok_or_else(|| "expected JSON array".to_owned())?;
            for item in values {
                validate_exact_type_expression(schema, inner, item, type_parameter)?;
            }
            return Ok(());
        }
        if let Some(inner) = wrapped_type(expression, "tuple") {
            let members = split_type_arguments(inner)?;
            let values = value
                .as_array()
                .ok_or_else(|| "expected JSON tuple array".to_owned())?;
            if values.len() != members.len() {
                return Err(format!(
                    "tuple has {} items, expected {}",
                    values.len(),
                    members.len()
                ));
            }
            for (member, item) in members.into_iter().zip(values) {
                validate_exact_type_expression(schema, member, item, type_parameter)?;
            }
            return Ok(());
        }
        if let Some(inner) = wrapped_type(expression, "page") {
            return validate_named_record(schema, "page", value, Some(inner));
        }
        if expression == "type_parameter" {
            return validate_exact_type_expression(
                schema,
                type_parameter.ok_or_else(|| "unbound type parameter".to_owned())?,
                value,
                None,
            );
        }
        if let Some(scalar) = schema
            .scalar_types
            .iter()
            .find(|scalar| scalar.name == expression)
        {
            return validate_scalar(scalar, value);
        }
        if let Some(record) = all_records(schema).find(|record| record.name == expression) {
            return validate_record_shape(schema, &record.payload, value, false, type_parameter);
        }
        if let Some(record) = schema
            .structured_authoring
            .records
            .iter()
            .find(|record| record.name == expression)
        {
            return validate_draft_record(schema, &record.fields, value);
        }
        if let Some(variants) = draft_variant_family(schema, expression) {
            return validate_draft_variant(schema, expression, variants, value);
        }
        if expression == "describe_schema_request" {
            return validate_record_shape(
                schema,
                &schema.schema_discovery.request,
                value,
                false,
                type_parameter,
            );
        }
        if expression == "error" {
            return validate_record_shape(
                schema,
                &schema.error_payload,
                value,
                false,
                type_parameter,
            );
        }
        if let Some(variants) =
            all_variant_families(schema).find(|family| family.name == expression)
        {
            return validate_variant(schema, variants, value, type_parameter);
        }
        if let Some(codes) = code_family(schema, expression) {
            let name = value
                .as_str()
                .ok_or_else(|| "expected JSON string enum".to_owned())?;
            return codes
                .contains(&name)
                .then_some(())
                .ok_or_else(|| format!("unknown {expression} value `{name}`"));
        }
        Err(format!("unresolved production schema type `{expression}`"))
    }

    fn wrapped_type<'a>(expression: &'a str, wrapper: &str) -> Option<&'a str> {
        expression
            .strip_prefix(wrapper)
            .and_then(|rest| rest.strip_prefix('<'))
            .and_then(|rest| rest.strip_suffix('>'))
    }

    fn split_type_arguments(expression: &str) -> Result<Vec<&str>, String> {
        let mut depth = 0_usize;
        let mut start = 0_usize;
        let mut output = Vec::new();
        for (index, byte) in expression.bytes().enumerate() {
            match byte {
                b'<' => depth += 1,
                b'>' => {
                    depth = depth
                        .checked_sub(1)
                        .ok_or_else(|| "unbalanced type expression".to_owned())?;
                }
                b',' if depth == 0 => {
                    output.push(&expression[start..index]);
                    start = index + 1;
                }
                _ => {}
            }
        }
        if depth != 0 || start == expression.len() {
            return Err("malformed type argument list".into());
        }
        output.push(&expression[start..]);
        Ok(output)
    }

    fn validate_scalar(
        scalar: &MachineScalarDescription,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let actual_kind = if value.is_boolean() {
            JsonScalarKind::Boolean
        } else if value.is_number() {
            JsonScalarKind::Number
        } else if value.is_string() {
            JsonScalarKind::String
        } else {
            return Err("expected a scalar JSON value".into());
        };
        if actual_kind != scalar.json_kind {
            return Err(format!(
                "JSON kind is {actual_kind:?}, expected {:?}",
                scalar.json_kind
            ));
        }
        match &scalar.domain {
            MachineScalarDomain::Boolean => value
                .as_bool()
                .map(|_| ())
                .ok_or_else(|| "expected boolean".into()),
            MachineScalarDomain::Utf8String => value
                .as_str()
                .map(|_| ())
                .ok_or_else(|| "expected UTF-8 string".into()),
            MachineScalarDomain::CanonicalIdentifier {
                grammar: _,
                minimum_utf8_bytes,
                maximum_utf8_bytes,
            } => {
                let text = value.as_str().ok_or_else(|| "expected string".to_owned())?;
                let length = u64::try_from(text.len())
                    .map_err(|_| "identifier length overflow".to_owned())?;
                if length < *minimum_utf8_bytes || length > *maximum_utf8_bytes {
                    return Err("canonical identifier length is outside policy".to_owned());
                }
                DraftSymbol::parse(text).map(|_| ()).map_err(str::to_owned)
            }
            MachineScalarDomain::SignedInteger { minimum, maximum } => {
                let number = value
                    .as_i64()
                    .ok_or_else(|| "expected signed integer".to_owned())?;
                (*minimum <= number && number <= *maximum)
                    .then_some(())
                    .ok_or_else(|| format!("signed integer outside {minimum}..={maximum}"))
            }
            MachineScalarDomain::UnsignedInteger { minimum, maximum } => {
                let number = value
                    .as_u64()
                    .ok_or_else(|| "expected unsigned integer".to_owned())?;
                (*minimum <= number && number <= *maximum)
                    .then_some(())
                    .ok_or_else(|| format!("unsigned integer outside {minimum}..={maximum}"))
            }
            MachineScalarDomain::LowercaseHex { encoded_bytes } => {
                let text = value.as_str().ok_or_else(|| "expected string".to_owned())?;
                let expected_length = usize::from(*encoded_bytes)
                    .checked_mul(2)
                    .ok_or_else(|| "hex format length overflow".to_owned())?;
                if text.len() != expected_length
                    || !text
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
                {
                    return Err(format!(
                        "expected {encoded_bytes} bytes of canonical lowercase hexadecimal"
                    ));
                }
                let parsed = match scalar.name.as_str() {
                    "workspace_id" => text.parse::<crate::WorkspaceId>().is_ok(),
                    "idempotency_key" => text.parse::<crate::IdempotencyKey>().is_ok(),
                    "snapshot_hash" => text.parse::<crate::SnapshotHash>().is_ok(),
                    "change_digest" => text.parse::<crate::ChangeDigest>().is_ok(),
                    "machine_schema_digest" => text.parse::<MachineSchemaDigest>().is_ok(),
                    _ => return Err(format!("no canonical parser for `{}`", scalar.name)),
                };
                parsed.then_some(()).ok_or_else(|| {
                    format!(
                        "expected {} lowercase hexadecimal bytes for `{}`",
                        encoded_bytes, scalar.name
                    )
                })
            }
            MachineScalarDomain::CanonicalUrlSafeBase64 {
                padding,
                whitespace,
                canonical_trailing_bits,
                maximum_decoded_bytes,
                maximum_encoded_bytes,
            } => {
                if *padding || *whitespace || !*canonical_trailing_bits {
                    return Err("machine bytes scalar describes a noncanonical policy".into());
                }
                let text = value.as_str().ok_or_else(|| "expected string".to_owned())?;
                if u64::try_from(text.len()).unwrap_or(u64::MAX) > *maximum_encoded_bytes {
                    return Err("encoded byte string exceeds policy".into());
                }
                let parsed = serde_json::from_value::<crate::schema::ByteString>(value.clone())
                    .map_err(|error| error.to_string())?;
                if u64::try_from(parsed.len()).unwrap_or(u64::MAX) > *maximum_decoded_bytes {
                    return Err("decoded byte string exceeds policy".into());
                }
                Ok(())
            }
            MachineScalarDomain::NodeId {
                workspace_bytes,
                minimum_serial,
                maximum_serial,
            } => {
                let text = value.as_str().ok_or_else(|| "expected string".to_owned())?;
                let (workspace, _) = text
                    .split_once(':')
                    .ok_or_else(|| "node ID has no separator".to_owned())?;
                if workspace.len() != usize::from(*workspace_bytes) * 2 {
                    return Err("node ID workspace has the wrong encoded length".into());
                }
                let parsed = text
                    .parse::<crate::NodeId>()
                    .map_err(|error| error.to_string())?;
                if parsed.to_string() != text {
                    return Err("node ID is not in canonical form".into());
                }
                (*minimum_serial <= parsed.serial() && parsed.serial() <= *maximum_serial)
                    .then_some(())
                    .ok_or_else(|| {
                        format!("node serial outside {minimum_serial}..={maximum_serial}")
                    })
            }
        }
    }

    fn validate_draft_record(
        schema: &SchemaDescription,
        fields: &[DraftFieldDescription],
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let object = value
            .as_object()
            .ok_or_else(|| "expected structured record object".to_owned())?;
        for key in object.keys() {
            if !fields.iter().any(|field| field.name == *key) {
                return Err(format!("unknown structured record field `{key}`"));
            }
        }
        for field in fields {
            match object.get(&field.name) {
                Some(value) if value.is_null() && field.nullable => {}
                Some(value) => {
                    let expression =
                        &draft_field_type_description(schema, field.field_type).type_expression;
                    validate_exact_type_expression(schema, expression, value, None)?;
                }
                None if field.required => {
                    return Err(format!(
                        "missing required structured field `{}`",
                        field.name
                    ));
                }
                None => {}
            }
        }
        Ok(())
    }

    fn draft_variant_family<'a>(
        schema: &'a SchemaDescription,
        expression: &str,
    ) -> Option<&'a [DraftVariantDescription]> {
        match expression {
            "expression_kind_draft" => Some(&schema.structured_authoring.expression_variants),
            "operation_draft" => Some(&schema.structured_authoring.operation_variants),
            "value_draft" => Some(&schema.structured_authoring.value_variants),
            "type_draft" => Some(&schema.structured_authoring.type_variants),
            _ => None,
        }
    }

    fn validate_draft_variant(
        schema: &SchemaDescription,
        family: &str,
        variants: &[DraftVariantDescription],
        value: &serde_json::Value,
    ) -> Result<(), String> {
        let external = family == "type_draft";
        let (name, payload) = if external {
            if let Some(name) = value.as_str() {
                (name, None)
            } else {
                let object = value
                    .as_object()
                    .ok_or_else(|| "expected external draft variant".to_owned())?;
                if object.len() != 1 {
                    return Err("external draft variant must have one key".into());
                }
                let (name, payload) = object.iter().next().expect("one external draft key");
                (name.as_str(), Some(payload))
            }
        } else {
            let object = value
                .as_object()
                .ok_or_else(|| "expected adjacent draft variant".to_owned())?;
            for key in object.keys() {
                if key != "kind" && key != "data" {
                    return Err(format!("unknown draft variant field `{key}`"));
                }
            }
            (
                object
                    .get("kind")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| "missing draft variant kind".to_owned())?,
                object.get("data"),
            )
        };
        let variant = variants
            .iter()
            .find(|variant| variant.name == name)
            .ok_or_else(|| format!("unknown {family} variant `{name}`"))?;
        match variant.shape {
            PayloadShapeKind::Unit => payload
                .is_none()
                .then_some(())
                .ok_or_else(|| "unit draft variant has content".into()),
            PayloadShapeKind::Newtype => {
                let expression = &draft_field_type_description(
                    schema,
                    variant
                        .newtype
                        .ok_or_else(|| "draft newtype has no type".to_owned())?,
                )
                .type_expression;
                validate_exact_type_expression(
                    schema,
                    expression,
                    payload.ok_or_else(|| "draft newtype is missing content".to_owned())?,
                    None,
                )
            }
            PayloadShapeKind::Record => validate_draft_record(
                schema,
                &variant.fields,
                payload.ok_or_else(|| "draft record is missing content".to_owned())?,
            ),
        }
    }

    fn validate_named_record(
        schema: &SchemaDescription,
        name: &str,
        value: &serde_json::Value,
        type_parameter: Option<&str>,
    ) -> Result<(), String> {
        let record = all_records(schema)
            .find(|record| record.name == name)
            .ok_or_else(|| format!("missing production record `{name}`"))?;
        validate_record_shape(schema, &record.payload, value, false, type_parameter)
    }

    fn validate_record_shape(
        schema: &SchemaDescription,
        shape: &PayloadShapeDescription,
        value: &serde_json::Value,
        tagged: bool,
        type_parameter: Option<&str>,
    ) -> Result<(), String> {
        let value = if tagged {
            value
                .get("data")
                .ok_or_else(|| "missing adjacent content field `data`".to_owned())?
        } else {
            value
        };
        let object = value
            .as_object()
            .ok_or_else(|| "expected JSON record object".to_owned())?;
        for key in object.keys() {
            if !shape.fields.iter().any(|field| field.name == *key) {
                return Err(format!("unknown record field `{key}`"));
            }
        }
        for field in &shape.fields {
            match object.get(&field.name) {
                Some(field_value) => validate_exact_type_expression(
                    schema,
                    &field.type_expression,
                    field_value,
                    type_parameter,
                )
                .map_err(|error| format!("field `{}`: {error}", field.name))?,
                None if field.required => {
                    return Err(format!("missing required field `{}`", field.name));
                }
                None => {}
            }
        }
        Ok(())
    }

    fn validate_variant(
        schema: &SchemaDescription,
        family: &NamedVariantDescription,
        value: &serde_json::Value,
        type_parameter: Option<&str>,
    ) -> Result<(), String> {
        let (name, payload) = match family.tagging.as_str() {
            "string_enum" => (
                value
                    .as_str()
                    .ok_or_else(|| "expected string enum".to_owned())?,
                None,
            ),
            "adjacently_tagged" => {
                let object = value
                    .as_object()
                    .ok_or_else(|| "expected adjacently tagged object".to_owned())?;
                let tag_field = family
                    .tag_field
                    .as_deref()
                    .ok_or_else(|| "missing production tag field".to_owned())?;
                let content_field = family
                    .content_field
                    .as_deref()
                    .ok_or_else(|| "missing production content field".to_owned())?;
                for key in object.keys() {
                    if key != tag_field && key != content_field {
                        return Err(format!("unknown adjacent variant field `{key}`"));
                    }
                }
                (
                    object
                        .get(tag_field)
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| format!("missing string tag field `{tag_field}`"))?,
                    object.get(content_field),
                )
            }
            "externally_tagged" => {
                if let Some(name) = value.as_str() {
                    (name, None)
                } else {
                    let object = value
                        .as_object()
                        .ok_or_else(|| "expected externally tagged value".to_owned())?;
                    if object.len() != 1 {
                        return Err("external variant must contain exactly one tag".into());
                    }
                    let (name, payload) = object.iter().next().expect("one external tag");
                    (name.as_str(), Some(payload))
                }
            }
            other => return Err(format!("unknown production tagging convention `{other}`")),
        };
        let variant = family
            .variants
            .iter()
            .find(|variant| variant.name == name)
            .ok_or_else(|| format!("unknown {} variant `{name}`", family.name))?;
        match variant.payload.shape {
            PayloadShapeKind::Unit => {
                if payload.is_some() {
                    return Err("unit variant unexpectedly has content".into());
                }
            }
            PayloadShapeKind::Newtype => validate_exact_type_expression(
                schema,
                variant
                    .payload
                    .newtype
                    .as_deref()
                    .ok_or_else(|| "newtype descriptor has no type".to_owned())?,
                payload.ok_or_else(|| "newtype variant is missing content".to_owned())?,
                type_parameter,
            )?,
            PayloadShapeKind::Record => {
                if family.tagging != "adjacently_tagged" {
                    return Err("record payload requires adjacent tagging".into());
                }
                validate_record_shape(schema, &variant.payload, value, true, type_parameter)?;
            }
        }
        Ok(())
    }

    fn all_records(schema: &SchemaDescription) -> impl Iterator<Item = &NamedPayloadDescription> {
        schema
            .schema_discovery
            .records
            .iter()
            .chain(&schema.semantic_records)
            .chain(&schema.transaction_records)
            .chain(&schema.run.records)
            .chain(&schema.query_records)
            .chain(&schema.error_records)
            .chain(&schema.envelopes)
    }

    fn all_variant_families(
        schema: &SchemaDescription,
    ) -> impl Iterator<Item = &NamedVariantDescription> {
        schema
            .schema_discovery
            .variants
            .iter()
            .chain(&schema.semantic_variants)
            .chain(&schema.transaction_variants)
            .chain(&schema.run.variants)
            .chain(&schema.query_variants)
            .chain(&schema.error_variants)
            .chain(&schema.identity_variants)
    }

    fn code_family<'a>(schema: &'a SchemaDescription, expression: &str) -> Option<Vec<&'a str>> {
        let codes = match expression {
            "node_kind" => schema
                .node_kinds
                .iter()
                .map(|code| code.name.as_str())
                .collect(),
            "operation_code" => schema
                .operations
                .iter()
                .map(|operation| operation.name.as_str())
                .collect(),
            "transaction_operation_code" => schema
                .transaction_operations
                .iter()
                .map(|code| code.name.as_str())
                .collect(),
            "error_code" => schema
                .errors
                .iter()
                .map(|code| code.name.as_str())
                .collect(),
            _ => return None,
        };
        Some(codes)
    }

    #[test]
    fn production_schema_catalogue_is_unique_closed_and_strict() {
        let schema = schema_description();
        let catalogue = schema_definition_catalogue(&schema).expect("closed catalogue");
        assert!(catalogue.len() > 100);
        for (name, definition) in &catalogue {
            assert_eq!(name, &definition.name);
            assert!(
                definition
                    .dependencies
                    .windows(2)
                    .all(|pair| pair[0] < pair[1])
            );
            for dependency in &definition.dependencies {
                assert!(
                    catalogue.contains_key(dependency),
                    "{name} has missing dependency {dependency}"
                );
            }
        }
        for invalid in [
            "",
            "list",
            "list<>",
            "list<i64,bool>",
            "tuple<>",
            "tuple<i64,>",
            "page<i64",
            "optional<i64>>",
            "I64",
            "i64 i64",
        ] {
            assert!(
                type_expression_dependencies(invalid).is_err(),
                "accepted malformed type expression {invalid}"
            );
        }
    }

    fn projected_record<'a>(
        definitions: &'a [SchemaDefinition],
        name: &str,
    ) -> &'a NamedPayloadDescription {
        match &definitions
            .iter()
            .find(|definition| definition.name == name)
            .unwrap_or_else(|| panic!("missing projected record {name}"))
            .body
        {
            SchemaDefinitionBody::Record(record) => record,
            _ => panic!("projected definition {name} is not a record"),
        }
    }

    fn projected_template<'a>(
        definitions: &'a [SchemaDefinition],
        name: &str,
    ) -> &'a EndpointProtocolTemplateDescription {
        match &definitions
            .iter()
            .find(|definition| definition.name == name)
            .unwrap_or_else(|| panic!("missing projected endpoint template {name}"))
            .body
        {
            SchemaDefinitionBody::EndpointTemplate(template) => template,
            _ => panic!("projected definition {name} is not an endpoint template"),
        }
    }

    fn template_record<'a>(
        template: &'a EndpointProtocolTemplateDescription,
        name: &str,
    ) -> &'a NamedPayloadDescription {
        template
            .records
            .iter()
            .find(|record| record.name == name)
            .unwrap_or_else(|| panic!("missing template record {name}"))
    }

    fn template_variant<'a>(
        template: &'a EndpointProtocolTemplateDescription,
        name: &str,
    ) -> &'a NamedVariantDescription {
        template
            .variants
            .iter()
            .find(|variant| variant.name == name)
            .unwrap_or_else(|| panic!("missing template variant {name}"))
    }

    fn endpoint_binding_variant<'a>(
        endpoint: &'a EndpointDescription,
        parameter: &str,
    ) -> &'a VariantPayloadDescription {
        &endpoint
            .bindings
            .iter()
            .find(|binding| binding.parameter == parameter)
            .unwrap_or_else(|| panic!("missing endpoint binding {parameter}"))
            .variant
    }

    fn assert_json_object_fields(value: &serde_json::Value, payload: &PayloadShapeDescription) {
        let object = value.as_object().expect("advertised record JSON object");
        let actual = object.keys().map(String::as_str).collect::<BTreeSet<_>>();
        let expected = payload
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn endpoint_roots_are_wire_complete_and_match_real_envelopes() {
        let workspace = WorkspaceId::from_bytes([0x31; 16]);
        let root = NodeId::new(workspace, 1).expect("root ID");
        let summary = WorkspaceSummary {
            workspace,
            revision: Revision::INITIAL,
            hash: crate::SnapshotHash::from_bytes([0x42; 32]),
            root,
            node_count: 1,
            complete: true,
            blocker_count: 0,
            entry_count: 0,
        };

        for (request_code, response_code) in [
            (RequestCode::CreateWorkspace, ResponseCode::WorkspaceCreated),
            (
                RequestCode::ApplyTransaction,
                ResponseCode::TransactionReceipt,
            ),
            (RequestCode::Run, ResponseCode::Run),
            (RequestCode::Shutdown, ResponseCode::Acknowledged),
            (RequestCode::DescribeSchema, ResponseCode::DescribeSchema),
        ] {
            let endpoint_name = request_code.machine_name();
            let root = SchemaRoot::ALL
                .into_iter()
                .find(|root| root.machine_name() == endpoint_name)
                .expect("control endpoint root");
            let DescribeSchemaResult::Roots(projected) = describe_schema(&DescribeSchemaRequest {
                projection: SchemaProjection::Roots { roots: vec![root] },
                known_digest: None,
            })
            .expect("control endpoint projection") else {
                panic!("control endpoint roots result")
            };
            let endpoint = match &projected
                .definitions
                .iter()
                .find(|definition| definition.name == endpoint_name)
                .expect("control endpoint definition")
                .body
            {
                SchemaDefinitionBody::Endpoint(endpoint) => endpoint,
                _ => panic!("control root is not an endpoint"),
            };
            assert_eq!(endpoint.template, "control_endpoint_protocol");
            assert_eq!(
                endpoint_binding_variant(endpoint, "request_variant").name,
                request_code.machine_name()
            );
            assert_eq!(
                endpoint_binding_variant(endpoint, "success_response_variant").name,
                response_code.machine_name()
            );
            let template = projected_template(&projected.definitions, &endpoint.template);
            assert_eq!(
                template
                    .parameters
                    .iter()
                    .map(|parameter| (parameter.name.as_str(), parameter.target_variant.as_str()))
                    .collect::<Vec<_>>(),
                vec![
                    ("request_variant", "request"),
                    ("success_response_variant", "response")
                ]
            );
            assert!(template_variant(template, "request").variants.is_empty());
            assert_eq!(
                template_variant(template, "response")
                    .variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<Vec<_>>(),
                vec![ResponseCode::Error.machine_name()]
            );
            assert_eq!(endpoint.protocol_version, PROTOCOL_VERSION);
            assert_eq!(endpoint.json_envelope_version, JSON_ENVELOPE_VERSION);
            let names = projected
                .definitions
                .iter()
                .map(|definition| definition.name.as_str())
                .collect::<BTreeSet<_>>();
            for dependency in [
                endpoint.template.as_str(),
                endpoint.boundary_error_envelope.as_str(),
                endpoint.typed_error.as_str(),
                endpoint.id_formats.as_str(),
                endpoint.limits.as_str(),
                "boundary_error",
                "boundary_error_kind",
            ] {
                assert!(
                    names.contains(dependency),
                    "missing control wire fact {dependency}"
                );
            }
        }

        for code in QueryCode::ALL {
            let endpoint_name = format!("query_{}", code.machine_name());
            let root = SchemaRoot::ALL
                .into_iter()
                .find(|root| root.machine_name() == endpoint_name)
                .expect("query endpoint root");
            let DescribeSchemaResult::Roots(projected) = describe_schema(&DescribeSchemaRequest {
                projection: SchemaProjection::Roots { roots: vec![root] },
                known_digest: None,
            })
            .expect("query endpoint projection") else {
                panic!("query endpoint roots result")
            };
            let names = projected
                .definitions
                .iter()
                .map(|definition| definition.name.as_str())
                .collect::<BTreeSet<_>>();
            let endpoint_definition = projected
                .definitions
                .iter()
                .find(|definition| definition.name == endpoint_name)
                .expect("endpoint definition");
            let endpoint = match &endpoint_definition.body {
                SchemaDefinitionBody::Endpoint(endpoint) => endpoint,
                _ => panic!("root is not an endpoint"),
            };
            assert_eq!(endpoint.template, "query_endpoint_protocol");
            assert_eq!(endpoint.protocol_version, PROTOCOL_VERSION);
            assert_eq!(endpoint.json_envelope_version, JSON_ENVELOPE_VERSION);
            assert_eq!(endpoint.id_formats, "id_formats");
            assert_eq!(endpoint.limits, "limits");
            assert_eq!(endpoint.typed_error, "error");
            assert_eq!(
                endpoint_binding_variant(endpoint, "query_variant").name,
                code.machine_name()
            );
            assert_eq!(
                endpoint_binding_variant(endpoint, "query_result_variant").name,
                code.machine_name()
            );
            let template = projected_template(&projected.definitions, &endpoint.template);
            assert_eq!(
                template
                    .parameters
                    .iter()
                    .map(|parameter| (parameter.name.as_str(), parameter.target_variant.as_str()))
                    .collect::<Vec<_>>(),
                vec![
                    ("query_variant", "query"),
                    ("query_result_variant", "query_result")
                ]
            );
            assert_eq!(
                template_variant(template, "request")
                    .variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<Vec<_>>(),
                vec![RequestCode::QueryBatch.machine_name()]
            );
            assert_eq!(
                template_variant(template, "response")
                    .variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<Vec<_>>(),
                vec![
                    ResponseCode::QueryBatchResult.machine_name(),
                    ResponseCode::Error.machine_name()
                ]
            );
            assert!(template_variant(template, "query").variants.is_empty());
            assert!(
                template_variant(template, "query_result")
                    .variants
                    .is_empty()
            );
            assert_eq!(
                template_variant(template, "query_outcome")
                    .variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["success", "error"]
            );
            assert_eq!(
                template_record(template, "query_batch_request")
                    .payload
                    .fields[2]
                    .type_expression,
                "list<query_item>"
            );
            assert_eq!(
                template_record(template, "query_item").payload.fields[1].type_expression,
                "query"
            );
            assert_eq!(
                template_record(template, "query_batch_result")
                    .payload
                    .fields[2]
                    .type_expression,
                "list<query_item_result>"
            );
            assert_eq!(
                template_record(template, "query_item_result")
                    .payload
                    .fields[1]
                    .type_expression,
                "query_outcome"
            );
            for shared in [
                "boundary_error_envelope",
                "boundary_error",
                "boundary_error_kind",
                "error",
                "error_code",
                "id_formats",
                "limits",
                "workspace_id",
                "request_id",
                "query_id",
                "revision",
            ] {
                assert!(names.contains(shared), "missing shared wire fact {shared}");
            }
        }

        let DescribeSchemaResult::Roots(projected) = describe_schema(&DescribeSchemaRequest {
            projection: SchemaProjection::Roots {
                roots: vec![SchemaRoot::QueryWorkspaceSummary],
            },
            known_digest: None,
        })
        .expect("workspace summary endpoint") else {
            panic!("workspace endpoint roots result")
        };
        let template = projected_template(&projected.definitions, "query_endpoint_protocol");
        let request = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(7),
            request: Request::QueryBatch(QueryBatchRequest {
                workspace,
                revision: Revision::INITIAL,
                queries: vec![QueryItem {
                    id: QueryId::new(9),
                    query: Query::WorkspaceSummary,
                }],
            }),
        };
        let request_json = serde_json::to_value(&request).expect("request envelope JSON");
        assert_json_object_fields(
            &request_json,
            &template_record(template, "request_envelope").payload,
        );
        assert_eq!(
            request_json
                .pointer("/request/kind")
                .and_then(serde_json::Value::as_str),
            Some(RequestCode::QueryBatch.machine_name())
        );
        assert_json_object_fields(
            request_json
                .pointer("/request/data")
                .expect("request batch JSON"),
            &template_record(template, "query_batch_request").payload,
        );
        assert_json_object_fields(
            request_json
                .pointer("/request/data/queries/0")
                .expect("request item JSON"),
            &template_record(template, "query_item").payload,
        );
        assert_eq!(
            request_json
                .pointer("/request/data/queries/0/query/kind")
                .and_then(serde_json::Value::as_str),
            Some(QueryCode::WorkspaceSummary.machine_name())
        );

        let response = ResponseEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(7),
            response: Response::QueryBatchResult(QueryBatchResult {
                workspace,
                revision: Revision::INITIAL,
                results: vec![
                    QueryItemResult {
                        id: QueryId::new(9),
                        outcome: QueryOutcome::Success(Box::new(QueryResult::WorkspaceSummary(
                            summary,
                        ))),
                    },
                    QueryItemResult {
                        id: QueryId::new(10),
                        outcome: QueryOutcome::Error(crate::LkError::new(
                            ErrorCode::InvalidQuery,
                            "bad query",
                        )),
                    },
                ],
            }),
        };
        let response_json = serde_json::to_value(&response).expect("response envelope JSON");
        assert_json_object_fields(
            &response_json,
            &template_record(template, "response_envelope").payload,
        );
        assert_eq!(
            response_json
                .pointer("/response/kind")
                .and_then(serde_json::Value::as_str),
            Some(ResponseCode::QueryBatchResult.machine_name())
        );
        assert_json_object_fields(
            response_json
                .pointer("/response/data")
                .expect("response batch JSON"),
            &template_record(template, "query_batch_result").payload,
        );
        assert_json_object_fields(
            response_json
                .pointer("/response/data/results/0")
                .expect("response item JSON"),
            &template_record(template, "query_item_result").payload,
        );
        assert_eq!(
            response_json
                .pointer("/response/data/results/0/outcome/kind")
                .and_then(serde_json::Value::as_str),
            Some("success")
        );
        assert_eq!(
            response_json
                .pointer("/response/data/results/0/outcome/data/kind")
                .and_then(serde_json::Value::as_str),
            Some(QueryCode::WorkspaceSummary.machine_name())
        );
        assert_eq!(
            response_json
                .pointer("/response/data/results/1/outcome/kind")
                .and_then(serde_json::Value::as_str),
            Some("error")
        );

        let typed_error = ResponseEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(7),
            response: Response::Error(crate::LkError::new(ErrorCode::InvalidQuery, "bad batch")),
        };
        assert_eq!(
            serde_json::to_value(typed_error)
                .expect("typed error JSON")
                .pointer("/response/kind")
                .and_then(serde_json::Value::as_str),
            Some(ResponseCode::Error.machine_name())
        );
        let boundary = BoundaryErrorEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: Some(RequestId::new(7)),
            error: BoundaryError {
                kind: BoundaryErrorKind::InvalidJson,
                message: "bad JSON".to_owned(),
            },
        };
        let boundary_json = serde_json::to_value(boundary).expect("boundary error JSON");
        assert_json_object_fields(
            &boundary_json,
            &projected_record(&projected.definitions, "boundary_error_envelope").payload,
        );
        assert_eq!(
            boundary_json
                .pointer("/error/kind")
                .and_then(serde_json::Value::as_str),
            Some(BoundaryErrorKind::InvalidJson.machine_name())
        );
    }

    #[test]
    fn endpoint_template_parameters_are_explicit_and_locally_closed() {
        let schema = schema_description();
        let templates = endpoint_protocol_templates(&schema).expect("endpoint templates");
        assert_eq!(templates.len(), 2);
        for template in templates {
            let local_names = template
                .records
                .iter()
                .map(|record| record.name.as_str())
                .chain(
                    template
                        .variants
                        .iter()
                        .map(|variant| variant.name.as_str()),
                )
                .collect::<BTreeSet<_>>();
            let dependencies =
                definition_dependencies(&SchemaDefinitionBody::EndpointTemplate(template.clone()))
                    .expect("template dependencies");
            assert!(
                dependencies
                    .iter()
                    .all(|dependency| !local_names.contains(dependency.as_str()))
            );
            assert!(template.parameters.iter().all(|parameter| {
                template
                    .variants
                    .iter()
                    .any(|variant| variant.name == parameter.target_variant)
                    && !parameter.semantics.is_empty()
            }));

            let mut invalid = template;
            invalid.parameters[0].target_variant = "unresolved_context".to_owned();
            assert!(
                definition_dependencies(&SchemaDefinitionBody::EndpointTemplate(invalid)).is_err()
            );
        }
    }

    #[test]
    fn root_projection_is_transitively_closed_unique_and_canonical() {
        let schema = schema_description();
        let catalogue = schema_definition_catalogue(&schema).expect("catalogue");
        let (left_roots, left) = project_schema_roots(
            &catalogue,
            &[SchemaRoot::QueryNode, SchemaRoot::ApplyTransaction],
        )
        .expect("left projection");
        let (right_roots, right) = project_schema_roots(
            &catalogue,
            &[SchemaRoot::ApplyTransaction, SchemaRoot::QueryNode],
        )
        .expect("right projection");
        assert_eq!(left_roots, right_roots);
        assert_eq!(left, right);
        assert!(left.windows(2).all(|pair| pair[0].name < pair[1].name));
        let names = left
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), left.len());
        for definition in &left {
            for dependency in &definition.dependencies {
                assert!(names.contains(dependency.as_str()));
            }
        }
        assert!(names.contains("apply_transaction_request"));
        assert!(names.contains("control_endpoint_protocol"));
        assert!(names.contains("query_endpoint_protocol"));
        assert!(names.contains("query_node"));
        assert!(names.contains("boundary_error_envelope"));
    }

    fn draft_field_type_description(
        schema: &SchemaDescription,
        field_type: DraftFieldType,
    ) -> &DraftFieldTypeDescription {
        let matches = schema
            .structured_authoring
            .draft_field_types
            .iter()
            .filter(|description| description.name == field_type.machine_name())
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1, "exact draft field type mapping");
        let description = matches[0];
        assert_eq!(description.name, field_type.machine_name());
        description
    }

    #[test]
    fn name_and_artifact_policies_are_exact_and_validator_derived() {
        let schema = schema_description();
        assert_eq!(
            schema.name_contract.named_node_kinds,
            vec![
                NodeKind::Package,
                NodeKind::Module,
                NodeKind::ProductType,
                NodeKind::SumType,
                NodeKind::Function,
                NodeKind::ProductField,
                NodeKind::SumVariant,
                NodeKind::Parameter,
            ]
        );
        assert_eq!(
            schema.name_contract.minimum_utf8_bytes,
            crate::schema::MINIMUM_NAME_UTF8_BYTES as u64
        );
        assert_eq!(
            schema.name_contract.maximum_utf8_bytes,
            crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64
        );
        assert_eq!(
            schema
                .name_contract
                .sibling_uniqueness_groups
                .iter()
                .map(|group| (
                    group.name.as_str(),
                    group.owner_kind,
                    group.member_kinds.as_slice()
                ))
                .collect::<Vec<_>>(),
            crate::schema::NameUniquenessGroup::ALL
                .into_iter()
                .map(|group| (
                    group.machine_name(),
                    group.owner_kind(),
                    group.member_kinds()
                ))
                .collect::<Vec<_>>()
        );
        assert_eq!(
            schema.limits.maximum_artifact_bytes,
            crate::artifact::MAXIMUM_ARTIFACT_BYTES as u64
        );
        assert_eq!(
            schema.limits.maximum_artifact_name_bytes,
            crate::artifact::MAXIMUM_ARTIFACT_NAME_BYTES as u64
        );
        let catalogue = schema_definition_catalogue(&schema).expect("catalogue");
        assert!(matches!(
            &catalogue.get("name_contract").expect("name contract").body,
            SchemaDefinitionBody::NameContract(value) if value == &schema.name_contract
        ));
        assert!(matches!(
            &catalogue.get("limits").expect("limits").body,
            SchemaDefinitionBody::Limits(value) if value == &schema.limits
        ));
    }

    #[test]
    fn schema_is_deterministic_complete_and_unique() {
        let first = schema_description();
        assert_eq!(first, schema_description());
        assert_eq!(first.operations.len(), OperationCode::ALL.len());
        assert_named_variant_counts(
            &first.semantic_variants,
            &[
                ("semantic_type", 5),
                ("value_ref", 3),
                ("region_role", 4),
                ("operation_kind", OperationCode::ALL.len()),
                ("node", NodeKind::ALL.len()),
            ],
        );
        assert_named_variant_counts(
            &first.transaction_variants,
            &[
                ("transaction_operation", TransactionOpCode::ALL.len()),
                ("node_target", 2),
                ("transaction_mode", 2),
            ],
        );
        assert_named_variant_counts(
            &first.query_variants,
            &[
                ("query", QueryCode::ALL.len()),
                ("query_result", QueryCode::ALL.len()),
                ("nominal_member_fact", 2),
                ("page_cursor", 10),
                ("query_outcome", 2),
                ("repair_target", 2),
                ("expected_category", 3),
                ("visible_cursor_purpose", VisibleCursorPurpose::ALL.len()),
                ("layout_failure", 3),
                ("definition_slot", 12),
                ("literal_value", 4),
                ("dependency_fact", 2),
                ("scalar_value", 4),
                ("change_kind", 11),
            ],
        );
        assert_named_variant_counts(
            &first.schema_discovery.variants,
            &[
                ("schema_projection", 3),
                ("describe_schema_result", 4),
                ("schema_root", SchemaRoot::ALL.len()),
                ("schema_definition_body", 14),
                ("payload_shape_kind", 3),
                ("json_scalar_kind", 3),
                ("machine_scalar_domain", 8),
                ("run_field_type", 7),
                ("runtime_value_payload", 6),
                ("draft_field_type", DraftFieldType::ALL.len()),
                ("operand_arity", 4),
                ("region_arity", 2),
                ("operand_use", 1),
                ("literal_field", 7),
                ("block_argument_role", 3),
                ("type_rule", 16),
            ],
        );
        assert_eq!(
            VisibleCursorPurpose::ALL
                .map(|value| serde_json::to_value(value).expect("visible purpose")),
            [
                serde_json::json!("visible_values"),
                serde_json::json!("legal_constructors"),
                serde_json::json!("repair_context"),
            ]
        );
        assert_eq!(RegionRole::ALL_STATIC.len() + 1, 4);
        assert_codes(
            &first.semantic_types,
            [
                ("unit", 1),
                ("bool", 2),
                ("i64", 3),
                ("bytes", 5),
                ("nominal", 4),
            ],
        );
        assert_variants(
            &first.structured_authoring.type_variants,
            [
                ("unit", 1),
                ("bool", 2),
                ("i64", 3),
                ("bytes", 0),
                ("nominal", 4),
            ],
        );
        assert_eq!(
            first
                .structured_authoring
                .draft_field_types
                .iter()
                .map(|description| (
                    description.name.as_str(),
                    description.type_expression.as_str(),
                ))
                .collect::<Vec<_>>(),
            DraftFieldType::ALL
                .into_iter()
                .map(|field_type| (field_type.machine_name(), field_type.type_expression(),))
                .collect::<Vec<_>>()
        );
        for field in first
            .structured_authoring
            .records
            .iter()
            .flat_map(|record| &record.fields)
            .chain(
                first
                    .structured_authoring
                    .expression_variants
                    .iter()
                    .chain(&first.structured_authoring.operation_variants)
                    .chain(&first.structured_authoring.value_variants)
                    .chain(&first.structured_authoring.type_variants)
                    .flat_map(|variant| &variant.fields),
            )
        {
            assert_eq!(field.nullable, !field.required, "{}", field.name);
        }
        assert!(
            first.structured_authoring.type_variants[..4]
                .iter()
                .all(|variant| variant.shape == PayloadShapeKind::Unit
                    && variant.newtype.is_none()
                    && variant.fields.is_empty())
        );
        let nominal = &first.structured_authoring.type_variants[4];
        assert_eq!(nominal.shape, PayloadShapeKind::Newtype);
        assert_eq!(nominal.newtype, Some(DraftFieldType::NodeTarget));
        assert!(nominal.fields.is_empty());
        assert!(
            !first
                .structured_authoring
                .records
                .iter()
                .any(|record| record.name == "type_draft")
        );
        assert_codes(
            &first.node_kinds,
            NodeKind::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.transaction_operations,
            TransactionOpCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variants(
            &first.structured_authoring.expression_variants,
            crate::transaction::ExpressionDraftCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variants(
            &first.structured_authoring.operation_variants,
            OperationCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variants(
            &first.structured_authoring.value_variants,
            crate::transaction::ValueDraftCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variant_payloads(
            &first.transaction_operation_payloads,
            TransactionOpCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variant_payloads(
            &first.query_payloads,
            QueryCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variant_payloads(
            &first.query_result_payloads,
            QueryCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_eq!(first.query_member_payloads.len(), 2);
        assert_eq!(first.query_cursor_payloads.len(), 10);
        assert!(
            first
                .query_records
                .iter()
                .any(|record| record.name == "nominal_type_result")
        );
        assert_variant_payloads(
            &first.request_payloads,
            RequestCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_variant_payloads(
            &first.response_payloads,
            ResponseCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_eq!(
            first.schema_discovery.roots,
            SchemaRoot::ALL
                .into_iter()
                .map(|root| root.machine_name().to_owned())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            first
                .schema_discovery
                .records
                .iter()
                .map(|record| record.name.as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            first.schema_discovery.records.len()
        );
        let catalogue = schema_definition_catalogue(&first).expect("catalogue");
        for root in SchemaRoot::ALL {
            assert!(catalogue.contains_key(root.machine_name()));
        }
        for variant in ["const_bool", "const_i64"] {
            let described = first
                .structured_authoring
                .expression_variants
                .iter()
                .find(|item| item.name == variant)
                .expect("newtype expression metadata");
            assert_eq!(described.shape, PayloadShapeKind::Newtype);
            assert!(described.fields.is_empty());
        }
        for variant in ["function_parameter", "block_argument"] {
            let described = first
                .structured_authoring
                .value_variants
                .iter()
                .find(|item| item.name == variant)
                .expect("newtype value metadata");
            assert_eq!(described.shape, PayloadShapeKind::Newtype);
            assert_eq!(described.newtype, Some(DraftFieldType::NodeTarget));
            assert!(described.fields.is_empty());
        }
        let if_fields = &first
            .structured_authoring
            .expression_variants
            .iter()
            .find(|variant| variant.name == "if")
            .expect("if metadata")
            .fields;
        assert_eq!(
            if_fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec!["condition", "result", "then_body", "else_body"]
        );
        let for_fields = &first
            .structured_authoring
            .expression_variants
            .iter()
            .find(|variant| variant.name == "for_i64")
            .expect("for metadata")
            .fields;
        assert!(
            for_fields
                .iter()
                .any(|field| field.name == "index_symbol" && field.declares_symbol)
        );
        assert!(
            for_fields
                .iter()
                .any(|field| field.name == "carried_symbol" && field.declares_symbol)
        );
        assert!(!first.structured_authoring.implicit_symbols_are_selectable);
        assert_eq!(
            first
                .run
                .fields
                .iter()
                .map(|field| (field.name.as_str(), field.field_type, field.required))
                .collect::<Vec<_>>(),
            vec![
                ("workspace", RunFieldType::Workspace, true),
                ("revision", RunFieldType::Revision, true),
                ("entry", RunFieldType::Node, true),
                ("arguments", RunFieldType::RuntimeValueList, true),
                ("policy", RunFieldType::RunPolicy, true),
            ]
        );
        assert_eq!(
            first
                .run
                .policy_fields
                .iter()
                .map(|field| (field.name.as_str(), field.field_type, field.required))
                .collect::<Vec<_>>(),
            vec![
                ("fuel", RunFieldType::U64, true),
                ("maximum_frames", RunFieldType::U32, true),
            ]
        );
        assert_eq!(
            first
                .run
                .runtime_values
                .iter()
                .map(|value| (value.name.as_str(), value.payload))
                .collect::<Vec<_>>(),
            vec![
                ("unit", RuntimeValuePayload::None),
                ("bool", RuntimeValuePayload::Bool),
                ("i64", RuntimeValuePayload::I64),
                ("bytes", RuntimeValuePayload::Bytes),
                ("product", RuntimeValuePayload::Product),
                ("sum", RuntimeValuePayload::Sum),
            ]
        );
        let product_runtime = first
            .run
            .runtime_values
            .iter()
            .find(|value| value.name == "product")
            .expect("product runtime schema");
        assert!(product_runtime
            .invariants
            .iter()
            .any(|invariant| invariant.contains("arbitrary") && invariant.contains("canonical")));
        let sum_runtime = first
            .run
            .runtime_values
            .iter()
            .find(|value| value.name == "sum")
            .expect("sum runtime schema");
        assert!(
            sum_runtime
                .invariants
                .iter()
                .any(|invariant| invariant.contains("payload") && invariant.contains("exact"))
        );
        assert!(
            first
                .run
                .limit_scope
                .iter()
                .any(|scope| scope.contains("aggregate across all Run arguments"))
        );
        assert!(
            first
                .run
                .limit_scope
                .iter()
                .any(|scope| scope.contains("peak frame arrays plus"))
        );
        assert_eq!(
            first.structured_authoring.maximum_request_depth,
            crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH as u32
        );
        for expected in [
            "product_field",
            "sum_variant",
            "call_argument",
            "product_binding",
            "match_arm",
        ] {
            assert!(
                first
                    .structured_authoring
                    .counted_item_categories
                    .iter()
                    .any(|category| category == expected)
            );
        }
        let match_operation = first
            .operations
            .iter()
            .find(|operation| operation.name == "match_sum")
            .expect("match operation");
        assert_eq!(
            match_operation.region_arity,
            RegionArity::MatchVariants {
                payload_type: TypeRule::VariantPayload,
                terminator: OperationCode::Yield,
                yield_type: TypeRule::MatchResult,
            }
        );
        assert!(match_operation.regions.is_empty());
        assert_codes(
            &first.queries,
            QueryCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_codes(
            &first.errors,
            ErrorCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_codes(
            &first.requests,
            RequestCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        assert_codes(
            &first.responses,
            ResponseCode::ALL.map(|code| (code.machine_name(), 0)),
        );
        for code in SemanticType::ALL {
            assert_eq!(
                serde_json::to_string(&code).expect("semantic type JSON"),
                format!("\"{}\"", code.machine_name())
            );
        }
        for code in NodeKind::ALL {
            assert_eq!(
                serde_json::to_string(&code).expect("node kind JSON"),
                format!("\"{}\"", code.machine_name())
            );
        }
        for code in ErrorCode::ALL {
            assert_eq!(
                serde_json::to_string(&code).expect("error code JSON"),
                format!("\"{}\"", code.machine_name())
            );
        }
        assert_eq!(
            first.limits.maximum_request_frame_bytes,
            MAX_JSON_INPUT_BYTES as u64
        );
        assert_eq!(
            first.limits.maximum_response_frame_bytes,
            MAX_JSON_OUTPUT_BYTES as u64
        );
        assert_eq!(
            first.limits.maximum_run_live_cells,
            crate::interpret::MAX_RUN_LIVE_CELLS as u64
        );
        assert_eq!(
            first.limits.maximum_error_related_ids,
            crate::error::MAX_ERROR_RELATED_IDS as u32
        );
        assert_eq!(
            first.limits.maximum_boundary_error_message_bytes,
            MAX_BOUNDARY_ERROR_MESSAGE_BYTES as u64
        );
        let request = DescribeSchemaRequest {
            projection: SchemaProjection::Full,
            known_digest: None,
        };
        let compact = encode_schema(&request, false).expect("compact");
        let pretty = encode_schema(&request, true).expect("pretty");
        assert_eq!(
            serde_json::from_slice::<DescribeSchemaResult>(&compact).expect("compact decode"),
            serde_json::from_slice::<DescribeSchemaResult>(&pretty).expect("pretty decode")
        );
        assert!(!compact.contains(&b'\n'));
    }

    #[test]
    fn digest_is_canonical_sensitive_and_excludes_projection_output() {
        let schema = schema_description();
        let digest = machine_schema_digest(&schema).expect("digest");
        assert_eq!(std::mem::size_of::<MachineSchemaDigest>(), 32);
        let digest_json = serde_json::to_string(&digest).expect("digest JSON");
        assert_eq!(digest_json.len(), 66);
        assert_eq!(
            serde_json::from_str::<MachineSchemaDigest>(&digest_json).expect("digest JSON decode"),
            digest
        );

        let mut reordered = schema.clone();
        reordered.scalar_types.reverse();
        reordered.semantic_types.reverse();
        reordered.node_kinds.reverse();
        reordered.name_contract.named_node_kinds.reverse();
        reordered.name_contract.sibling_uniqueness_groups.reverse();
        reordered
            .name_contract
            .sibling_uniqueness_groups
            .iter_mut()
            .for_each(|group| group.member_kinds.reverse());
        reordered.operations.reverse();
        reordered.semantic_records.reverse();
        reordered.semantic_variants.reverse();
        reordered
            .semantic_variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.transaction_operations.reverse();
        reordered.transaction_operation_payloads.reverse();
        reordered.transaction_records.reverse();
        reordered.transaction_variants.reverse();
        reordered
            .transaction_variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.structured_authoring.draft_field_types.reverse();
        reordered.structured_authoring.records.reverse();
        reordered.structured_authoring.expression_variants.reverse();
        reordered.structured_authoring.operation_variants.reverse();
        reordered.structured_authoring.value_variants.reverse();
        reordered.structured_authoring.type_variants.reverse();
        reordered.structured_authoring.implicit_node_kinds.reverse();
        reordered
            .structured_authoring
            .counted_item_categories
            .reverse();
        reordered.run.runtime_values.reverse();
        reordered.run.records.reverse();
        reordered.run.variants.reverse();
        reordered
            .run
            .variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.queries.reverse();
        reordered.query_payloads.reverse();
        reordered.query_result_payloads.reverse();
        reordered.query_records.reverse();
        reordered.query_variants.reverse();
        reordered
            .query_variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.query_member_payloads.reverse();
        reordered.query_cursor_payloads.reverse();
        reordered.errors.reverse();
        reordered.error_records.reverse();
        reordered.error_variants.reverse();
        reordered
            .error_variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.requests.reverse();
        reordered.request_payloads.reverse();
        reordered.responses.reverse();
        reordered.response_payloads.reverse();
        reordered.identity_variants.reverse();
        reordered
            .identity_variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.envelopes.reverse();
        reordered.boundary_error_kinds.reverse();
        reordered.schema_discovery.records.reverse();
        reordered.schema_discovery.variants.reverse();
        reordered
            .schema_discovery
            .variants
            .iter_mut()
            .for_each(|variant| variant.variants.reverse());
        reordered.schema_discovery.projection_payloads.reverse();
        reordered.schema_discovery.result_payloads.reverse();
        reordered.schema_discovery.roots.reverse();
        reordered.nominal_declarations.declaration_kinds.reverse();
        reordered.nominal_declarations.member_kinds.reverse();
        assert_eq!(
            machine_schema_digest(&reordered).expect("reordered digest"),
            digest
        );

        let mut changed_scalar_domain = schema.clone();
        changed_scalar_domain
            .scalar_types
            .iter_mut()
            .find(|scalar| scalar.name == "u8")
            .expect("u8 scalar")
            .domain = MachineScalarDomain::UnsignedInteger {
            minimum: 0,
            maximum: 254,
        };
        assert_ne!(
            machine_schema_digest(&changed_scalar_domain).expect("scalar domain digest"),
            digest
        );
        let mut changed_definition_slot = schema.clone();
        changed_definition_slot
            .query_variants
            .iter_mut()
            .find(|family| family.name == "definition_slot")
            .and_then(|family| family.variants.first_mut())
            .expect("definition slot variant")
            .name = "changed_definition_slot".to_owned();
        assert_ne!(
            machine_schema_digest(&changed_definition_slot).expect("enum family digest"),
            digest
        );
        let mut changed_request = schema.clone();
        changed_request.requests[0].name = "changed_request".to_owned();
        assert_ne!(
            machine_schema_digest(&changed_request).expect("request digest"),
            digest
        );
        let mut changed_field = schema.clone();
        changed_field.request_payloads[0].payload = record_payload(&[("changed", "u8", true)]);
        assert_ne!(
            machine_schema_digest(&changed_field).expect("field digest"),
            digest
        );
        let mut changed_operation = schema.clone();
        changed_operation.operations[0].complete = !changed_operation.operations[0].complete;
        assert_ne!(
            machine_schema_digest(&changed_operation).expect("operation digest"),
            digest
        );
        let mut changed_draft_mapping = schema.clone();
        changed_draft_mapping.structured_authoring.draft_field_types[0].type_expression =
            "string".into();
        assert_ne!(
            machine_schema_digest(&changed_draft_mapping).expect("draft mapping digest"),
            digest
        );
        let mut changed_nullability = schema.clone();
        changed_nullability
            .structured_authoring
            .records
            .iter_mut()
            .find(|record| record.name == "sum_variant")
            .and_then(|record| {
                record
                    .fields
                    .iter_mut()
                    .find(|field| field.name == "payload")
            })
            .expect("nullable draft field")
            .nullable = false;
        assert_ne!(
            machine_schema_digest(&changed_nullability).expect("draft nullability digest"),
            digest
        );
        let mut changed_optional_expression = schema.clone();
        let optional = changed_optional_expression
            .transaction_records
            .iter_mut()
            .flat_map(|record| &mut record.payload.fields)
            .find(|field| !field.required)
            .expect("optional machine field");
        optional.type_expression = "node_id".into();
        assert_ne!(
            machine_schema_digest(&changed_optional_expression)
                .expect("machine optional expression digest"),
            digest
        );
        for (catalogue, record_name, field_name) in [
            ("transaction", "apply_transaction_request", "response"),
            ("query", "query_batch_request", "queries"),
            ("transaction", "transaction_receipt", "published"),
            ("run", "run_result", "value"),
        ] {
            let mut changed = schema.clone();
            let records = match catalogue {
                "transaction" => &mut changed.transaction_records,
                "query" => &mut changed.query_records,
                "run" => &mut changed.run.records,
                _ => unreachable!(),
            };
            let field = records
                .iter_mut()
                .find(|record| record.name == record_name)
                .and_then(|record| {
                    record
                        .payload
                        .fields
                        .iter_mut()
                        .find(|field| field.name == field_name)
                })
                .expect("digest mutation field");
            field.required = !field.required;
            assert_ne!(
                machine_schema_digest(&changed).expect("boundary field digest"),
                digest,
                "{record_name}.{field_name}"
            );
        }
        let mut changed_name_contract = schema.clone();
        changed_name_contract.name_contract.minimum_utf8_bytes += 1;
        assert_ne!(
            machine_schema_digest(&changed_name_contract).expect("name contract digest"),
            digest
        );
        let mut changed_name_group = schema.clone();
        changed_name_group.name_contract.sibling_uniqueness_groups[0]
            .member_kinds
            .push(NodeKind::Module);
        assert_ne!(
            machine_schema_digest(&changed_name_group).expect("name group digest"),
            digest
        );
        for mutate in [
            |limits: &mut BoundaryLimits| limits.maximum_response_frame_bytes += 1,
            |limits: &mut BoundaryLimits| limits.maximum_artifact_bytes += 1,
            |limits: &mut BoundaryLimits| limits.maximum_artifact_name_bytes += 1,
        ] {
            let mut changed_limit = schema.clone();
            mutate(&mut changed_limit.limits);
            assert_ne!(
                machine_schema_digest(&changed_limit).expect("limit digest"),
                digest
            );
        }

        let manifest = describe_schema(&DescribeSchemaRequest::manifest()).expect("manifest");
        let DescribeSchemaResult::Manifest(manifest) = manifest else {
            panic!("manifest projection")
        };
        assert_eq!(manifest.digest, digest);
        assert_eq!(
            machine_schema_digest(&schema).expect("digest after output"),
            digest
        );
    }

    #[test]
    fn roots_and_known_digest_share_one_complete_digest() {
        let schema = schema_description();
        let digest = machine_schema_digest(&schema).expect("digest");
        assert!(matches!(
            describe_schema(&DescribeSchemaRequest {
                projection: SchemaProjection::Manifest,
                known_digest: Some(MachineSchemaDigest::from_bytes([0; 32])),
            })
            .expect("mismatched digest"),
            DescribeSchemaResult::Manifest(_)
        ));
        for projection in [
            SchemaProjection::Manifest,
            SchemaProjection::Roots {
                roots: vec![SchemaRoot::Error, SchemaRoot::Limits],
            },
            SchemaProjection::Full,
        ] {
            assert_eq!(
                describe_schema(&DescribeSchemaRequest {
                    projection,
                    known_digest: Some(digest),
                })
                .expect("known digest"),
                DescribeSchemaResult::Unchanged { digest }
            );
        }
    }

    #[test]
    fn root_requests_validate_and_project_in_canonical_order() {
        for roots in [
            vec![],
            vec![SchemaRoot::RuntimeValue, SchemaRoot::RuntimeValue],
            SchemaRoot::ALL[..MAX_SCHEMA_ROOTS + 1].to_vec(),
        ] {
            assert!(
                DescribeSchemaRequest {
                    projection: SchemaProjection::Roots { roots },
                    known_digest: None,
                }
                .validate()
                .is_err()
            );
        }
        let result = describe_schema(&DescribeSchemaRequest {
            projection: SchemaProjection::Roots {
                roots: vec![SchemaRoot::Limits, SchemaRoot::Error],
            },
            known_digest: None,
        })
        .expect("roots");
        let DescribeSchemaResult::Roots(result) = result else {
            panic!("roots projection")
        };
        assert_eq!(result.roots, vec![SchemaRoot::Error, SchemaRoot::Limits]);
        assert!(
            result
                .definitions
                .windows(2)
                .all(|pair| pair[0].name < pair[1].name)
        );
        let names = result
            .definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<BTreeSet<_>>();
        for definition in &result.definitions {
            assert!(
                definition
                    .dependencies
                    .iter()
                    .all(|dependency| names.contains(dependency.as_str()))
            );
        }
    }

    #[test]
    fn schema_projection_byte_measurements_are_retained() {
        let digest = active_machine_schema_digest().expect("digest");
        let cases = [
            ("manifest", DescribeSchemaRequest::manifest()),
            (
                "selected_agent_task_roots",
                DescribeSchemaRequest {
                    projection: SchemaProjection::Roots {
                        roots: vec![
                            SchemaRoot::CreateWorkspace,
                            SchemaRoot::ApplyTransaction,
                            SchemaRoot::QueryWorkspaceSummary,
                            SchemaRoot::QueryNode,
                            SchemaRoot::QueryBlockers,
                            SchemaRoot::QueryBody,
                            SchemaRoot::QueryIncomingUses,
                            SchemaRoot::QueryRepairContext,
                            SchemaRoot::QuerySemanticDiff,
                            SchemaRoot::QueryNominalType,
                            SchemaRoot::Run,
                            SchemaRoot::Shutdown,
                        ],
                    },
                    known_digest: None,
                },
            ),
            (
                "full",
                DescribeSchemaRequest {
                    projection: SchemaProjection::Full,
                    known_digest: None,
                },
            ),
            (
                "unchanged",
                DescribeSchemaRequest {
                    projection: SchemaProjection::Full,
                    known_digest: Some(digest),
                },
            ),
        ];
        let mut sizes = Vec::new();
        for (name, request) in cases {
            let result = describe_schema(&request).expect("projection");
            let definition_count = match &result {
                DescribeSchemaResult::Roots(result) => Some(result.definitions.len()),
                _ => None,
            };
            let json = serde_json::to_vec(&result).expect("projection JSON");
            let ipc_frame = encode_response(
                RequestId::new(1),
                &Response::DescribeSchema(Box::new(result)),
                false,
            )
            .expect("projection IPC JSON")
            .len()
                + u32::BITS as usize / 8;
            eprintln!(
                "schema_projection_bytes name={name} definitions={definition_count:?} json={} ipc_frame={ipc_frame}",
                json.len()
            );
            sizes.push((name, definition_count, json.len(), ipc_frame));
        }
        assert!(
            sizes
                .iter()
                .all(|(_, _, json, ipc_frame)| *json > 0 && *ipc_frame > 0)
        );
        assert_eq!(
            sizes,
            vec![
                ("manifest", None, 1_241, 1_319),
                ("selected_agent_task_roots", Some(112), 85_827, 85_905),
                ("full", None, 133_774, 133_852),
                ("unchanged", None, 105, 183),
            ]
        );
        assert!(sizes[1].2 < 86_009);
    }

    fn assert_variant_payloads<const N: usize>(
        actual: &[VariantPayloadDescription],
        expected: [(&'static str, u8); N],
    ) {
        assert_eq!(actual.len(), N);
        assert_eq!(
            actual
                .iter()
                .map(|variant| variant.name.as_str())
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );
    }

    fn assert_variants<const N: usize>(
        actual: &[DraftVariantDescription],
        expected: [(&'static str, u8); N],
    ) {
        assert_eq!(actual.len(), N);
        assert_eq!(
            actual
                .iter()
                .map(|variant| variant.name.as_str())
                .collect::<Vec<_>>(),
            expected
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );
    }

    fn assert_named_variant_counts(actual: &[NamedVariantDescription], expected: &[(&str, usize)]) {
        assert_eq!(actual.len(), expected.len());
        for (name, count) in expected {
            let family = actual
                .iter()
                .find(|family| family.name == *name)
                .unwrap_or_else(|| panic!("missing variant family {name}"));
            assert_eq!(family.variants.len(), *count, "{name}");
            assert_eq!(
                family
                    .variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                *count,
                "duplicate {name} variant"
            );
        }
    }

    fn assert_codes<const N: usize>(actual: &[CodeDescription], expected: [(&'static str, u8); N]) {
        assert_eq!(actual.len(), N);
        let actual = actual
            .iter()
            .map(|code| code.name.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            actual,
            expected
                .into_iter()
                .map(|(name, _)| name)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            actual.iter().copied().collect::<BTreeSet<_>>().len(),
            actual.len()
        );
    }
}
