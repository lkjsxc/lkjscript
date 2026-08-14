//! Strict, bounded JSON transport projection and runtime machine contract description.

use crate::ids::RequestId;
use crate::protocol::{PROTOCOL_VERSION, Request, RequestCode, Response, ResponseCode};
use crate::query::{
    MAX_BATCH_ITEMS, MAX_BATCH_QUERIES, MAX_CONTEXT_ITEMS, MAX_PAGE_ITEMS, QueryCode,
};
use crate::schema::{LiteralField, NodeKind, OperandUse, OperationCode, SemanticType, TypeRule};
use crate::transaction::{MAX_RETURNED_BINDINGS, TransactionOpCode};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{self, Write};

pub const JSON_ENVELOPE_VERSION: u16 = 2;
pub const MAX_JSON_INPUT_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_JSON_OUTPUT_BYTES: usize = 32 * 1024 * 1024;
const MAX_BOUNDARY_ERROR_MESSAGE_BYTES: usize = 1024;
const BOUNDARY_ERROR_FALLBACK: &[u8] =
    b"{\"version\":2,\"error\":{\"kind\":\"output\",\"message\":\"cannot encode boundary error\"}}";

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
    pub queries: Vec<CodeDescription>,
    pub errors: Vec<CodeDescription>,
    pub requests: Vec<CodeDescription>,
    pub responses: Vec<CodeDescription>,
    pub limits: BoundaryLimits,
    pub id_formats: IdFormats,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CodeDescription {
    pub name: String,
    pub tag: u8,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDescription {
    pub name: String,
    pub tag: u8,
    pub operands: Vec<OperandDescription>,
    pub results: Vec<TypeRule>,
    pub literal_fields: Vec<LiteralField>,
    pub complete: bool,
    pub terminator: bool,
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
                    complete: descriptor.complete,
                    terminator: descriptor.terminator,
                }
            })
            .collect(),
        transaction_operations: TransactionOpCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        queries: QueryCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        errors: crate::ErrorCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        requests: RequestCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
            .collect(),
        responses: ResponseCode::ALL
            .into_iter()
            .map(|code| described(code.machine_name(), code.stable_tag()))
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
    use crate::{ErrorCode, QueryId, Revision, WorkspaceId};

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
            text.replacen("\"version\":2", "\"version\":1", 1),
            text.replacen("\"request_id\":1", "\"request_id\":0", 1),
            text.replacen("{\"version\":2", "{\"unknown\":0,\"version\":2", 1),
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
        let compact = encode_schema(false).expect("compact");
        let pretty = encode_schema(true).expect("pretty");
        assert_eq!(
            serde_json::from_slice::<SchemaDescription>(&compact).expect("compact decode"),
            serde_json::from_slice::<SchemaDescription>(&pretty).expect("pretty decode")
        );
        assert!(!compact.contains(&b'\n'));
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
