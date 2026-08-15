//! Strict, bounded JSON transport projection and runtime machine contract description.

use crate::ids::RequestId;
use crate::protocol::{PROTOCOL_VERSION, Request, RequestCode, Response, ResponseCode};
use crate::query::{
    MAX_BATCH_ITEMS, MAX_BATCH_QUERIES, MAX_CONTEXT_ITEMS, MAX_PAGE_ITEMS, QueryCode,
};
use crate::schema::{
    BlockArgumentRole, LiteralField, NodeKind, OperandArity, OperandUse, OperationCode,
    RegionArity, RegionRole, SemanticType, TypeRule,
};
use crate::transaction::{MAX_RETURNED_BINDINGS, TransactionOpCode};
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::io::{self, Write};
use std::str::FromStr;

pub const JSON_ENVELOPE_VERSION: u16 = 4;
pub const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
pub const MACHINE_SCHEMA_IDENTITY: &str = "lkjscript-machine-schema-v4";
pub const MAX_SCHEMA_SECTIONS: usize = SchemaSection::ALL.len();
const MACHINE_SCHEMA_DIGEST_DOMAIN: &str = "lkjscript.machine-schema.digest.v1";
const MAX_BOUNDARY_ERROR_MESSAGE_BYTES: usize = 1024;
const BOUNDARY_ERROR_FALLBACK: &[u8] =
    b"{\"version\":4,\"error\":{\"kind\":\"output\",\"message\":\"cannot encode boundary error\"}}";

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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MachineSchemaDigest([u8; 32]);

impl MachineSchemaDigest {
    pub const BYTE_LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }
}

