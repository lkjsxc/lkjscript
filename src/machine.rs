//! Strict, bounded JSON transport projection and runtime machine contract description.

use crate::ids::RequestId;
use crate::protocol::{PROTOCOL_VERSION, Request, RequestCode, Response, ResponseCode};
use crate::query::{
    MAX_BATCH_ITEMS, MAX_BATCH_QUERIES, MAX_CONTEXT_ITEMS, MAX_PAGE_ITEMS, QueryCode,
};
use crate::schema::{
    BlockArgumentRole, LiteralField, NodeKind, OperandArity, OperandUse, OperationCode, RegionRole,
    SemanticType, TypeRule,
};
use crate::transaction::{MAX_RETURNED_BINDINGS, TransactionOpCode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Write};

pub const JSON_ENVELOPE_VERSION: u16 = 3;
pub const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_BOUNDARY_ERROR_MESSAGE_BYTES: usize = 1024;
const BOUNDARY_ERROR_FALLBACK: &[u8] =
    b"{\"version\":3,\"error\":{\"kind\":\"output\",\"message\":\"cannot encode boundary error\"}}";

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
    const fn machine_name(self) -> &'static str {
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
    Ok(envelope)
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
    encode_bounded(
        &ResponseEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id,
            response: response.clone(),
        },
        pretty,
    )
}

pub fn encode_schema(pretty: bool) -> Result<Vec<u8>, MachineError> {
    encode_bounded(&schema_description(), pretty)
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDescription {
    pub binary_protocol_version: u16,
    pub json_envelope_version: u16,
    pub semantic_types: Vec<CodeDescription>,
    pub node_kinds: Vec<CodeDescription>,
    pub operations: Vec<OperationDescription>,
    pub transaction_operations: Vec<CodeDescription>,
    pub transaction_operation_payloads: Vec<VariantPayloadDescription>,
    pub structured_authoring: StructuredAuthoringDescription,
    pub run: RunDescription,
    pub queries: Vec<CodeDescription>,
    pub query_payloads: Vec<VariantPayloadDescription>,
    pub errors: Vec<CodeDescription>,
    pub error_payload: PayloadShapeDescription,
    pub requests: Vec<CodeDescription>,
    pub request_payloads: Vec<VariantPayloadDescription>,
    pub responses: Vec<CodeDescription>,
    pub response_payloads: Vec<VariantPayloadDescription>,
    pub envelopes: Vec<NamedPayloadDescription>,
    pub boundary_error_kinds: Vec<String>,
    pub limits: BoundaryLimits,
    pub id_formats: IdFormats,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeDescription {
    pub name: String,
    pub tag: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadShapeKind {
    Unit,
    Newtype,
    Record,
}
impl PayloadShapeKind {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::Unit => 1,
            Self::Newtype => 2,
            Self::Record => 3,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Unit),
            2 => Some(Self::Newtype),
            3 => Some(Self::Record),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineFieldDescription {
    pub name: String,
    pub type_expression: String,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadShapeDescription {
    pub shape: PayloadShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newtype: Option<String>,
    pub fields: Vec<MachineFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VariantPayloadDescription {
    pub name: String,
    pub tag: u8,
    pub payload: PayloadShapeDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedPayloadDescription {
    pub name: String,
    pub payload: PayloadShapeDescription,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunFieldType {
    Workspace,
    Revision,
    Node,
    RuntimeValueList,
    RunPolicy,
    U64,
    U32,
}
impl RunFieldType {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::Workspace => 1,
            Self::Revision => 2,
            Self::Node => 3,
            Self::RuntimeValueList => 4,
            Self::RunPolicy => 5,
            Self::U64 => 6,
            Self::U32 => 7,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Workspace),
            2 => Some(Self::Revision),
            3 => Some(Self::Node),
            4 => Some(Self::RuntimeValueList),
            5 => Some(Self::RunPolicy),
            6 => Some(Self::U64),
            7 => Some(Self::U32),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunFieldDescription {
    pub name: String,
    pub field_type: RunFieldType,
    pub required: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunDescription {
    pub fields: Vec<RunFieldDescription>,
    pub policy_fields: Vec<RunFieldDescription>,
    pub runtime_values: Vec<RuntimeValueDescription>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValuePayload {
    None,
    Bool,
    I64,
}
impl RuntimeValuePayload {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::None => 1,
            Self::Bool => 2,
            Self::I64 => 3,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::None),
            2 => Some(Self::Bool),
            3 => Some(Self::I64),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeValueDescription {
    pub name: String,
    pub tag: u8,
    pub payload: RuntimeValuePayload,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredAuthoringDescription {
    pub records: Vec<DraftRecordDescription>,
    pub expression_variants: Vec<DraftVariantDescription>,
    pub value_variants: Vec<DraftVariantDescription>,
    pub allocation_order: String,
    pub explicit_handles_are_selectable: bool,
    pub implicit_handles_are_selectable: bool,
    pub implicit_node_kinds: Vec<NodeKind>,
    pub maximum_request_depth: u32,
    pub maximum_request_items: u64,
    pub counted_item_categories: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftFieldType {
    LocalHandle,
    NodeTarget,
    NodeId,
    String,
    SemanticType,
    I64,
    U8,
    Value,
    ValueList,
    ExpressionKind,
    ExpressionList,
    ParameterList,
    FunctionBody,
    YieldingBody,
    Bool,
    Expression,
}
impl DraftFieldType {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::LocalHandle => 1,
            Self::NodeTarget => 2,
            Self::NodeId => 3,
            Self::String => 4,
            Self::SemanticType => 5,
            Self::I64 => 6,
            Self::U8 => 7,
            Self::Value => 8,
            Self::ValueList => 9,
            Self::ExpressionKind => 10,
            Self::ExpressionList => 11,
            Self::ParameterList => 12,
            Self::FunctionBody => 13,
            Self::YieldingBody => 14,
            Self::Bool => 15,
            Self::Expression => 16,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::LocalHandle),
            2 => Some(Self::NodeTarget),
            3 => Some(Self::NodeId),
            4 => Some(Self::String),
            5 => Some(Self::SemanticType),
            6 => Some(Self::I64),
            7 => Some(Self::U8),
            8 => Some(Self::Value),
            9 => Some(Self::ValueList),
            10 => Some(Self::ExpressionKind),
            11 => Some(Self::ExpressionList),
            12 => Some(Self::ParameterList),
            13 => Some(Self::FunctionBody),
            14 => Some(Self::YieldingBody),
            15 => Some(Self::Bool),
            16 => Some(Self::Expression),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFieldDescription {
    pub name: String,
    pub field_type: DraftFieldType,
    pub required: bool,
    pub declares_handle: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRecordDescription {
    pub name: String,
    pub fields: Vec<DraftFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftVariantDescription {
    pub name: String,
    pub tag: u8,
    pub shape: PayloadShapeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub newtype: Option<DraftFieldType>,
    pub fields: Vec<DraftFieldDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDescription {
    pub name: String,
    pub tag: u8,
    pub operand_arity: OperandArity,
    pub operands: Vec<OperandDescription>,
    pub results: Vec<TypeRule>,
    pub literal_fields: Vec<LiteralField>,
    pub regions: Vec<RegionDescription>,
    pub complete: bool,
    pub terminator: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RegionDescription {
    pub role: RegionRole,
    pub block_arguments: Vec<BlockArgumentDescription>,
    pub terminator: OperationCode,
    pub yield_type: TypeRule,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockArgumentDescription {
    pub role: BlockArgumentRole,
    pub ty: TypeRule,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperandDescription {
    pub ty: TypeRule,
    pub use_mode: OperandUse,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryLimits {
    pub maximum_frame_bytes: u64,
    pub maximum_frame_items: u64,
    pub maximum_json_input_bytes: u64,
    pub maximum_json_output_bytes: u64,
    pub maximum_page_items: u32,
    pub maximum_batch_queries: u32,
    pub maximum_batch_items: u32,
    pub maximum_context_items_per_category: u32,
    pub maximum_returned_bindings: u32,
    pub maximum_run_arguments: u32,
    pub maximum_run_fuel: u64,
    pub maximum_run_frames: u32,
    pub maximum_run_live_value_slots: u64,
    pub maximum_error_related_ids: u32,
    pub maximum_persistence_head_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdFormats {
    pub workspace: String,
    pub idempotency_key: String,
    pub node: String,
    pub snapshot_hash: String,
    pub change_digest: String,
    pub revision: String,
    pub request_id: String,
    pub query_id: String,
    pub local_handle: String,
}

fn machine_field(name: &str, type_expression: &str, required: bool) -> MachineFieldDescription {
    MachineFieldDescription {
        name: name.into(),
        type_expression: type_expression.into(),
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
fn variant_payload(
    name: &str,
    tag: u8,
    payload: PayloadShapeDescription,
) -> VariantPayloadDescription {
    VariantPayloadDescription {
        name: name.into(),
        tag,
        payload,
    }
}

fn draft_field(
    name: &str,
    field_type: DraftFieldType,
    required: bool,
    declares_handle: bool,
) -> DraftFieldDescription {
    DraftFieldDescription {
        name: name.to_owned(),
        field_type,
        required,
        declares_handle,
    }
}

fn structured_records() -> Vec<DraftRecordDescription> {
    use DraftFieldType as T;
    vec![
        DraftRecordDescription {
            name: "create_function".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("parameters", T::ParameterList, true, false),
                draft_field("result", T::SemanticType, true, false),
                draft_field("body", T::FunctionBody, false, false),
            ],
        },
        DraftRecordDescription {
            name: "function_parameter".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::SemanticType, true, false),
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
            name: "expression".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("operation", T::ExpressionKind, true, false),
            ],
        },
        DraftRecordDescription {
            name: "define_function_body".into(),
            fields: vec![
                draft_field("function", T::NodeTarget, true, false),
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
        C::AddI64 | C::LtI64 => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("lhs", T::Value, true, false),
                draft_field("rhs", T::Value, true, false),
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
            vec![draft_field("expected", T::SemanticType, true, false)],
        ),
        C::If => (
            PayloadShapeKind::Record,
            None,
            vec![
                draft_field("condition", T::Value, true, false),
                draft_field("result", T::SemanticType, true, false),
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
                draft_field("carried", T::SemanticType, true, false),
                draft_field("index_handle", T::LocalHandle, true, true),
                draft_field("carried_handle", T::LocalHandle, true, true),
                draft_field("body", T::YieldingBody, true, false),
            ],
        ),
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        tag: code.stable_tag(),
        shape,
        newtype,
        fields,
    }
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
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        tag: code.stable_tag(),
        shape,
        newtype,
        fields,
    }
}

fn request_payload(code: RequestCode) -> VariantPayloadDescription {
    let payload = match code {
        RequestCode::CreateWorkspace | RequestCode::Shutdown | RequestCode::DescribeSchema => {
            unit_payload()
        }
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
    variant_payload(code.machine_name(), code.stable_tag(), payload)
}

fn response_payload(code: ResponseCode) -> VariantPayloadDescription {
    let payload = match code {
        ResponseCode::WorkspaceCreated => newtype_payload("workspace_summary"),
        ResponseCode::TransactionReceipt => newtype_payload("transaction_receipt"),
        ResponseCode::QueryBatchResult => newtype_payload("query_batch_result"),
        ResponseCode::Run => newtype_payload("run_result"),
        ResponseCode::Acknowledged => unit_payload(),
        ResponseCode::Error => newtype_payload("error"),
        ResponseCode::SchemaDescription => newtype_payload("schema_description"),
    };
    variant_payload(code.machine_name(), code.stable_tag(), payload)
}

fn transaction_payload(code: TransactionOpCode) -> VariantPayloadDescription {
    let payload = match code {
        TransactionOpCode::CreatePackage => {
            record_payload(&[("handle", "local_handle", true), ("name", "string", true)])
        }
        TransactionOpCode::CreateModule => record_payload(&[
            ("handle", "local_handle", true),
            ("package", "node_target", true),
            ("name", "string", true),
        ]),
        TransactionOpCode::CreateFunction => record_payload(&[
            ("handle", "local_handle", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("parameters", "list<function_parameter_draft>", true),
            ("result", "semantic_type", true),
            ("body", "function_body_draft", false),
        ]),
        TransactionOpCode::DefineFunctionBody => record_payload(&[
            ("function", "node_target", true),
            ("body", "function_body_draft", true),
        ]),
        TransactionOpCode::InsertExpression => record_payload(&[
            ("block", "node_id", true),
            ("before", "node_id", false),
            ("expression", "expression_draft", true),
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
    variant_payload(code.machine_name(), code.stable_tag(), payload)
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
    };
    variant_payload(code.machine_name(), code.stable_tag(), payload)
}

fn error_payload() -> PayloadShapeDescription {
    record_payload(&[
        ("code", "error_code", true),
        ("workspace", "workspace_id", false),
        ("revision", "revision", false),
        ("operation_index", "u32", false),
        ("local_handle", "local_handle", false),
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
        NamedPayloadDescription {
            name: "boundary_error".into(),
            payload: record_payload(&[
                ("kind", "boundary_error_kind", true),
                ("message", "string", true),
            ]),
        },
    ]
}

pub fn schema_description() -> SchemaDescription {
    SchemaDescription {
        binary_protocol_version: PROTOCOL_VERSION,
        json_envelope_version: JSON_ENVELOPE_VERSION,
        semantic_types: SemanticType::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        node_kinds: NodeKind::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        operations: OperationCode::ALL
            .into_iter()
            .map(|code| {
                let descriptor = code.descriptor();
                OperationDescription {
                    name: descriptor.machine_name.to_owned(),
                    tag: descriptor.stable_tag,
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
        transaction_operations: TransactionOpCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        transaction_operation_payloads: TransactionOpCode::ALL
            .into_iter()
            .map(transaction_payload)
            .collect(),
        structured_authoring: StructuredAuthoringDescription {
            records: structured_records(),
            expression_variants: crate::transaction::ExpressionDraftCode::ALL
                .into_iter()
                .map(expression_variant)
                .collect(),
            value_variants: crate::transaction::ValueDraftCode::ALL
                .into_iter()
                .map(value_variant)
                .collect(),
            allocation_order: "depth_first_preorder_canonical_nodes".to_owned(),
            explicit_handles_are_selectable: true,
            implicit_handles_are_selectable: false,
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
                "function_body".into(),
                "yielding_body".into(),
                "expression".into(),
                "call_argument".into(),
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
                    tag: code.stable_tag(),
                    payload: match code {
                        crate::interpret::RuntimeValueCode::Unit => RuntimeValuePayload::None,
                        crate::interpret::RuntimeValueCode::Bool => RuntimeValuePayload::Bool,
                        crate::interpret::RuntimeValueCode::I64 => RuntimeValuePayload::I64,
                    },
                })
                .collect(),
        },
        queries: QueryCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        query_payloads: QueryCode::ALL.into_iter().map(query_payload).collect(),
        errors: crate::ErrorCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        error_payload: error_payload(),
        requests: RequestCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        request_payloads: RequestCode::ALL.into_iter().map(request_payload).collect(),
        responses: ResponseCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        response_payloads: ResponseCode::ALL
            .into_iter()
            .map(response_payload)
            .collect(),
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
            maximum_frame_bytes: crate::protocol::MAXIMUM_FRAME_BYTES as u64,
            maximum_frame_items: crate::protocol::MAXIMUM_FRAME_ITEMS as u64,
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
            maximum_run_live_value_slots: crate::interpret::MAX_RUN_LIVE_VALUE_SLOTS as u64,
            maximum_error_related_ids: crate::error::MAX_ERROR_RELATED_IDS as u32,
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
            local_handle: "JSON unsigned 32-bit integer".to_owned(),
        },
    }
}

fn described(name: &str, tag: u8) -> CodeDescription {
    CodeDescription {
        name: name.to_owned(),
        tag,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{ContextBudget, PageRequest, Query, QueryBatchRequest, QueryItem};
    use crate::transaction::{
        ExpressionDraft, ExpressionKindDraft, Transaction, TransactionMode, TransactionOp,
        TransactionResponseSpec, YieldingBodyDraft,
    };
    use crate::{
        ApplyTransactionRequest, ErrorCode, LocalHandle, NodeId, NodeTarget, QueryId, Revision,
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
            text.replacen("\"version\":3", "\"version\":2", 1),
            text.replacen("\"request_id\":1", "\"request_id\":0", 1),
            text.replacen("{\"version\":3", "{\"unknown\":0,\"version\":3", 1),
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
        let local = NodeTarget::Local;
        let nested = |levels: usize| {
            let mut expression = ExpressionDraft {
                handle: LocalHandle::new(1),
                operation: ExpressionKindDraft::ConstI64(1),
            };
            for level in 0..levels {
                let inner = expression.handle;
                let else_handle = LocalHandle::new(100 + level as u32);
                expression = ExpressionDraft {
                    handle: LocalHandle::new(1000 + level as u32),
                    operation: ExpressionKindDraft::If {
                        condition: ValueDraft::FunctionParameter(local(LocalHandle::new(9))),
                        result: SemanticType::I64,
                        then_body: YieldingBodyDraft {
                            operations: vec![expression],
                            yield_value: ValueDraft::OperationResult {
                                operation: local(inner),
                                output: 0,
                            },
                        },
                        else_body: YieldingBodyDraft {
                            operations: vec![ExpressionDraft {
                                handle: else_handle,
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
    fn described_representative_payloads_match_strict_json_shapes() {
        let schema = schema_description();
        let workspace = WorkspaceId::from_bytes([9; 16]);
        let node = NodeId::new(workspace, 9).expect("node");
        let refine = TransactionOp::RefineHole {
            hole: NodeTarget::Existing(node),
            replacement: crate::OperationDraft::ConstI64(7),
        };
        assert_tagged_record(
            serde_json::to_value(refine).expect("refine JSON"),
            &schema.transaction_operation_payloads,
            "refine_hole",
        );

        let repair = Query::RepairContext {
            target: crate::query::RepairTarget::Hole(node),
            budget: ContextBudget {
                body_before: 1,
                body_after: 1,
                visible_values: 1,
                incoming_uses: 1,
                include_incompatible: true,
            },
        };
        assert_tagged_record(
            serde_json::to_value(repair).expect("repair JSON"),
            &schema.query_payloads,
            "repair_context",
        );

        let run = Request::Run {
            workspace,
            revision: Revision::INITIAL,
            entry: node,
            arguments: vec![crate::RuntimeValue::I64(3)],
            policy: crate::RunPolicy {
                fuel: 10,
                maximum_frames: 2,
            },
        };
        assert_tagged_record(
            serde_json::to_value(run).expect("run JSON"),
            &schema.request_payloads,
            "run",
        );

        let error = Response::Error(crate::error::LkError::new(ErrorCode::InvalidOperand, "bad"));
        let encoded = serde_json::to_value(error.clone()).expect("error JSON");
        let data = encoded
            .get("data")
            .and_then(serde_json::Value::as_object)
            .expect("error data");
        let required = schema
            .error_payload
            .fields
            .iter()
            .filter(|field| field.required)
            .map(|field| field.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let actual = data
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, required);
        assert_eq!(
            schema
                .error_payload
                .fields
                .iter()
                .filter(|field| !field.required)
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                "workspace",
                "revision",
                "operation_index",
                "local_handle",
                "target",
                "expected_kind",
                "actual_kind",
                "expected_type",
                "actual_type",
            ]
        );

        let response_envelope = ResponseEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(1),
            response: error,
        };
        assert_named_record(
            serde_json::to_value(response_envelope).expect("response envelope"),
            &schema.envelopes,
            "response_envelope",
        );

        let request_envelope = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(1),
            request: Request::DescribeSchema,
        };
        assert_named_record(
            serde_json::to_value(request_envelope).expect("request envelope"),
            &schema.envelopes,
            "request_envelope",
        );
        let boundary = BoundaryErrorEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: None,
            error: BoundaryError {
                kind: BoundaryErrorKind::InvalidJson,
                message: "bad".into(),
            },
        };
        let value = serde_json::to_value(boundary).expect("boundary envelope");
        assert_named_record(value.clone(), &schema.envelopes, "boundary_error_envelope");
        assert_named_record(
            value.get("error").cloned().expect("boundary data"),
            &schema.envelopes,
            "boundary_error",
        );
    }

    fn assert_tagged_record(
        value: serde_json::Value,
        variants: &[VariantPayloadDescription],
        name: &str,
    ) {
        assert_eq!(
            value.get("kind").and_then(serde_json::Value::as_str),
            Some(name)
        );
        let payload = &variants
            .iter()
            .find(|variant| variant.name == name)
            .expect("variant metadata")
            .payload;
        assert_eq!(payload.shape, PayloadShapeKind::Record);
        assert_named_fields(value.get("data").expect("record data"), payload);
    }

    fn assert_named_record(
        value: serde_json::Value,
        variants: &[NamedPayloadDescription],
        name: &str,
    ) {
        let payload = &variants
            .iter()
            .find(|variant| variant.name == name)
            .expect("named metadata")
            .payload;
        assert_named_fields(&value, payload);
    }

    fn assert_named_fields(value: &serde_json::Value, payload: &PayloadShapeDescription) {
        let object = value.as_object().expect("record object");
        let actual = object
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let expected = payload
            .fields
            .iter()
            .filter(|field| field.required || object.contains_key(&field.name))
            .map(|field| field.name.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(actual, expected);
    }

    #[test]
    fn schema_is_deterministic_complete_and_unique() {
        let first = schema_description();
        assert_eq!(first, schema_description());
        assert_eq!(first.operations.len(), OperationCode::ALL.len());
        assert_codes(
            &first.semantic_types,
            SemanticType::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.node_kinds,
            NodeKind::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.transaction_operations,
            TransactionOpCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variants(
            &first.structured_authoring.expression_variants,
            crate::transaction::ExpressionDraftCode::ALL
                .map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variants(
            &first.structured_authoring.value_variants,
            crate::transaction::ValueDraftCode::ALL
                .map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variant_payloads(
            &first.transaction_operation_payloads,
            TransactionOpCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variant_payloads(
            &first.query_payloads,
            QueryCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variant_payloads(
            &first.request_payloads,
            RequestCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variant_payloads(
            &first.response_payloads,
            ResponseCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
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
                .any(|field| field.name == "index_handle" && field.declares_handle)
        );
        assert!(
            for_fields
                .iter()
                .any(|field| field.name == "carried_handle" && field.declares_handle)
        );
        assert!(!first.structured_authoring.implicit_handles_are_selectable);
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
                .map(|value| (value.name.as_str(), value.tag, value.payload))
                .collect::<Vec<_>>(),
            vec![
                ("unit", 1, RuntimeValuePayload::None),
                ("bool", 2, RuntimeValuePayload::Bool),
                ("i64", 3, RuntimeValuePayload::I64),
            ]
        );
        assert_eq!(
            first.structured_authoring.maximum_request_depth,
            crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH as u32
        );
        assert!(
            first
                .structured_authoring
                .counted_item_categories
                .iter()
                .any(|category| category == "call_argument")
        );
        assert_codes(
            &first.queries,
            QueryCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.errors,
            ErrorCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.requests,
            RequestCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.responses,
            ResponseCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
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
            first.limits.maximum_frame_items,
            crate::protocol::MAXIMUM_FRAME_ITEMS as u64
        );
        assert_eq!(
            first.limits.maximum_run_live_value_slots,
            crate::interpret::MAX_RUN_LIVE_VALUE_SLOTS as u64
        );
        assert_eq!(
            first.limits.maximum_error_related_ids,
            crate::error::MAX_ERROR_RELATED_IDS as u32
        );
        let compact = encode_schema(false).expect("compact");
        let pretty = encode_schema(true).expect("pretty");
        assert_eq!(
            serde_json::from_slice::<SchemaDescription>(&compact).expect("compact decode"),
            serde_json::from_slice::<SchemaDescription>(&pretty).expect("pretty decode")
        );
        assert!(!compact.contains(&b'\n'));
    }

    fn assert_variant_payloads<const N: usize>(
        actual: &[VariantPayloadDescription],
        expected: [(&'static str, u8); N],
    ) {
        assert_eq!(actual.len(), N);
        assert_eq!(
            actual
                .iter()
                .map(|variant| (variant.name.as_str(), variant.tag))
                .collect::<Vec<_>>(),
            expected
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
                .map(|variant| (variant.name.as_str(), variant.tag))
                .collect::<Vec<_>>(),
            expected
        );
    }

    fn assert_codes<const N: usize>(actual: &[CodeDescription], expected: [(&'static str, u8); N]) {
        assert_eq!(actual.len(), N);
        let actual: Vec<_> = actual
            .iter()
            .map(|code| (code.name.as_str(), code.tag))
            .collect();
        assert_eq!(actual, expected);
        let mut names = std::collections::BTreeSet::new();
        let mut tags = std::collections::BTreeSet::new();
        assert!(
            actual
                .iter()
                .all(|(name, tag)| names.insert(*name) && tags.insert(*tag))
        );
    }
}