impl fmt::Display for MachineSchemaDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = [0_u8; 64];
        for (index, byte) in self.0.iter().copied().enumerate() {
            output[index * 2] = HEX[usize::from(byte >> 4)];
            output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
        }
        formatter.write_str(std::str::from_utf8(&output).map_err(|_| fmt::Error)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MachineSchemaDigestParseError;

impl fmt::Display for MachineSchemaDigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("machine schema digest must be exactly 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for MachineSchemaDigestParseError {}

impl FromStr for MachineSchemaDigest {
    type Err = MachineSchemaDigestParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(MachineSchemaDigestParseError);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let digit = |value: u8| match value {
                b'0'..=b'9' => Some(value - b'0'),
                b'a'..=b'f' => Some(value - b'a' + 10),
                _ => None,
            };
            let high = digit(pair[0]).ok_or(MachineSchemaDigestParseError)?;
            let low = digit(pair[1]).ok_or(MachineSchemaDigestParseError)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for MachineSchemaDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for MachineSchemaDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DigestVisitor;
        impl Visitor<'_> for DigestVisitor {
            type Value = MachineSchemaDigest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a canonical lowercase machine schema digest")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(DigestVisitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaSection {
    IdentityAndEnvelopes,
    SemanticTypesAndNodes,
    NominalDeclarations,
    TransactionsAndExpressions,
    QueriesAndRepair,
    RuntimeAndRun,
    ErrorsAndLimits,
}

impl SchemaSection {
    pub const ALL: [Self; 7] = [
        Self::IdentityAndEnvelopes,
        Self::SemanticTypesAndNodes,
        Self::NominalDeclarations,
        Self::TransactionsAndExpressions,
        Self::QueriesAndRepair,
        Self::RuntimeAndRun,
        Self::ErrorsAndLimits,
    ];

    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::IdentityAndEnvelopes => 1,
            Self::SemanticTypesAndNodes => 2,
            Self::NominalDeclarations => 3,
            Self::TransactionsAndExpressions => 4,
            Self::QueriesAndRepair => 5,
            Self::RuntimeAndRun => 6,
            Self::ErrorsAndLimits => 7,
        }
    }

    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::IdentityAndEnvelopes),
            2 => Some(Self::SemanticTypesAndNodes),
            3 => Some(Self::NominalDeclarations),
            4 => Some(Self::TransactionsAndExpressions),
            5 => Some(Self::QueriesAndRepair),
            6 => Some(Self::RuntimeAndRun),
            7 => Some(Self::ErrorsAndLimits),
            _ => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::IdentityAndEnvelopes => "identity_and_envelopes",
            Self::SemanticTypesAndNodes => "semantic_types_and_nodes",
            Self::NominalDeclarations => "nominal_declarations",
            Self::TransactionsAndExpressions => "transactions_and_expressions",
            Self::QueriesAndRepair => "queries_and_repair",
            Self::RuntimeAndRun => "runtime_and_run",
            Self::ErrorsAndLimits => "errors_and_limits",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaProjection {
    Manifest,
    Sections { sections: Vec<SchemaSection> },
    Full,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DescribeSchemaRequest {
    pub projection: SchemaProjection,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub known_digest: Option<MachineSchemaDigest>,
}

impl DescribeSchemaRequest {
    pub fn manifest() -> Self {
        Self {
            projection: SchemaProjection::Manifest,
            known_digest: None,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if let SchemaProjection::Sections { sections } = &self.projection {
            if sections.is_empty() {
                return Err("schema section list must not be empty");
            }
            if sections.len() > MAX_SCHEMA_SECTIONS {
                return Err("schema section count exceeds policy");
            }
            let mut canonical = sections.clone();
            canonical.sort_unstable();
            if canonical.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err("schema section list contains a duplicate");
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DescribeSchemaResult {
    Unchanged {
        digest: MachineSchemaDigest,
    },
    Manifest(SchemaManifest),
    Sections(SchemaSections),
    Full {
        digest: MachineSchemaDigest,
        description: Box<SchemaDescription>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaManifest {
    pub schema_identity: String,
    pub digest: MachineSchemaDigest,
    pub binary_protocol_version: u16,
    pub json_envelope_version: u16,
    pub artifact_format_version: u16,
    pub artifact_magic_hex: String,
    pub semantic_schema_identity: String,
    pub sections: Vec<CodeDescription>,
    pub maximum_sections_per_request: u8,
    pub full_available: bool,
    pub maximum_frame_bytes: u64,
    pub maximum_json_output_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaSections {
    pub digest: MachineSchemaDigest,
    pub sections: Vec<SchemaSectionPayload>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SchemaSectionPayload {
    IdentityAndEnvelopes(Box<IdentityAndEnvelopesSection>),
    SemanticTypesAndNodes(Box<SemanticTypesAndNodesSection>),
    NominalDeclarations(Box<NominalDeclarationsDescription>),
    TransactionsAndExpressions(Box<TransactionsAndExpressionsSection>),
    QueriesAndRepair(Box<QueriesAndRepairSection>),
    RuntimeAndRun(Box<RunDescription>),
    ErrorsAndLimits(Box<ErrorsAndLimitsSection>),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDescription {
    pub machine_schema_identity: String,
    pub binary_protocol_version: u16,
    pub json_envelope_version: u16,
    pub artifact_format_version: u16,
    pub artifact_magic_hex: String,
    pub semantic_schema_identity: String,
    pub schema_discovery: SchemaDiscoveryDescription,
    pub scalar_types: Vec<MachineScalarDescription>,
    pub semantic_types: Vec<CodeDescription>,
    pub node_kinds: Vec<CodeDescription>,
    pub name_contract: NameContractDescription,
    pub operations: Vec<OperationDescription>,
    pub semantic_records: Vec<NamedPayloadDescription>,
    pub semantic_variants: Vec<NamedVariantDescription>,
    pub transaction_operations: Vec<CodeDescription>,
    pub transaction_operation_payloads: Vec<VariantPayloadDescription>,
    pub transaction_records: Vec<NamedPayloadDescription>,
    pub transaction_variants: Vec<NamedVariantDescription>,
    pub structured_authoring: StructuredAuthoringDescription,
    pub run: RunDescription,
    pub queries: Vec<CodeDescription>,
    pub query_payloads: Vec<VariantPayloadDescription>,
    pub query_result_payloads: Vec<VariantPayloadDescription>,
    pub query_records: Vec<NamedPayloadDescription>,
    pub query_variants: Vec<NamedVariantDescription>,
    pub query_member_payloads: Vec<VariantPayloadDescription>,
    pub query_cursor_payloads: Vec<VariantPayloadDescription>,
    pub errors: Vec<CodeDescription>,
    pub error_payload: PayloadShapeDescription,
    pub error_records: Vec<NamedPayloadDescription>,
    pub error_variants: Vec<NamedVariantDescription>,
    pub requests: Vec<CodeDescription>,
    pub request_payloads: Vec<VariantPayloadDescription>,
    pub responses: Vec<CodeDescription>,
    pub response_payloads: Vec<VariantPayloadDescription>,
    pub identity_variants: Vec<NamedVariantDescription>,
    pub envelopes: Vec<NamedPayloadDescription>,
    pub boundary_error_kinds: Vec<String>,
    pub limits: BoundaryLimits,
    pub id_formats: IdFormats,
    pub nominal_declarations: NominalDeclarationsDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaDiscoveryDescription {
    pub digest_format: String,
    pub digest_domain: String,
    pub request: PayloadShapeDescription,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub projection_payloads: Vec<VariantPayloadDescription>,
    pub result_payloads: Vec<VariantPayloadDescription>,
    pub sections: Vec<CodeDescription>,
    pub section_payloads: Vec<VariantPayloadDescription>,
    pub maximum_sections_per_request: u8,
    pub full_available: bool,
    pub known_digest_match_precedes_projection: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalDeclarationsDescription {
    pub declaration_kinds: Vec<NodeKind>,
    pub member_kinds: Vec<NodeKind>,
    pub shape_invariants: Vec<String>,
    pub layout_invariants: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityAndEnvelopesSection {
    pub machine_schema_identity: String,
    pub binary_protocol_version: u16,
    pub json_envelope_version: u16,
    pub artifact_format_version: u16,
    pub artifact_magic_hex: String,
    pub semantic_schema_identity: String,
    pub schema_discovery: SchemaDiscoveryDescription,
    pub scalar_types: Vec<MachineScalarDescription>,
    pub requests: Vec<CodeDescription>,
    pub request_payloads: Vec<VariantPayloadDescription>,
    pub responses: Vec<CodeDescription>,
    pub response_payloads: Vec<VariantPayloadDescription>,
    pub identity_variants: Vec<NamedVariantDescription>,
    pub envelopes: Vec<NamedPayloadDescription>,
    pub id_formats: IdFormats,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticTypesAndNodesSection {
    pub semantic_types: Vec<CodeDescription>,
    pub node_kinds: Vec<CodeDescription>,
    pub name_contract: NameContractDescription,
    pub operations: Vec<OperationDescription>,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionsAndExpressionsSection {
    pub transaction_operations: Vec<CodeDescription>,
    pub transaction_operation_payloads: Vec<VariantPayloadDescription>,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub structured_authoring: StructuredAuthoringDescription,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueriesAndRepairSection {
    pub queries: Vec<CodeDescription>,
    pub query_payloads: Vec<VariantPayloadDescription>,
    pub query_result_payloads: Vec<VariantPayloadDescription>,
    pub query_records: Vec<NamedPayloadDescription>,
    pub query_variants: Vec<NamedVariantDescription>,
    pub query_member_payloads: Vec<VariantPayloadDescription>,
    pub query_cursor_payloads: Vec<VariantPayloadDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorsAndLimitsSection {
    pub errors: Vec<CodeDescription>,
    pub error_payload: PayloadShapeDescription,
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub boundary_error_kinds: Vec<String>,
    pub limits: BoundaryLimits,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeDescription {
    pub name: String,
    pub tag: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NameContractDescription {
    pub named_node_kinds: Vec<NodeKind>,
    pub minimum_utf8_bytes: u64,
    pub maximum_utf8_bytes: u64,
    pub sibling_uniqueness_groups: Vec<NameUniquenessGroupDescription>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NameUniquenessGroupDescription {
    pub name: String,
    pub owner_kind: NodeKind,
    pub member_kinds: Vec<NodeKind>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JsonScalarKind {
    Boolean,
    Number,
    String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum MachineScalarDomain {
    Boolean,
    Utf8String,
    SignedInteger {
        minimum: i64,
        maximum: i64,
    },
    UnsignedInteger {
        minimum: u64,
        maximum: u64,
    },
    LowercaseHex {
        encoded_bytes: u8,
    },
    NodeId {
        workspace_bytes: u8,
        minimum_serial: u64,
        maximum_serial: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MachineScalarDescription {
    pub name: String,
    pub json_kind: JsonScalarKind,
    pub domain: MachineScalarDomain,
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

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamedVariantDescription {
    pub name: String,
    pub tagging: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag_field: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_field: Option<String>,
    pub variants: Vec<VariantPayloadDescription>,
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
    pub records: Vec<NamedPayloadDescription>,
    pub variants: Vec<NamedVariantDescription>,
    pub limit_scope: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeValuePayload {
    None,
    Bool,
    I64,
    Product,
    Sum,
}
impl RuntimeValuePayload {
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::None => 1,
            Self::Bool => 2,
            Self::I64 => 3,
            Self::Product => 4,
            Self::Sum => 5,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::None),
            2 => Some(Self::Bool),
            3 => Some(Self::I64),
            4 => Some(Self::Product),
            5 => Some(Self::Sum),
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
    pub fields: Vec<MachineFieldDescription>,
    pub invariants: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredAuthoringDescription {
    pub draft_field_types: Vec<DraftFieldTypeDescription>,
    pub records: Vec<DraftRecordDescription>,
    pub expression_variants: Vec<DraftVariantDescription>,
    pub operation_variants: Vec<DraftVariantDescription>,
    pub value_variants: Vec<DraftVariantDescription>,
    pub type_variants: Vec<DraftVariantDescription>,
    pub expression_tagging: String,
    pub operation_tagging: String,
    pub value_tagging: String,
    pub type_tagging: String,
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
    TypeDraft,
    ProductFieldList,
    SumVariantList,
    ProductFieldValueList,
    MatchArmList,
    OperationMatchArmList,
}
impl DraftFieldType {
    pub const ALL: [Self; 22] = [
        Self::LocalHandle,
        Self::NodeTarget,
        Self::NodeId,
        Self::String,
        Self::SemanticType,
        Self::I64,
        Self::U8,
        Self::Value,
        Self::ValueList,
        Self::ExpressionKind,
        Self::ExpressionList,
        Self::ParameterList,
        Self::FunctionBody,
        Self::YieldingBody,
        Self::Bool,
        Self::Expression,
        Self::TypeDraft,
        Self::ProductFieldList,
        Self::SumVariantList,
        Self::ProductFieldValueList,
        Self::MatchArmList,
        Self::OperationMatchArmList,
    ];

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
            Self::TypeDraft => 17,
            Self::ProductFieldList => 18,
            Self::SumVariantList => 19,
            Self::ProductFieldValueList => 20,
            Self::MatchArmList => 21,
            Self::OperationMatchArmList => 22,
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
            17 => Some(Self::TypeDraft),
            18 => Some(Self::ProductFieldList),
            19 => Some(Self::SumVariantList),
            20 => Some(Self::ProductFieldValueList),
            21 => Some(Self::MatchArmList),
            22 => Some(Self::OperationMatchArmList),
            _ => None,
        }
    }

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::LocalHandle => "local_handle",
            Self::NodeTarget => "node_target",
            Self::NodeId => "node_id",
            Self::String => "string",
            Self::SemanticType => "semantic_type",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::Value => "value",
            Self::ValueList => "value_list",
            Self::ExpressionKind => "expression_kind",
            Self::ExpressionList => "expression_list",
            Self::ParameterList => "parameter_list",
            Self::FunctionBody => "function_body",
            Self::YieldingBody => "yielding_body",
            Self::Bool => "bool",
            Self::Expression => "expression",
            Self::TypeDraft => "type_draft",
            Self::ProductFieldList => "product_field_list",
            Self::SumVariantList => "sum_variant_list",
            Self::ProductFieldValueList => "product_field_value_list",
            Self::MatchArmList => "match_arm_list",
            Self::OperationMatchArmList => "operation_match_arm_list",
        }
    }

    pub const fn type_expression(self) -> &'static str {
        match self {
            Self::LocalHandle => "local_handle",
            Self::NodeTarget => "node_target",
            Self::NodeId => "node_id",
            Self::String => "string",
            Self::SemanticType => "semantic_type",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::Value => "value_draft",
            Self::ValueList => "list<value_draft>",
            Self::ExpressionKind => "expression_kind_draft",
            Self::ExpressionList => "list<expression>",
            Self::ParameterList => "list<function_parameter>",
            Self::FunctionBody => "function_body",
            Self::YieldingBody => "yielding_body",
            Self::Bool => "bool",
            Self::Expression => "expression",
            Self::TypeDraft => "type_draft",
            Self::ProductFieldList => "list<product_field>",
            Self::SumVariantList => "list<sum_variant>",
            Self::ProductFieldValueList => "list<product_field_value>",
            Self::MatchArmList => "list<match_arm>",
            Self::OperationMatchArmList => "list<operation_match_arm>",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFieldTypeDescription {
    pub tag: u8,
    pub name: String,
    pub type_expression: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftFieldDescription {
    pub name: String,
    pub field_type: DraftFieldType,
    pub required: bool,
    pub nullable: bool,
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
    pub region_arity: RegionArity,
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
    pub maximum_artifact_bytes: u64,
    pub maximum_artifact_name_bytes: u64,
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
    pub maximum_run_live_cells: u64,
    pub maximum_runtime_value_depth: u32,
    pub maximum_runtime_value_items: u64,
    pub maximum_runtime_value_bytes: u64,
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
    pub machine_schema_digest: String,
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
        unsigned("local_handle", 0, u32::MAX.into()),
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
            .map(|(variant, tag)| variant_payload(variant, tag, unit_payload()))
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
    declares_handle: bool,
) -> DraftFieldDescription {
    DraftFieldDescription {
        name: name.to_owned(),
        field_type,
        required,
        nullable: !required,
        declares_handle,
    }
}

fn structured_records() -> Vec<DraftRecordDescription> {
    use DraftFieldType as T;
    vec![
        DraftRecordDescription {
            name: "create_product_type".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("fields", T::ProductFieldList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "product_field".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("name", T::String, true, false),
                draft_field("ty", T::TypeDraft, true, false),
            ],
        },
        DraftRecordDescription {
            name: "create_sum_type".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("module", T::NodeTarget, true, false),
                draft_field("name", T::String, true, false),
                draft_field("variants", T::SumVariantList, true, false),
            ],
        },
        DraftRecordDescription {
            name: "sum_variant".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
                draft_field("name", T::String, true, false),
                draft_field("payload", T::TypeDraft, false, false),
            ],
        },
        DraftRecordDescription {
            name: "create_function".into(),
            fields: vec![
                draft_field("handle", T::LocalHandle, true, true),
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
                draft_field("handle", T::LocalHandle, true, true),
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
                draft_field("payload_handle", T::LocalHandle, false, true),
                draft_field("body", T::YieldingBody, true, false),
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
                draft_field("index_handle", T::LocalHandle, true, true),
                draft_field("carried_handle", T::LocalHandle, true, true),
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
        tag: code.stable_tag(),
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
        tag: code.stable_tag(),
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
            tag: 1,
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "bool".into(),
            tag: 2,
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "i64".into(),
            tag: 3,
            shape: PayloadShapeKind::Unit,
            newtype: None,
            fields: Vec::new(),
        },
        DraftVariantDescription {
            name: "nominal".into(),
            tag: 4,
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
    };
    DraftVariantDescription {
        name: code.machine_name().into(),
        tag: code.stable_tag(),
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
                variant_payload("unit", 1, unit_payload()),
                variant_payload("bool", 2, unit_payload()),
                variant_payload("i64", 3, unit_payload()),
                variant_payload("nominal", 4, newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "value_ref",
            vec![
                variant_payload("function_parameter", 1, newtype_payload("node_id")),
                variant_payload("block_argument", 2, newtype_payload("node_id")),
                variant_payload(
                    "operation_result",
                    3,
                    record_payload(&[("operation", "node_id", true), ("output", "u8", true)]),
                ),
            ],
        ),
        external_variant(
            "region_role",
            vec![
                variant_payload("if_then", 1, unit_payload()),
                variant_payload("if_else", 2, unit_payload()),
                variant_payload("for_body", 3, unit_payload()),
                variant_payload("match_arm", 4, newtype_payload("node_id")),
            ],
        ),
        named_variant(
            "operation_kind",
            vec![
                variant_payload("const_unit", 6, unit_payload()),
                variant_payload("const_i64", 1, newtype_payload("i64")),
                variant_payload("const_bool", 2, newtype_payload("bool")),
                variant_payload(
                    "add_i64",
                    3,
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "lt_i64",
                    7,
                    record_payload(&[("lhs", "value_ref", true), ("rhs", "value_ref", true)]),
                ),
                variant_payload(
                    "call",
                    8,
                    record_payload(&[
                        ("function", "node_id", true),
                        ("arguments", "list<value_ref>", true),
                    ]),
                ),
                variant_payload(
                    "hole",
                    4,
                    record_payload(&[("expected", "semantic_type", true)]),
                ),
                variant_payload(
                    "if",
                    9,
                    record_payload(&[
                        ("condition", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("then_region", "node_id", true),
                        ("else_region", "node_id", true),
                    ]),
                ),
                variant_payload(
                    "for_i64",
                    10,
                    record_payload(&[
                        ("start", "value_ref", true),
                        ("end_exclusive", "value_ref", true),
                        ("step", "i64", true),
                        ("initial", "value_ref", true),
                        ("carried", "semantic_type", true),
                        ("body_region", "node_id", true),
                    ]),
                ),
                variant_payload("return", 5, record_payload(&[("value", "value_ref", true)])),
                variant_payload("yield", 11, record_payload(&[("value", "value_ref", true)])),
                variant_payload(
                    "construct_product",
                    12,
                    record_payload(&[
                        ("product", "node_id", true),
                        ("fields", "list<canonical_product_field_value>", true),
                    ]),
                ),
                variant_payload(
                    "project_field",
                    13,
                    record_payload(&[("value", "value_ref", true), ("field", "node_id", true)]),
                ),
                variant_payload(
                    "construct_variant",
                    14,
                    record_payload(&[
                        ("variant", "node_id", true),
                        ("payload", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "match_sum",
                    15,
                    record_payload(&[
                        ("scrutinee", "value_ref", true),
                        ("result", "semantic_type", true),
                        ("arms", "list<canonical_match_arm>", true),
                    ]),
                ),
            ],
        ),
        named_variant(
            "node",
            vec![
                variant_payload(
                    "workspace_root",
                    1,
                    record_payload(&[("packages", "list<node_id>", true)]),
                ),
                variant_payload(
                    "package",
                    2,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("modules", "list<node_id>", true),
                        ("entry", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "module",
                    3,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("types", "list<node_id>", true),
                        ("functions", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_type",
                    10,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("fields", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "product_field",
                    11,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "sum_type",
                    12,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("name", "string", true),
                        ("variants", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "sum_variant",
                    13,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("payload", "semantic_type", false),
                    ]),
                ),
                variant_payload(
                    "function",
                    4,
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
                    5,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("name", "string", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "region",
                    6,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("blocks", "list<node_id>", true),
                    ]),
                ),
                variant_payload(
                    "block",
                    7,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("arguments", "list<node_id>", true),
                        ("operations", "list<node_id>", true),
                        ("terminator", "node_id", false),
                    ]),
                ),
                variant_payload(
                    "block_argument",
                    9,
                    record_payload(&[
                        ("owner", "node_id", true),
                        ("ordinal", "u32", true),
                        ("ty", "semantic_type", true),
                    ]),
                ),
                variant_payload(
                    "operation",
                    8,
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
            &[("return_handles", "list<local_handle>", true)],
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
                    "list<tuple<local_handle,node_id>>",
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
                variant_payload("existing", 1, newtype_payload("node_id")),
                variant_payload("local", 2, newtype_payload("local_handle")),
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
            variant_payload("unit", 1, unit_payload()),
            variant_payload("bool", 2, newtype_payload("bool")),
            variant_payload("i64", 3, newtype_payload("i64")),
            variant_payload("product", 4, newtype_payload("runtime_product_data")),
            variant_payload("sum", 5, newtype_payload("runtime_sum_data")),
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
        ResponseCode::DescribeSchema => newtype_payload("describe_schema_result"),
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
        TransactionOpCode::CreateProductType => record_payload(&[
            ("handle", "local_handle", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("fields", "list<product_field>", true),
        ]),
        TransactionOpCode::CreateSumType => record_payload(&[
            ("handle", "local_handle", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("variants", "list<sum_variant>", true),
        ]),
        TransactionOpCode::CreateFunction => record_payload(&[
            ("handle", "local_handle", true),
            ("module", "node_target", true),
            ("name", "string", true),
            ("parameters", "list<function_parameter>", true),
            ("result", "type_draft", true),
            ("body", "function_body", false),
        ]),
        TransactionOpCode::DefineFunctionBody => record_payload(&[
            ("function", "node_target", true),
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
        QueryCode::NominalType => record_payload(&[
            ("declaration", "node_id", true),
            ("page", "page_request", true),
        ]),
    };
    variant_payload(code.machine_name(), code.stable_tag(), payload)
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
    variant_payload(code.machine_name(), code.stable_tag(), newtype_payload(ty))
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
                variant_payload("success", 1, newtype_payload("query_result")),
                variant_payload("error", 2, newtype_payload("error")),
            ],
        ),
        named_variant(
            "repair_target",
            vec![
                variant_payload("hole", 1, newtype_payload("node_id")),
                variant_payload(
                    "operand",
                    2,
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
                variant_payload("i64", 1, newtype_payload("i64")),
                variant_payload("bool", 2, newtype_payload("bool")),
                variant_payload("expected_type", 3, newtype_payload("semantic_type")),
            ],
        ),
        named_variant(
            "dependency_fact",
            vec![
                variant_payload(
                    "value_operand",
                    1,
                    record_payload(&[("index", "u64", true), ("value", "value_ref", true)]),
                ),
                variant_payload(
                    "definition",
                    2,
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
                variant_payload("i64", 1, newtype_payload("i64")),
                variant_payload("bool", 2, newtype_payload("bool")),
                variant_payload("type", 3, newtype_payload("semantic_type")),
            ],
        ),
        named_variant(
            "change_kind",
            vec![
                variant_payload("created", 1, record_payload(&[("kind", "node_kind", true)])),
                variant_payload("deleted", 2, record_payload(&[("kind", "node_kind", true)])),
                variant_payload(
                    "renamed",
                    3,
                    record_payload(&[("before", "string", true), ("after", "string", true)]),
                ),
                variant_payload(
                    "scalar_attribute_changed",
                    4,
                    record_payload(&[
                        ("before", "scalar_value", true),
                        ("after", "scalar_value", true),
                    ]),
                ),
                variant_payload(
                    "containment_changed",
                    5,
                    record_payload(&[("before_count", "u64", true), ("after_count", "u64", true)]),
                ),
                variant_payload(
                    "operand_changed",
                    6,
                    record_payload(&[
                        ("index", "u64", true),
                        ("before", "value_ref", false),
                        ("after", "value_ref", false),
                    ]),
                ),
                variant_payload(
                    "definition_changed",
                    7,
                    record_payload(&[("before", "node_id", true), ("after", "node_id", true)]),
                ),
                variant_payload(
                    "entry_function_changed",
                    8,
                    record_payload(&[("before", "node_id", false), ("after", "node_id", false)]),
                ),
                variant_payload(
                    "completeness_changed",
                    9,
                    record_payload(&[("complete", "bool", true)]),
                ),
                variant_payload(
                    "operation_refined",
                    10,
                    record_payload(&[
                        ("before", "operation_code", true),
                        ("after", "operation_code", true),
                        ("result_type", "semantic_type", true),
                        ("replacement", "operation_kind", true),
                    ]),
                ),
                variant_payload("allocated_and_tombstoned", 11, unit_payload()),
            ],
        ),
    ]
}

fn query_member_payloads() -> Vec<VariantPayloadDescription> {
    vec![
        variant_payload(
            "product_field",
            1,
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
            2,
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
        variant_payload("blockers", 1, common(&[("next", "u64", true)])),
        variant_payload(
            "owner_chain",
            2,
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "body",
            3,
            common(&[("block", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "incoming_uses",
            4,
            common(&[("value", "value_ref", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "definition_references",
            5,
            common(&[("target", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "dependencies",
            6,
            common(&[("node", "node_id", true), ("next", "u64", true)]),
        ),
        variant_payload(
            "visible_values",
            7,
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
            8,
            common(&[
                ("target", "repair_target", true),
                ("expected", "semantic_type", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "diff",
            9,
            record_payload(&[
                ("workspace", "workspace_id", true),
                ("from", "revision", true),
                ("to", "revision", true),
                ("next", "u64", true),
            ]),
        ),
        variant_payload(
            "nominal_type",
            10,
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
                ("tag", "u8", true),
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
        record(
            "code_description",
            &[("name", "string", true), ("tag", "u8", true)],
        ),
        record(
            "draft_field_type_description",
            &[
                ("tag", "u8", true),
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
                ("declares_handle", "bool", true),
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
                ("tag", "u8", true),
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
                ("explicit_handles_are_selectable", "bool", true),
                ("implicit_handles_are_selectable", "bool", true),
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
                ("tag", "u8", true),
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
                ("tag", "u8", true),
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
                ("sections", "list<code_description>", true),
                (
                    "section_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("maximum_sections_per_request", "u8", true),
                ("full_available", "bool", true),
                ("known_digest_match_precedes_projection", "bool", true),
            ],
        ),
        record(
            "schema_description",
            &[
                ("machine_schema_identity", "string", true),
                ("binary_protocol_version", "u16", true),
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
                ("id_formats", "id_formats", true),
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
                ("maximum_frame_bytes", "u64", true),
                ("maximum_frame_items", "u64", true),
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
                ("maximum_error_related_ids", "u32", true),
                ("maximum_persistence_head_bytes", "u64", true),
            ],
        ),
        record(
            "id_formats",
            &[
                ("workspace", "string", true),
                ("idempotency_key", "string", true),
                ("node", "string", true),
                ("snapshot_hash", "string", true),
                ("change_digest", "string", true),
                ("revision", "string", true),
                ("request_id", "string", true),
                ("query_id", "string", true),
                ("local_handle", "string", true),
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
                ("binary_protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("sections", "list<code_description>", true),
                ("maximum_sections_per_request", "u8", true),
                ("full_available", "bool", true),
                ("maximum_frame_bytes", "u64", true),
                ("maximum_json_output_bytes", "u64", true),
            ],
        ),
        record(
            "schema_sections",
            &[
                ("digest", "machine_schema_digest", true),
                ("sections", "list<schema_section_payload>", true),
            ],
        ),
        record(
            "identity_and_envelopes_section",
            &[
                ("machine_schema_identity", "string", true),
                ("binary_protocol_version", "u16", true),
                ("json_envelope_version", "u16", true),
                ("artifact_format_version", "u16", true),
                ("artifact_magic_hex", "string", true),
                ("semantic_schema_identity", "string", true),
                ("schema_discovery", "schema_discovery_description", true),
                ("scalar_types", "list<machine_scalar_description>", true),
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
                ("id_formats", "id_formats", true),
            ],
        ),
        record(
            "semantic_types_and_nodes_section",
            &[
                ("semantic_types", "list<code_description>", true),
                ("node_kinds", "list<code_description>", true),
                ("name_contract", "name_contract_description", true),
                ("operations", "list<operation_description>", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
            ],
        ),
        record(
            "nominal_declarations_section",
            &[
                ("declaration_kinds", "list<node_kind>", true),
                ("member_kinds", "list<node_kind>", true),
                ("shape_invariants", "list<string>", true),
                ("layout_invariants", "list<string>", true),
            ],
        ),
        record(
            "transactions_and_expressions_section",
            &[
                ("transaction_operations", "list<code_description>", true),
                (
                    "transaction_operation_payloads",
                    "list<variant_payload_description>",
                    true,
                ),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                (
                    "structured_authoring",
                    "structured_authoring_description",
                    true,
                ),
            ],
        ),
        record(
            "queries_and_repair_section",
            &[
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
            ],
        ),
        record(
            "runtime_and_run_section",
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
            "errors_and_limits_section",
            &[
                ("errors", "list<code_description>", true),
                ("error_payload", "payload_shape_description", true),
                ("records", "list<named_payload_description>", true),
                ("variants", "list<named_variant_description>", true),
                ("boundary_error_kinds", "list<string>", true),
                ("limits", "boundary_limits", true),
            ],
        ),
    ]
}

fn schema_discovery_variants(
    projection_payloads: &[VariantPayloadDescription],
    result_payloads: &[VariantPayloadDescription],
    section_payloads: &[VariantPayloadDescription],
) -> Vec<NamedVariantDescription> {
    vec![
        named_variant("schema_projection", projection_payloads.to_vec()),
        named_variant("describe_schema_result", result_payloads.to_vec()),
        named_variant("schema_section_payload", section_payloads.to_vec()),
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
                variant_payload("boolean", 1, unit_payload()),
                variant_payload("utf8_string", 2, unit_payload()),
                variant_payload(
                    "signed_integer",
                    3,
                    record_payload(&[("minimum", "i64", true), ("maximum", "i64", true)]),
                ),
                variant_payload(
                    "unsigned_integer",
                    4,
                    record_payload(&[("minimum", "u64", true), ("maximum", "u64", true)]),
                ),
                variant_payload(
                    "lowercase_hex",
                    5,
                    record_payload(&[("encoded_bytes", "u8", true)]),
                ),
                variant_payload(
                    "node_id",
                    6,
                    record_payload(&[
                        ("workspace_bytes", "u8", true),
                        ("minimum_serial", "u64", true),
                        ("maximum_serial", "u64", true),
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
                ("product", 4),
                ("sum", 5),
            ],
        ),
        unit_variants(
            "draft_field_type",
            DraftFieldType::ALL
                .into_iter()
                .map(|field_type| (field_type.machine_name(), field_type.stable_tag())),
        ),
        named_variant(
            "operand_arity",
            vec![
                variant_payload("fixed", 1, newtype_payload("u8")),
                variant_payload("call_target_parameters", 2, unit_payload()),
                variant_payload("product_fields", 3, unit_payload()),
                variant_payload("variant_payload", 4, unit_payload()),
            ],
        ),
        named_variant(
            "region_arity",
            vec![
                variant_payload("fixed", 1, newtype_payload("u8")),
                variant_payload(
                    "match_variants",
                    2,
                    record_payload(&[
                        ("payload_type", "type_rule", true),
                        ("terminator", "operation_code", true),
                        ("yield_type", "type_rule", true),
                    ]),
                ),
            ],
        ),
        unit_variants("operand_use", [("copy", 1)]),
        unit_variants(
            "literal_field",
            [
                ("i64_value", 1),
                ("bool_value", 2),
                ("expected_type", 3),
                ("result_type", 4),
                ("carried_type", 5),
                ("positive_step", 6),
            ],
        ),
        unit_variants(
            "block_argument_role",
            [("loop_index", 1), ("loop_carried", 2), ("match_payload", 3)],
        ),
        named_variant(
            "type_rule",
            vec![
                variant_payload("fixed", 1, newtype_payload("semantic_type")),
                variant_payload("payload_expected", 2, unit_payload()),
                variant_payload("owner_function_result", 3, unit_payload()),
                variant_payload("payload_result", 4, unit_payload()),
                variant_payload("payload_carried", 5, unit_payload()),
                variant_payload("call_target_parameter", 6, unit_payload()),
                variant_payload("call_target_result", 7, unit_payload()),
                variant_payload("owning_region_yield", 8, unit_payload()),
                variant_payload("product_field_type", 9, unit_payload()),
                variant_payload("product_declaration_result", 10, unit_payload()),
                variant_payload("projection_owner", 11, unit_payload()),
                variant_payload("projected_field_result", 12, unit_payload()),
                variant_payload("variant_payload", 13, unit_payload()),
                variant_payload("variant_owner_result", 14, unit_payload()),
                variant_payload("match_scrutinee", 15, unit_payload()),
                variant_payload("match_result", 16, unit_payload()),
            ],
        ),
    ]
}

fn schema_discovery_description() -> SchemaDiscoveryDescription {
    let sections = SchemaSection::ALL
        .into_iter()
        .map(|section| described(section.machine_name(), section.stable_tag()))
        .collect::<Vec<_>>();
    let projection_payloads = vec![
        variant_payload("manifest", 1, unit_payload()),
        variant_payload(
            "sections",
            2,
            record_payload(&[("sections", "list<schema_section>", true)]),
        ),
        variant_payload("full", 3, unit_payload()),
    ];
    let result_payloads = vec![
        variant_payload(
            "unchanged",
            1,
            record_payload(&[("digest", "machine_schema_digest", true)]),
        ),
        variant_payload("manifest", 2, newtype_payload("schema_manifest")),
        variant_payload("sections", 3, newtype_payload("schema_sections")),
        variant_payload(
            "full",
            4,
            record_payload(&[
                ("digest", "machine_schema_digest", true),
                ("description", "schema_description", true),
            ]),
        ),
    ];
    let section_payloads = sections
        .iter()
        .map(|section| {
            variant_payload(
                &section.name,
                section.tag,
                newtype_payload(&format!("{}_section", section.name)),
            )
        })
        .collect::<Vec<_>>();
    SchemaDiscoveryDescription {
        digest_format: "64 lowercase hexadecimal characters encoding 32 bytes".into(),
        digest_domain: MACHINE_SCHEMA_DIGEST_DOMAIN.into(),
        request: record_payload(&[
            ("projection", "schema_projection", true),
            ("known_digest", "machine_schema_digest", false),
        ]),
        records: schema_discovery_records(),
        variants: schema_discovery_variants(
            &projection_payloads,
            &result_payloads,
            &section_payloads,
        ),
        projection_payloads,
        result_payloads,
        section_payloads,
        sections,
        maximum_sections_per_request: MAX_SCHEMA_SECTIONS as u8,
        full_available: true,
        known_digest_match_precedes_projection: true,
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
        binary_protocol_version: PROTOCOL_VERSION,
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
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .chain(std::iter::once(described("nominal", 4)))
            .collect(),
        node_kinds: NodeKind::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        name_contract: name_contract_description(),
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
            .map(|code| described(code.machine_name(), code.stable_tag()))
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
                    tag: field_type.stable_tag(),
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
                "product_field".into(),
                "sum_variant".into(),
                "function_body".into(),
                "yielding_body".into(),
                "expression".into(),
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
                    tag: code.stable_tag(),
                    payload: match code {
                        crate::interpret::RuntimeValueCode::Unit => RuntimeValuePayload::None,
                        crate::interpret::RuntimeValueCode::Bool => RuntimeValuePayload::Bool,
                        crate::interpret::RuntimeValueCode::I64 => RuntimeValuePayload::I64,
                        crate::interpret::RuntimeValueCode::Product => RuntimeValuePayload::Product,
                        crate::interpret::RuntimeValueCode::Sum => RuntimeValuePayload::Sum,
                    },
                    fields: match code {
                        crate::interpret::RuntimeValueCode::Unit => vec![],
                        crate::interpret::RuntimeValueCode::Bool => vec![MachineFieldDescription { name: "data".into(), type_expression: "bool".into(), required: true }],
                        crate::interpret::RuntimeValueCode::I64 => vec![MachineFieldDescription { name: "data".into(), type_expression: "i64".into(), required: true }],
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
                        _ => vec!["value must have the exact primitive semantic type".into()],
                    },
                })
                .collect(),
            records: run_records(),
            variants: run_variants(),
            limit_scope: vec![
                "argument count applies to the complete Run arguments list".into(),
                "runtime value depth applies per nested value root; item and encoded-byte limits aggregate across all Run arguments".into(),
                "live-cell policy applies to peak frame arenas plus argument, edge, return, and public flatten scratch before allocation or copy".into(),
                "fuel charges before work: one base per instruction or transfer plus max(1, materialized cells) for every logically copied value; variant construction charges its full canonical sum cells".into(),
            ],
        },
        queries: QueryCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
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
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        error_payload: error_payload(),
        error_records: error_records(),
        error_variants: error_variants(),
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
            maximum_frame_bytes: crate::protocol::MAXIMUM_FRAME_BYTES as u64,
            maximum_frame_items: crate::protocol::MAXIMUM_FRAME_ITEMS as u64,
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
    schema.semantic_types.sort_by_key(|item| item.tag);
    schema.node_kinds.sort_by_key(|item| item.tag);
    schema
        .name_contract
        .named_node_kinds
        .sort_by_key(|item| item.stable_tag());
    schema
        .name_contract
        .sibling_uniqueness_groups
        .sort_by(|left, right| left.name.cmp(&right.name));
    for group in &mut schema.name_contract.sibling_uniqueness_groups {
        group.member_kinds.sort_by_key(|item| item.stable_tag());
    }
    schema.operations.sort_by_key(|item| item.tag);
    schema
        .semantic_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .semantic_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.semantic_variants {
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema.transaction_operations.sort_by_key(|item| item.tag);
    schema
        .transaction_operation_payloads
        .sort_by_key(|item| item.tag);
    schema
        .transaction_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .transaction_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.transaction_variants {
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema
        .structured_authoring
        .draft_field_types
        .sort_by_key(|item| item.tag);
    schema
        .structured_authoring
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .structured_authoring
        .expression_variants
        .sort_by_key(|item| item.tag);
    schema
        .structured_authoring
        .operation_variants
        .sort_by_key(|item| item.tag);
    schema
        .structured_authoring
        .value_variants
        .sort_by_key(|item| item.tag);
    schema
        .structured_authoring
        .type_variants
        .sort_by_key(|item| item.tag);
    schema
        .structured_authoring
        .implicit_node_kinds
        .sort_by_key(|item| item.stable_tag());
    schema.structured_authoring.counted_item_categories.sort();
    schema.run.runtime_values.sort_by_key(|item| item.tag);
    schema
        .run
        .records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .run
        .variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.run.variants {
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema.queries.sort_by_key(|item| item.tag);
    schema.query_payloads.sort_by_key(|item| item.tag);
    schema.query_result_payloads.sort_by_key(|item| item.tag);
    schema
        .query_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .query_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.query_variants {
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema.query_member_payloads.sort_by_key(|item| item.tag);
    schema.query_cursor_payloads.sort_by_key(|item| item.tag);
    schema.errors.sort_by_key(|item| item.tag);
    schema
        .error_records
        .sort_by(|left, right| left.name.cmp(&right.name));
    schema
        .error_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.error_variants {
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema.requests.sort_by_key(|item| item.tag);
    schema.request_payloads.sort_by_key(|item| item.tag);
    schema.responses.sort_by_key(|item| item.tag);
    schema.response_payloads.sort_by_key(|item| item.tag);
    schema
        .identity_variants
        .sort_by(|left, right| left.name.cmp(&right.name));
    for variant in &mut schema.identity_variants {
        variant.variants.sort_by_key(|item| item.tag);
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
        variant.variants.sort_by_key(|item| item.tag);
    }
    schema
        .schema_discovery
        .projection_payloads
        .sort_by_key(|item| item.tag);
    schema
        .schema_discovery
        .result_payloads
        .sort_by_key(|item| item.tag);
    schema
        .schema_discovery
        .sections
        .sort_by_key(|item| item.tag);
    schema
        .schema_discovery
        .section_payloads
        .sort_by_key(|item| item.tag);
    schema
        .nominal_declarations
        .declaration_kinds
        .sort_by_key(|item| item.stable_tag());
    schema
        .nominal_declarations
        .member_kinds
        .sort_by_key(|item| item.stable_tag());
    schema
}

pub fn machine_schema_digest(
    description: &SchemaDescription,
) -> crate::Result<MachineSchemaDigest> {
    let canonical = canonicalize_schema(description.clone());
    let bytes = crate::protocol::canonical_schema_facts_bytes(&canonical)?;
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
    let digest = machine_schema_digest(&description).map_err(|error| error.to_string())?;
    if request.known_digest == Some(digest) {
        return Ok(DescribeSchemaResult::Unchanged { digest });
    }
    match &request.projection {
        SchemaProjection::Manifest => Ok(DescribeSchemaResult::Manifest(schema_manifest(
            &description,
            digest,
        ))),
        SchemaProjection::Sections { sections } => {
            let mut sections = sections.clone();
            sections.sort_unstable();
            Ok(DescribeSchemaResult::Sections(SchemaSections {
                digest,
                sections: sections
                    .into_iter()
                    .map(|section| section_payload(&description, section))
                    .collect(),
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
        binary_protocol_version: description.binary_protocol_version,
        json_envelope_version: description.json_envelope_version,
        artifact_format_version: description.artifact_format_version,
        artifact_magic_hex: description.artifact_magic_hex.clone(),
        semantic_schema_identity: description.semantic_schema_identity.clone(),
        sections: SchemaSection::ALL
            .into_iter()
            .map(|section| described(section.machine_name(), section.stable_tag()))
            .collect(),
        maximum_sections_per_request: MAX_SCHEMA_SECTIONS as u8,
        full_available: true,
        maximum_frame_bytes: description.limits.maximum_frame_bytes,
        maximum_json_output_bytes: description.limits.maximum_json_output_bytes,
    }
}

fn section_payload(
    description: &SchemaDescription,
    section: SchemaSection,
) -> SchemaSectionPayload {
    match section {
        SchemaSection::IdentityAndEnvelopes => {
            SchemaSectionPayload::IdentityAndEnvelopes(Box::new(IdentityAndEnvelopesSection {
                machine_schema_identity: description.machine_schema_identity.clone(),
                binary_protocol_version: description.binary_protocol_version,
                json_envelope_version: description.json_envelope_version,
                artifact_format_version: description.artifact_format_version,
                artifact_magic_hex: description.artifact_magic_hex.clone(),
                semantic_schema_identity: description.semantic_schema_identity.clone(),
                schema_discovery: description.schema_discovery.clone(),
                scalar_types: description.scalar_types.clone(),
                requests: description.requests.clone(),
                request_payloads: description.request_payloads.clone(),
                responses: description.responses.clone(),
                response_payloads: description.response_payloads.clone(),
                identity_variants: description.identity_variants.clone(),
                envelopes: description.envelopes.clone(),
                id_formats: description.id_formats.clone(),
            }))
        }
        SchemaSection::SemanticTypesAndNodes => {
            SchemaSectionPayload::SemanticTypesAndNodes(Box::new(SemanticTypesAndNodesSection {
                semantic_types: description.semantic_types.clone(),
                node_kinds: description.node_kinds.clone(),
                name_contract: description.name_contract.clone(),
                operations: description.operations.clone(),
                records: description.semantic_records.clone(),
                variants: description.semantic_variants.clone(),
            }))
        }
        SchemaSection::NominalDeclarations => SchemaSectionPayload::NominalDeclarations(Box::new(
            description.nominal_declarations.clone(),
        )),
        SchemaSection::TransactionsAndExpressions => {
            SchemaSectionPayload::TransactionsAndExpressions(Box::new(
                TransactionsAndExpressionsSection {
                    transaction_operations: description.transaction_operations.clone(),
                    transaction_operation_payloads: description
                        .transaction_operation_payloads
                        .clone(),
                    records: description.transaction_records.clone(),
                    variants: description.transaction_variants.clone(),
                    structured_authoring: description.structured_authoring.clone(),
                },
            ))
        }
        SchemaSection::QueriesAndRepair => {
            SchemaSectionPayload::QueriesAndRepair(Box::new(QueriesAndRepairSection {
                queries: description.queries.clone(),
                query_payloads: description.query_payloads.clone(),
                query_result_payloads: description.query_result_payloads.clone(),
                query_records: description.query_records.clone(),
                query_variants: description.query_variants.clone(),
                query_member_payloads: description.query_member_payloads.clone(),
                query_cursor_payloads: description.query_cursor_payloads.clone(),
            }))
        }
        SchemaSection::RuntimeAndRun => {
            SchemaSectionPayload::RuntimeAndRun(Box::new(description.run.clone()))
        }
        SchemaSection::ErrorsAndLimits => {
            SchemaSectionPayload::ErrorsAndLimits(Box::new(ErrorsAndLimitsSection {
                errors: description.errors.clone(),
                error_payload: description.error_payload.clone(),
                records: description.error_records.clone(),
                variants: description.error_variants.clone(),
                boundary_error_kinds: description.boundary_error_kinds.clone(),
                limits: description.limits.clone(),
            }))
        }
    }
}

pub fn all_schema_sections(description: &SchemaDescription) -> Vec<SchemaSectionPayload> {
    SchemaSection::ALL
        .into_iter()
        .map(|section| section_payload(description, section))
        .collect()
}

pub fn reconstruct_schema_from_sections(
    sections: &[SchemaSectionPayload],
) -> Option<SchemaDescription> {
    let [
        SchemaSectionPayload::IdentityAndEnvelopes(identity),
        SchemaSectionPayload::SemanticTypesAndNodes(semantic),
        SchemaSectionPayload::NominalDeclarations(nominal),
        SchemaSectionPayload::TransactionsAndExpressions(transactions),
        SchemaSectionPayload::QueriesAndRepair(queries),
        SchemaSectionPayload::RuntimeAndRun(run),
        SchemaSectionPayload::ErrorsAndLimits(errors),
    ] = sections
    else {
        return None;
    };
    Some(SchemaDescription {
        machine_schema_identity: identity.machine_schema_identity.clone(),
        binary_protocol_version: identity.binary_protocol_version,
        json_envelope_version: identity.json_envelope_version,
        artifact_format_version: identity.artifact_format_version,
        artifact_magic_hex: identity.artifact_magic_hex.clone(),
        semantic_schema_identity: identity.semantic_schema_identity.clone(),
        schema_discovery: identity.schema_discovery.clone(),
        scalar_types: identity.scalar_types.clone(),
        semantic_types: semantic.semantic_types.clone(),
        node_kinds: semantic.node_kinds.clone(),
        name_contract: semantic.name_contract.clone(),
        operations: semantic.operations.clone(),
        semantic_records: semantic.records.clone(),
        semantic_variants: semantic.variants.clone(),
        transaction_operations: transactions.transaction_operations.clone(),
        transaction_operation_payloads: transactions.transaction_operation_payloads.clone(),
        transaction_records: transactions.records.clone(),
        transaction_variants: transactions.variants.clone(),
        structured_authoring: transactions.structured_authoring.clone(),
        run: run.as_ref().clone(),
        queries: queries.queries.clone(),
        query_payloads: queries.query_payloads.clone(),
        query_result_payloads: queries.query_result_payloads.clone(),
        query_records: queries.query_records.clone(),
        query_variants: queries.query_variants.clone(),
        query_member_payloads: queries.query_member_payloads.clone(),
        query_cursor_payloads: queries.query_cursor_payloads.clone(),
        errors: errors.errors.clone(),
        error_payload: errors.error_payload.clone(),
        error_records: errors.records.clone(),
        error_variants: errors.variants.clone(),
        requests: identity.requests.clone(),
        request_payloads: identity.request_payloads.clone(),
        responses: identity.responses.clone(),
        response_payloads: identity.response_payloads.clone(),
        identity_variants: identity.identity_variants.clone(),
        envelopes: identity.envelopes.clone(),
        boundary_error_kinds: errors.boundary_error_kinds.clone(),
        limits: errors.limits.clone(),
        id_formats: identity.id_formats.clone(),
        nominal_declarations: nominal.as_ref().clone(),
    })
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
    use crate::query::{
        ContextBudget, PageRequest, Query, QueryBatchRequest, QueryItem, VisibleCursorPurpose,
    };
    use crate::transaction::{
        ExpressionDraft, ExpressionKindDraft, Transaction, TransactionMode, TransactionOp,
        TransactionReceipt, TransactionResponseSpec, YieldingBodyDraft,
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
            text.replacen("\"version\":4", "\"version\":3", 1),
            text.replacen("\"request_id\":1", "\"request_id\":0", 1),
            text.replacen("{\"version\":4", "{\"unknown\":0,\"version\":4", 1),
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
        assert_eq!(schema.scalar_types.len(), 17);
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
                NodeTarget::Local(LocalHandle::new(1)),
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
                SchemaProjection::Sections {
                    sections: vec![SchemaSection::IdentityAndEnvelopes],
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
                DescribeSchemaResult::Sections(SchemaSections {
                    digest,
                    sections: vec![],
                }),
                DescribeSchemaResult::Full {
                    digest,
                    description: Box::new(schema.clone()),
                },
            ],
        );
        assert_family_samples(
            &schema,
            "schema_section_payload",
            &all_schema_sections(&schema),
        );
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
                MachineScalarDomain::NodeId {
                    workspace_bytes: 16,
                    minimum_serial: 1,
                    maximum_serial: u64::MAX,
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
        assert_family_samples(&schema, "operand_use", &[OperandUse::Copy]);
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
        for section in SchemaSection::ALL {
            validate_exact_type_expression(
                &schema,
                "schema_section",
                &serde_json::to_value(section).expect("section JSON"),
                None,
            )
            .expect("schema section contract");
        }
    }

    #[test]
    fn schema_json_is_strict_and_rejects_invalid_section_contracts() {
        let envelope = RequestEnvelope {
            version: JSON_ENVELOPE_VERSION,
            request_id: RequestId::new(2),
            request: Request::DescribeSchema(DescribeSchemaRequest {
                projection: SchemaProjection::Sections {
                    sections: vec![SchemaSection::RuntimeAndRun],
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
            valid.replace("runtime_and_run", "unknown_section"),
            valid.replace(
                "\"runtime_and_run\"",
                "\"runtime_and_run\",\"runtime_and_run\"",
            ),
            valid.replace("[\"runtime_and_run\"]", "[]"),
            valid.replace(
                "[\"runtime_and_run\"]",
                "[\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\",\"runtime_and_run\"]",
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
                        condition: ValueDraft::FunctionParameter(NodeTarget::Existing(
                            NodeId::new(workspace, 3).expect("parameter"),
                        )),
                        result: SemanticType::I64.into(),
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
                ("match_arm", "payload_handle", true),
                ("insert_expression", "before", true),
                ("construct_variant", "payload", true),
                ("construct_variant", "payload", true),
            ]
        );
        let workspace = WorkspaceId::from_bytes([0x71; 16]);
        let node = NodeId::new(workspace, 7).expect("node");
        let handle = LocalHandle::new(1);
        let body = crate::transaction::FunctionBodyDraft {
            operations: vec![],
            return_value: ValueDraft::FunctionParameter(NodeTarget::Existing(node)),
        };

        let create_function = TransactionOp::CreateFunction {
            handle,
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
                handle,
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
            handle,
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
            payload_handle: Some(handle),
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
            returned_bindings: vec![(handle, node)],
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
            yield_value: value,
        };
        let function_body = FunctionBodyDraft {
            operations: vec![],
            return_value: value,
        };
        let expression = ExpressionDraft {
            handle: LocalHandle::new(20),
            operation: ExpressionKindDraft::ConstI64(1),
        };
        let transaction_samples = vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "p".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: target,
                name: "m".into(),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: target,
                name: "f".into(),
                parameters: vec![FunctionParameterDraft {
                    handle: LocalHandle::new(4),
                    name: "x".into(),
                    ty: TypeDraft::I64,
                }],
                result: TypeDraft::I64,
                body: Some(function_body.clone()),
            },
            TransactionOp::DefineFunctionBody {
                function: target,
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
                value,
            },
            TransactionOp::DeleteOwnedSubtree { root: target },
            TransactionOp::RefineHole {
                hole: target,
                replacement: OperationDraft::ConstI64(1),
            },
            TransactionOp::CreateProductType {
                handle: LocalHandle::new(5),
                module: target,
                name: "pair".into(),
                fields: vec![ProductFieldDraft {
                    handle: LocalHandle::new(6),
                    name: "value".into(),
                    ty: TypeDraft::I64,
                }],
            },
            TransactionOp::CreateSumType {
                handle: LocalHandle::new(7),
                module: target,
                name: "maybe".into(),
                variants: vec![SumVariantDraft {
                    handle: LocalHandle::new(8),
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
                code.stable_tag(),
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
                code.stable_tag(),
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
            use_mode: OperandUse::Copy,
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
            operand_uses: vec![OperandUse::Copy],
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
                description.tag,
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
            use_mode: Some(OperandUse::Copy),
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
                code.stable_tag(),
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
                description.tag,
            );
        }

        let expression_samples = vec![
            ExpressionKindDraft::ConstUnit,
            ExpressionKindDraft::ConstBool(true),
            ExpressionKindDraft::ConstI64(1),
            ExpressionKindDraft::AddI64 {
                lhs: value,
                rhs: value,
            },
            ExpressionKindDraft::LtI64 {
                lhs: value,
                rhs: value,
            },
            ExpressionKindDraft::Call {
                function: target,
                arguments: vec![value],
            },
            ExpressionKindDraft::Hole {
                expected: TypeDraft::I64,
            },
            ExpressionKindDraft::If {
                condition: value,
                result: TypeDraft::I64,
                then_body: yielding.clone(),
                else_body: yielding.clone(),
            },
            ExpressionKindDraft::ForI64 {
                start: value,
                end_exclusive: value,
                step: 1,
                initial: value,
                carried: TypeDraft::I64,
                index_handle: LocalHandle::new(30),
                carried_handle: LocalHandle::new(31),
                body: yielding.clone(),
            },
            ExpressionKindDraft::ConstructProduct {
                product: target,
                fields: vec![ProductFieldValueDraft {
                    field: target,
                    value,
                }],
            },
            ExpressionKindDraft::ProjectField {
                value,
                field: target,
            },
            ExpressionKindDraft::ConstructVariant {
                variant: target,
                payload: Some(value),
            },
            ExpressionKindDraft::MatchSum {
                scrutinee: value,
                result: TypeDraft::I64,
                arms: vec![MatchArmDraft {
                    variant: target,
                    payload_handle: Some(LocalHandle::new(32)),
                    body: yielding.clone(),
                }],
            },
        ];
        assert_eq!(expression_samples.len(), ExpressionDraftCode::ALL.len());
        for (sample, code) in expression_samples.iter().zip(ExpressionDraftCode::ALL) {
            assert_draft_variant_serde_contract(
                sample,
                &schema,
                &schema.structured_authoring.expression_variants,
                code.machine_name(),
                code.stable_tag(),
            );
        }

        let operation_samples = vec![
            OperationDraft::ConstUnit,
            OperationDraft::ConstI64(1),
            OperationDraft::ConstBool(true),
            OperationDraft::AddI64 {
                lhs: value,
                rhs: value,
            },
            OperationDraft::LtI64 {
                lhs: value,
                rhs: value,
            },
            OperationDraft::Call {
                function: target,
                arguments: vec![value],
            },
            OperationDraft::Hole {
                expected: TypeDraft::I64,
            },
            OperationDraft::If {
                condition: value,
                result: TypeDraft::I64,
                then_region: target,
                else_region: target,
            },
            OperationDraft::ForI64 {
                start: value,
                end_exclusive: value,
                step: 1,
                initial: value,
                carried: TypeDraft::I64,
                body_region: target,
            },
            OperationDraft::Return { value },
            OperationDraft::Yield { value },
            OperationDraft::ConstructProduct {
                product: target,
                fields: vec![ProductFieldValueDraft {
                    field: target,
                    value,
                }],
            },
            OperationDraft::ProjectField {
                value,
                field: target,
            },
            OperationDraft::ConstructVariant {
                variant: target,
                payload: Some(value),
            },
            OperationDraft::MatchSum {
                scrutinee: value,
                result: TypeDraft::I64,
                arms: vec![MatchArmOperationDraft {
                    variant: target,
                    region: target,
                }],
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
                code.stable_tag(),
            );
        }

        let value_samples = [
            ValueDraft::FunctionParameter(target),
            ValueDraft::OperationResult {
                operation: target,
                output: 0,
            },
            ValueDraft::BlockArgument(target),
        ];
        assert_eq!(value_samples.len(), ValueDraftCode::ALL.len());
        for (sample, code) in value_samples.iter().zip(ValueDraftCode::ALL) {
            assert_eq!(sample.code(), code);
            assert_draft_variant_serde_contract(
                sample,
                &schema,
                &schema.structured_authoring.value_variants,
                code.machine_name(),
                code.stable_tag(),
            );
        }

        let runtime_samples = vec![
            RuntimeValue::Unit,
            RuntimeValue::Bool(true),
            RuntimeValue::I64(1),
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
            assert_eq!(advertised.tag, code.stable_tag());
            assert_machine_variant_serde_contract(
                sample,
                runtime_variants,
                code.machine_name(),
                code.stable_tag(),
            );
        }

        let type_samples = [
            TypeDraft::Unit,
            TypeDraft::Bool,
            TypeDraft::I64,
            TypeDraft::Nominal(target),
        ];
        assert_eq!(
            type_samples.len(),
            schema.structured_authoring.type_variants.len()
        );
        for (index, (sample, description)) in type_samples
            .iter()
            .zip(&schema.structured_authoring.type_variants)
            .enumerate()
        {
            assert_eq!(description.tag, u8::try_from(index + 1).expect("type tag"));
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
                handle: LocalHandle::new(5),
                module: target,
                name: "pair".into(),
                fields: vec![ProductFieldDraft {
                    handle: LocalHandle::new(6),
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
                handle: LocalHandle::new(6),
                name: "value".into(),
                ty: TypeDraft::I64
            },
            ProductFieldDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "create_sum_type",
            TransactionOp::CreateSumType {
                handle: LocalHandle::new(7),
                module: target,
                name: "maybe".into(),
                variants: vec![SumVariantDraft {
                    handle: LocalHandle::new(8),
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
                handle: LocalHandle::new(8),
                name: "some".into(),
                payload: Some(TypeDraft::I64)
            },
            SumVariantDraft,
            false
        );
        structured_record_count += check_draft_record!(
            "create_function",
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: target,
                name: "f".into(),
                parameters: vec![FunctionParameterDraft {
                    handle: LocalHandle::new(4),
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
                handle: LocalHandle::new(4),
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
                value
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
                payload_handle: Some(LocalHandle::new(32)),
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
                function: target,
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
            return_handles: vec![LocalHandle::new(1)],
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
            returned_bindings: vec![(LocalHandle::new(1), node)],
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
                code.stable_tag(),
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
                code.stable_tag(),
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
        let sections = SchemaSections {
            digest,
            sections: all_schema_sections(&schema),
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
        count += check!("id_formats", schema.id_formats.clone());
        count += check!(
            "nominal_declarations_description",
            schema.nominal_declarations.clone()
        );
        count += check!("schema_manifest", manifest);
        count += check!("schema_sections", sections.clone());
        for section in &sections.sections {
            match section {
                SchemaSectionPayload::IdentityAndEnvelopes(value) => {
                    count += check!("identity_and_envelopes_section", value.clone())
                }
                SchemaSectionPayload::SemanticTypesAndNodes(value) => {
                    count += check!("semantic_types_and_nodes_section", value.clone())
                }
                SchemaSectionPayload::NominalDeclarations(value) => {
                    count += check!("nominal_declarations_section", value.clone())
                }
                SchemaSectionPayload::TransactionsAndExpressions(value) => {
                    count += check!("transactions_and_expressions_section", value.clone())
                }
                SchemaSectionPayload::QueriesAndRepair(value) => {
                    count += check!("queries_and_repair_section", value.clone())
                }
                SchemaSectionPayload::RuntimeAndRun(value) => {
                    count += check!("runtime_and_run_section", value.clone())
                }
                SchemaSectionPayload::ErrorsAndLimits(value) => {
                    count += check!("errors_and_limits_section", value.clone())
                }
            }
        }
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
        tag: u8,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let schema = schema_description();
        let description = variants
            .iter()
            .find(|variant| variant.name == name)
            .unwrap_or_else(|| panic!("missing variant {name}"));
        assert_eq!(description.tag, tag, "{name} stable tag");
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
        tag: u8,
    ) where
        T: serde::de::DeserializeOwned + Serialize,
    {
        let description = variants
            .iter()
            .find(|variant| variant.name == name)
            .unwrap_or_else(|| panic!("missing draft variant {name}"));
        assert_eq!(description.tag, tag, "{name} stable tag");
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
            let description = family
                .variants
                .iter()
                .find(|variant| variant.name == name)
                .unwrap_or_else(|| panic!("missing {family_name} descriptor {name}"));
            if family.tagging == "adjacently_tagged" {
                assert_machine_variant_serde_contract(
                    sample,
                    &family.variants,
                    name,
                    description.tag,
                );
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
            "schema_section" => schema
                .schema_discovery
                .sections
                .iter()
                .map(|code| code.name.as_str())
                .collect(),
            _ => return None,
        };
        Some(codes)
    }

    #[test]
    fn every_schema_type_expression_resolves_exactly_once() {
        use std::collections::BTreeMap;

        let schema = schema_description();
        let mut definitions = BTreeMap::<String, usize>::new();
        let mut expressions = Vec::<String>::new();
        for description in &schema.schema_discovery.records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.semantic_records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.transaction_records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.structured_authoring.records {
            register_name(&mut definitions, &description.name);
        }
        for description in &schema.run.records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.query_records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.error_records {
            register_record(&mut definitions, &mut expressions, description);
        }
        for description in &schema.envelopes {
            register_record(&mut definitions, &mut expressions, description);
        }
        register_name(&mut definitions, "describe_schema_request");
        collect_shape_expressions(&schema.schema_discovery.request, &mut expressions);
        register_name(&mut definitions, "error");
        collect_shape_expressions(&schema.error_payload, &mut expressions);

        for description in schema
            .schema_discovery
            .variants
            .iter()
            .chain(&schema.identity_variants)
            .chain(&schema.semantic_variants)
            .chain(&schema.transaction_variants)
            .chain(&schema.run.variants)
            .chain(&schema.query_variants)
            .chain(&schema.error_variants)
        {
            register_name(&mut definitions, &description.name);
            for variant in &description.variants {
                collect_shape_expressions(&variant.payload, &mut expressions);
            }
        }
        for (name, variants) in [
            (
                "expression_kind_draft",
                &schema.structured_authoring.expression_variants,
            ),
            (
                "operation_draft",
                &schema.structured_authoring.operation_variants,
            ),
            ("value_draft", &schema.structured_authoring.value_variants),
            ("type_draft", &schema.structured_authoring.type_variants),
        ] {
            register_name(&mut definitions, name);
            for variant in variants {
                if let Some(newtype) = variant.newtype {
                    expressions.push(
                        draft_field_type_description(&schema, newtype)
                            .type_expression
                            .clone(),
                    );
                }
                expressions.extend(variant.fields.iter().map(|field| {
                    draft_field_type_description(&schema, field.field_type)
                        .type_expression
                        .clone()
                }));
            }
        }
        for description in &schema.structured_authoring.records {
            expressions.extend(description.fields.iter().map(|field| {
                draft_field_type_description(&schema, field.field_type)
                    .type_expression
                    .clone()
            }));
        }
        for value in &schema.run.runtime_values {
            expressions.extend(
                value
                    .fields
                    .iter()
                    .map(|field| field.type_expression.clone()),
            );
        }

        for scalar in &schema.scalar_types {
            register_name(&mut definitions, &scalar.name);
        }
        register_name(&mut definitions, "type_parameter");
        for code in [
            "node_kind",
            "operation_code",
            "transaction_operation_code",
            "error_code",
            "schema_section",
        ] {
            register_name(&mut definitions, code);
        }

        for expression in expressions {
            for name in type_expression_names(&expression) {
                let count = definitions.get(name).copied().unwrap_or_default();
                assert_eq!(
                    count, 1,
                    "type expression `{expression}` references `{name}` {count} times"
                );
            }
        }
        assert!(definitions.values().all(|count| *count == 1));
    }

    fn register_name(definitions: &mut std::collections::BTreeMap<String, usize>, name: &str) {
        *definitions.entry(name.to_owned()).or_default() += 1;
    }

    fn register_record(
        definitions: &mut std::collections::BTreeMap<String, usize>,
        expressions: &mut Vec<String>,
        description: &NamedPayloadDescription,
    ) {
        *definitions.entry(description.name.clone()).or_default() += 1;
        collect_shape_expressions(&description.payload, expressions);
    }

    fn collect_shape_expressions(shape: &PayloadShapeDescription, output: &mut Vec<String>) {
        if let Some(newtype) = &shape.newtype {
            output.push(newtype.clone());
        }
        for field in &shape.fields {
            assert_eq!(
                field.type_expression.starts_with("optional<")
                    && field.type_expression.ends_with('>'),
                !field.required,
                "{}.{} must advertise requiredness and nullability exactly",
                field.name,
                field.type_expression
            );
            output.push(field.type_expression.clone());
        }
    }

    fn type_expression_names(expression: &str) -> Vec<&str> {
        expression
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .filter(|name| {
                !name.is_empty() && !matches!(*name, "list" | "tuple" | "page" | "optional")
            })
            .collect()
    }

    fn draft_field_type_description(
        schema: &SchemaDescription,
        field_type: DraftFieldType,
    ) -> &DraftFieldTypeDescription {
        let matches = schema
            .structured_authoring
            .draft_field_types
            .iter()
            .filter(|description| description.tag == field_type.stable_tag())
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
        let sections = all_schema_sections(&schema);
        let SchemaSectionPayload::SemanticTypesAndNodes(semantic) = &sections[1] else {
            panic!("semantic types and nodes section")
        };
        assert_eq!(semantic.name_contract, schema.name_contract);
        let SchemaSectionPayload::ErrorsAndLimits(errors) = &sections[6] else {
            panic!("errors and limits section")
        };
        assert_eq!(errors.limits, schema.limits);
    }

    #[test]
    fn schema_is_deterministic_complete_and_unique() {
        let first = schema_description();
        assert_eq!(first, schema_description());
        assert_eq!(first.operations.len(), OperationCode::ALL.len());
        assert_named_variant_counts(
            &first.semantic_variants,
            &[
                ("semantic_type", 4),
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
                ("literal_value", 3),
                ("dependency_fact", 2),
                ("scalar_value", 3),
                ("change_kind", 11),
            ],
        );
        assert_named_variant_counts(
            &first.schema_discovery.variants,
            &[
                ("schema_projection", 3),
                ("describe_schema_result", 4),
                ("schema_section_payload", SchemaSection::ALL.len()),
                ("payload_shape_kind", 3),
                ("json_scalar_kind", 3),
                ("machine_scalar_domain", 6),
                ("run_field_type", 7),
                ("runtime_value_payload", 5),
                ("draft_field_type", DraftFieldType::ALL.len()),
                ("operand_arity", 4),
                ("region_arity", 2),
                ("operand_use", 1),
                ("literal_field", 6),
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
            [("unit", 1), ("bool", 2), ("i64", 3), ("nominal", 4)],
        );
        assert_variants(
            &first.structured_authoring.type_variants,
            [("unit", 1), ("bool", 2), ("i64", 3), ("nominal", 4)],
        );
        assert_eq!(
            first
                .structured_authoring
                .draft_field_types
                .iter()
                .map(|description| (
                    description.name.as_str(),
                    description.tag,
                    description.type_expression.as_str(),
                ))
                .collect::<Vec<_>>(),
            DraftFieldType::ALL
                .into_iter()
                .map(|field_type| (
                    field_type.machine_name(),
                    field_type.stable_tag(),
                    field_type.type_expression(),
                ))
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
            first.structured_authoring.type_variants[..3]
                .iter()
                .all(|variant| variant.shape == PayloadShapeKind::Unit
                    && variant.newtype.is_none()
                    && variant.fields.is_empty())
        );
        let nominal = &first.structured_authoring.type_variants[3];
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
            TransactionOpCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variants(
            &first.structured_authoring.expression_variants,
            crate::transaction::ExpressionDraftCode::ALL
                .map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variants(
            &first.structured_authoring.operation_variants,
            OperationCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
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
            &first.query_result_payloads,
            QueryCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
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
            RequestCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_variant_payloads(
            &first.response_payloads,
            ResponseCode::ALL.map(|code| (code.machine_name(), code.stable_tag())),
        );
        assert_codes(
            &first.schema_discovery.sections,
            SchemaSection::ALL.map(|section| (section.machine_name(), section.stable_tag())),
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
        for section in SchemaSection::ALL {
            let record_name = format!("{}_section", section.machine_name());
            assert!(
                first
                    .schema_discovery
                    .records
                    .iter()
                    .any(|record| record.name == record_name),
                "missing {record_name}"
            );
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
                ("product", 4, RuntimeValuePayload::Product),
                ("sum", 5, RuntimeValuePayload::Sum),
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
                .any(|scope| scope.contains("peak frame arenas plus"))
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
            first.limits.maximum_run_live_cells,
            crate::interpret::MAX_RUN_LIVE_CELLS as u64
        );
        assert_eq!(
            first.limits.maximum_error_related_ids,
            crate::error::MAX_ERROR_RELATED_IDS as u32
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
        reordered.schema_discovery.sections.reverse();
        reordered.schema_discovery.section_payloads.reverse();
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
            .tag = 99;
        assert_ne!(
            machine_schema_digest(&changed_definition_slot).expect("enum family digest"),
            digest
        );
        let mut changed_tag = schema.clone();
        changed_tag.requests[0].tag = changed_tag.requests[0].tag.saturating_add(1);
        assert_ne!(
            machine_schema_digest(&changed_tag).expect("tag digest"),
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
            |limits: &mut BoundaryLimits| limits.maximum_frame_items += 1,
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
    fn sections_reconstruct_full_and_known_digest_short_circuits_every_projection() {
        let schema = schema_description();
        let sections = all_schema_sections(&schema);
        assert_eq!(
            reconstruct_schema_from_sections(&sections),
            Some(schema.clone())
        );
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
            SchemaProjection::Sections {
                sections: vec![SchemaSection::ErrorsAndLimits],
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
    fn section_requests_validate_and_project_in_canonical_order() {
        for sections in [
            vec![],
            vec![SchemaSection::RuntimeAndRun, SchemaSection::RuntimeAndRun],
        ] {
            assert!(
                DescribeSchemaRequest {
                    projection: SchemaProjection::Sections { sections },
                    known_digest: None,
                }
                .validate()
                .is_err()
            );
        }
        let result = describe_schema(&DescribeSchemaRequest {
            projection: SchemaProjection::Sections {
                sections: vec![
                    SchemaSection::ErrorsAndLimits,
                    SchemaSection::IdentityAndEnvelopes,
                ],
            },
            known_digest: None,
        })
        .expect("sections");
        let DescribeSchemaResult::Sections(result) = result else {
            panic!("sections projection")
        };
        assert!(matches!(
            result.sections.as_slice(),
            [
                SchemaSectionPayload::IdentityAndEnvelopes(_),
                SchemaSectionPayload::ErrorsAndLimits(_)
            ]
        ));
    }

    #[test]
    fn schema_projection_byte_measurements_are_retained() {
        let digest = active_machine_schema_digest().expect("digest");
        let cases = [
            ("manifest", DescribeSchemaRequest::manifest()),
            (
                "selected_nominal_construction_sections",
                DescribeSchemaRequest {
                    projection: SchemaProjection::Sections {
                        sections: vec![
                            SchemaSection::SemanticTypesAndNodes,
                            SchemaSection::NominalDeclarations,
                            SchemaSection::TransactionsAndExpressions,
                            SchemaSection::QueriesAndRepair,
                            SchemaSection::RuntimeAndRun,
                            SchemaSection::ErrorsAndLimits,
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
            let json = serde_json::to_vec(&result).expect("projection JSON");
            let binary = crate::protocol::encoded_response_size(
                RequestId::new(1),
                &Response::DescribeSchema(Box::new(result)),
            )
            .expect("projection binary");
            eprintln!(
                "schema_projection_bytes name={name} json={} binary={binary}",
                json.len()
            );
            sizes.push((name, json.len(), binary));
        }
        assert!(
            sizes
                .iter()
                .all(|(_, json, binary)| *json > 0 && *binary > 0)
        );
        assert!(sizes[0].1 < sizes[2].1);
        assert!(sizes[3].1 < sizes[0].1);
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
