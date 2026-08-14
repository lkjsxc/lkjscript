use crate::artifact::DecodePolicy;
use crate::codec::{CodecError, CodecErrorKind, Reader, TagDomain, Writer};
use crate::diff::{Change, ChangeKind, ScalarValue};
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{
    ChangeDigest, IdempotencyKey, LocalHandle, NodeId, RequestId, Revision, SnapshotHash,
    WorkspaceId,
};
use crate::interpret::{RunResult, RuntimeValue};
use crate::machine::{
    BoundaryLimits, CodeDescription, IdFormats, OperandDescription, OperationDescription,
    SchemaDescription,
};
use crate::query::*;
use crate::schema::{
    LiteralField, NodeKind, OperandUse, OperationCode, OperationDraft, OperationKind, SemanticType,
    TypeRule, ValueDraft, ValueRef,
};
use crate::transaction::{
    ApplyTransactionRequest, MAX_RETURNED_BINDINGS, NodeTarget, Transaction, TransactionMode,
    TransactionOp, TransactionOpCode, TransactionReceipt, TransactionResponseSpec,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAXIMUM_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_FRAME_ITEMS: usize = 100_000;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RequestCode {
    CreateWorkspace,
    ApplyTransaction,
    QueryBatch,
    Run,
    Shutdown,
    DescribeSchema,
}

impl RequestCode {
    pub const ALL: [Self; 6] = [
        Self::CreateWorkspace,
        Self::ApplyTransaction,
        Self::QueryBatch,
        Self::Run,
        Self::Shutdown,
        Self::DescribeSchema,
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::CreateWorkspace => 1,
            Self::ApplyTransaction => 2,
            Self::QueryBatch => 3,
            Self::Run => 4,
            Self::Shutdown => 5,
            Self::DescribeSchema => 6,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::CreateWorkspace),
            2 => Some(Self::ApplyTransaction),
            3 => Some(Self::QueryBatch),
            4 => Some(Self::Run),
            5 => Some(Self::Shutdown),
            6 => Some(Self::DescribeSchema),
            _ => None,
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::CreateWorkspace => "create_workspace",
            Self::ApplyTransaction => "apply_transaction",
            Self::QueryBatch => "query_batch",
            Self::Run => "run",
            Self::Shutdown => "shutdown",
            Self::DescribeSchema => "describe_schema",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ResponseCode {
    WorkspaceCreated,
    TransactionReceipt,
    QueryBatchResult,
    Run,
    Acknowledged,
    Error,
    SchemaDescription,
}

impl ResponseCode {
    pub const ALL: [Self; 7] = [
        Self::WorkspaceCreated,
        Self::TransactionReceipt,
        Self::QueryBatchResult,
        Self::Run,
        Self::Acknowledged,
        Self::Error,
        Self::SchemaDescription,
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::WorkspaceCreated => 101,
            Self::TransactionReceipt => 102,
            Self::QueryBatchResult => 103,
            Self::Run => 104,
            Self::Acknowledged => 105,
            Self::SchemaDescription => 106,
            Self::Error => 255,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            101 => Some(Self::WorkspaceCreated),
            102 => Some(Self::TransactionReceipt),
            103 => Some(Self::QueryBatchResult),
            104 => Some(Self::Run),
            105 => Some(Self::Acknowledged),
            106 => Some(Self::SchemaDescription),
            255 => Some(Self::Error),
            _ => None,
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspaceCreated => "workspace_created",
            Self::TransactionReceipt => "transaction_receipt",
            Self::QueryBatchResult => "query_batch_result",
            Self::Run => "run",
            Self::Acknowledged => "acknowledged",
            Self::Error => "error",
            Self::SchemaDescription => "schema_description",
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
pub enum Request {
    CreateWorkspace,
    ApplyTransaction(ApplyTransactionRequest),
    QueryBatch(QueryBatchRequest),
    Run {
        workspace: WorkspaceId,
        revision: Revision,
        entry: NodeId,
    },
    Shutdown,
    DescribeSchema,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Response {
    WorkspaceCreated(WorkspaceSummary),
    TransactionReceipt(TransactionReceipt),
    QueryBatchResult(QueryBatchResult),
    Run(RunResult),
    Acknowledged,
    Error(LkError),
    SchemaDescription(Box<SchemaDescription>),
}

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
        let mut stream = UnixStream::connect(&self.endpoint)?;
        stream.set_read_timeout(Some(CLIENT_IO_TIMEOUT))?;
        stream.set_write_timeout(Some(CLIENT_IO_TIMEOUT))?;
        write_request(&mut stream, request_id, request)?;
        stream.shutdown(Shutdown::Write)?;
        let (response_id, response) = read_response(&mut stream)?.ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "daemon closed the connection before a response frame",
            )
        })?;
        if response_id != request_id {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "response request identity does not match request",
            ));
        }
        Ok(response)
    }

    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }
}

pub fn encoded_request_size(request_id: RequestId, request: &Request) -> Result<usize> {
    let mut bytes = Vec::new();
    write_request(&mut bytes, request_id, request)?;
    Ok(bytes.len())
}

pub fn encoded_response_size(request_id: RequestId, response: &Response) -> Result<usize> {
    let mut bytes = Vec::new();
    write_response(&mut bytes, request_id, response)?;
    Ok(bytes.len())
}

pub(crate) fn write_request(
    writer: &mut impl Write,
    request_id: RequestId,
    request: &Request,
) -> Result<()> {
    if request_id.get() == 0 {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "request ID zero is reserved",
        ));
    }
    let mut body = Writer::new();
    body.u16(PROTOCOL_VERSION);
    body.u64(request_id.get());
    put_request(&mut body, request)?;
    write_frame(writer, &body.finish())
}

pub(crate) fn read_request(reader: &mut impl Read) -> Result<Option<(RequestId, Request)>> {
    let Some(frame) = read_frame(reader)? else {
        return Ok(None);
    };
    let mut body = Reader::new(&frame);
    let version = body.u16().map_err(protocol_codec)?;
    if version != PROTOCOL_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "protocol version is unsupported",
        ));
    }
    let request_id = RequestId::new(body.u64().map_err(protocol_codec)?);
    if request_id.get() == 0 {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "request ID zero is reserved",
        ));
    }
    let request = read_request_body(&mut body)?;
    body.finish().map_err(protocol_codec)?;
    require_connection_eof(reader, "request")?;
    Ok(Some((request_id, request)))
}

pub(crate) fn write_response(
    writer: &mut impl Write,
    request_id: RequestId,
    response: &Response,
) -> Result<()> {
    let mut body = Writer::new();
    body.u16(PROTOCOL_VERSION);
    body.u64(request_id.get());
    put_response(&mut body, response)?;
    write_frame(writer, &body.finish())
}

fn read_response(reader: &mut impl Read) -> Result<Option<(RequestId, Response)>> {
    let Some(frame) = read_frame(reader)? else {
        return Ok(None);
    };
    let mut body = Reader::new(&frame);
    let version = body.u16().map_err(protocol_codec)?;
    if version != PROTOCOL_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "protocol version is unsupported",
        ));
    }
    let request_id = RequestId::new(body.u64().map_err(protocol_codec)?);
    let response = read_response_body(&mut body)?;
    body.finish().map_err(protocol_codec)?;
    require_connection_eof(reader, "response")?;
    Ok(Some((request_id, response)))
}

fn require_connection_eof(reader: &mut impl Read, frame_kind: &str) -> Result<()> {
    let mut trailing = [0_u8; 1];
    match reader.read(&mut trailing) {
        Ok(0) => Ok(()),
        Ok(_) => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("connection contains bytes after its single {frame_kind} frame"),
        )),
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn transaction_fingerprint(request: &ApplyTransactionRequest) -> Result<[u8; 32]> {
    let mut writer = Writer::new();
    writer.fixed(b"lkjscript.apply-transaction.v2\0");
    put_apply_transaction_request(&mut writer, request)?;
    Ok(*blake3::hash(&writer.finish()).as_bytes())
}

pub(crate) fn put_transaction_receipt(
    writer: &mut Writer,
    receipt: &TransactionReceipt,
) -> Result<()> {
    if receipt.returned_bindings.len() > MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "transaction receipt bindings exceed response policy",
        ));
    }
    put_workspace(writer, receipt.workspace);
    writer.u64(receipt.base_revision.get());
    writer.u64(receipt.revision.get());
    writer.fixed(&receipt.hash.as_bytes());
    writer.bool(receipt.published);
    writer.u64(receipt.created_count);
    put_count(writer, receipt.returned_bindings.len())?;
    for (handle, node) in &receipt.returned_bindings {
        writer.u32(handle.get());
        put_node_id(writer, *node);
    }
    writer.u64(receipt.change_count);
    writer.fixed(&receipt.change_digest.as_bytes());
    writer.bool(receipt.complete_before);
    writer.bool(receipt.complete_after);
    writer.u64(receipt.blocker_count_before);
    writer.u64(receipt.blocker_count_after);
    Ok(())
}

pub(crate) fn read_transaction_receipt(reader: &mut Reader<'_>) -> Result<TransactionReceipt> {
    let workspace = read_workspace(reader)?;
    let base_revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let hash = read_hash(reader)?;
    let published = reader.bool().map_err(protocol_codec)?;
    let created_count = reader.u64().map_err(protocol_codec)?;
    let count = reader
        .count(MAX_RETURNED_BINDINGS)
        .map_err(protocol_codec)?;
    let mut returned_bindings = Vec::with_capacity(count);
    for _ in 0..count {
        returned_bindings.push((read_handle(reader)?, read_node_id(reader)?));
    }
    let change_count = reader.u64().map_err(protocol_codec)?;
    let change_digest = read_change_digest(reader)?;
    let complete_before = reader.bool().map_err(protocol_codec)?;
    let complete_after = reader.bool().map_err(protocol_codec)?;
    let blocker_count_before = reader.u64().map_err(protocol_codec)?;
    let blocker_count_after = reader.u64().map_err(protocol_codec)?;
    Ok(TransactionReceipt {
        workspace,
        base_revision,
        revision,
        hash,
        published,
        created_count,
        returned_bindings,
        change_count,
        change_digest,
        complete_before,
        complete_after,
        blocker_count_before,
        blocker_count_after,
    })
}

fn put_request(writer: &mut Writer, request: &Request) -> Result<()> {
    match request {
        Request::CreateWorkspace => writer.u8(RequestCode::CreateWorkspace.stable_tag()),
        Request::ApplyTransaction(r) => {
            writer.u8(RequestCode::ApplyTransaction.stable_tag());
            put_apply_transaction_request(writer, r)?
        }
        Request::QueryBatch(r) => {
            writer.u8(RequestCode::QueryBatch.stable_tag());
            put_query_batch_request(writer, r)?
        }
        Request::Run {
            workspace,
            revision,
            entry,
        } => {
            writer.u8(RequestCode::Run.stable_tag());
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
            put_node_id(writer, *entry)
        }
        Request::Shutdown => writer.u8(RequestCode::Shutdown.stable_tag()),
        Request::DescribeSchema => writer.u8(RequestCode::DescribeSchema.stable_tag()),
    }
    Ok(())
}
fn read_request_body(reader: &mut Reader<'_>) -> Result<Request> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match RequestCode::from_stable_tag(tag) {
        Some(RequestCode::CreateWorkspace) => Ok(Request::CreateWorkspace),
        Some(RequestCode::ApplyTransaction) => Ok(Request::ApplyTransaction(
            read_apply_transaction_request(reader)?,
        )),
        Some(RequestCode::QueryBatch) => Ok(Request::QueryBatch(read_query_batch_request(reader)?)),
        Some(RequestCode::Run) => Ok(Request::Run {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
            entry: read_node_id(reader)?,
        }),
        Some(RequestCode::Shutdown) => Ok(Request::Shutdown),
        Some(RequestCode::DescribeSchema) => Ok(Request::DescribeSchema),
        None => Err(protocol_codec(
            reader.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}
fn put_response(writer: &mut Writer, response: &Response) -> Result<()> {
    match response {
        Response::WorkspaceCreated(v) => {
            writer.u8(ResponseCode::WorkspaceCreated.stable_tag());
            put_workspace_summary(writer, v)?
        }
        Response::TransactionReceipt(v) => {
            writer.u8(ResponseCode::TransactionReceipt.stable_tag());
            put_transaction_receipt(writer, v)?
        }
        Response::QueryBatchResult(v) => {
            writer.u8(ResponseCode::QueryBatchResult.stable_tag());
            put_query_batch_result(writer, v)?
        }
        Response::Run(v) => {
            writer.u8(ResponseCode::Run.stable_tag());
            put_run_result(writer, v)
        }
        Response::Acknowledged => writer.u8(ResponseCode::Acknowledged.stable_tag()),
        Response::Error(v) => {
            writer.u8(ResponseCode::Error.stable_tag());
            put_error(writer, v)?
        }
        Response::SchemaDescription(v) => {
            writer.u8(ResponseCode::SchemaDescription.stable_tag());
            put_schema_description(writer, v)?
        }
    }
    Ok(())
}
fn read_response_body(reader: &mut Reader<'_>) -> Result<Response> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match ResponseCode::from_stable_tag(tag) {
        Some(ResponseCode::WorkspaceCreated) => {
            Ok(Response::WorkspaceCreated(read_workspace_summary(reader)?))
        }
        Some(ResponseCode::TransactionReceipt) => Ok(Response::TransactionReceipt(
            read_transaction_receipt(reader)?,
        )),
        Some(ResponseCode::QueryBatchResult) => {
            Ok(Response::QueryBatchResult(read_query_batch_result(reader)?))
        }
        Some(ResponseCode::Run) => Ok(Response::Run(read_run_result(reader)?)),
        Some(ResponseCode::Acknowledged) => Ok(Response::Acknowledged),
        Some(ResponseCode::Error) => Ok(Response::Error(read_error(reader)?)),
        Some(ResponseCode::SchemaDescription) => {
            let schema = read_schema_description(reader)?;
            if schema != crate::machine::schema_description() {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "schema description does not match the current build contract",
                ));
            }
            Ok(Response::SchemaDescription(Box::new(schema)))
        }
        None => Err(protocol_codec(
            reader.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}

fn put_schema_description(writer: &mut Writer, schema: &SchemaDescription) -> Result<()> {
    writer.u16(schema.binary_protocol_version);
    writer.u16(schema.json_envelope_version);
    put_code_descriptions(writer, &schema.semantic_types)?;
    put_code_descriptions(writer, &schema.node_kinds)?;
    put_count(writer, schema.operations.len())?;
    for operation in &schema.operations {
        writer.string(&operation.name).map_err(protocol_codec)?;
        writer.u8(operation.tag);
        put_count(writer, operation.operands.len())?;
        for operand in &operation.operands {
            put_type_rule(writer, operand.ty);
            writer.u8(match operand.use_mode {
                OperandUse::Copy => 1,
            });
        }
        put_count(writer, operation.results.len())?;
        for result in &operation.results {
            put_type_rule(writer, *result);
        }
        put_count(writer, operation.literal_fields.len())?;
        for literal in &operation.literal_fields {
            writer.u8(match literal {
                LiteralField::I64Value => 1,
                LiteralField::BoolValue => 2,
                LiteralField::ExpectedType => 3,
            });
        }
        writer.bool(operation.complete);
        writer.bool(operation.terminator);
    }
    put_code_descriptions(writer, &schema.transaction_operations)?;
    put_code_descriptions(writer, &schema.queries)?;
    put_code_descriptions(writer, &schema.errors)?;
    put_code_descriptions(writer, &schema.requests)?;
    put_code_descriptions(writer, &schema.responses)?;
    let limits = &schema.limits;
    writer.u64(limits.maximum_frame_bytes);
    writer.u64(limits.maximum_frame_items);
    writer.u64(limits.maximum_json_input_bytes);
    writer.u64(limits.maximum_json_output_bytes);
    writer.u32(limits.maximum_page_items);
    writer.u32(limits.maximum_batch_queries);
    writer.u32(limits.maximum_batch_items);
    writer.u32(limits.maximum_context_items_per_category);
    writer.u32(limits.maximum_returned_bindings);
    writer.u64(limits.maximum_persistence_head_bytes);
    let ids = &schema.id_formats;
    for value in [
        &ids.workspace,
        &ids.idempotency_key,
        &ids.node,
        &ids.snapshot_hash,
        &ids.change_digest,
        &ids.revision,
        &ids.request_id,
        &ids.query_id,
        &ids.local_handle,
    ] {
        writer.string(value).map_err(protocol_codec)?;
    }
    Ok(())
}

fn read_schema_description(reader: &mut Reader<'_>) -> Result<SchemaDescription> {
    let binary_protocol_version = reader.u16().map_err(protocol_codec)?;
    let json_envelope_version = reader.u16().map_err(protocol_codec)?;
    let semantic_types = read_code_descriptions(reader)?;
    let node_kinds = read_code_descriptions(reader)?;
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut operations = Vec::with_capacity(count);
    for _ in 0..count {
        let name = read_protocol_string(reader)?;
        let tag = reader.u8().map_err(protocol_codec)?;
        let operand_count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
        let mut operands = Vec::with_capacity(operand_count);
        for _ in 0..operand_count {
            let ty = read_type_rule(reader)?;
            let use_mode_tag = reader.u8().map_err(protocol_codec)?;
            let use_mode = match use_mode_tag {
                1 => OperandUse::Copy,
                _ => {
                    return Err(protocol_codec(
                        reader.unknown_tag(TagDomain::ProtocolMessage, use_mode_tag),
                    ));
                }
            };
            operands.push(OperandDescription { ty, use_mode });
        }
        let result_count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
        let mut results = Vec::with_capacity(result_count);
        for _ in 0..result_count {
            results.push(read_type_rule(reader)?);
        }
        let literal_count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
        let mut literal_fields = Vec::with_capacity(literal_count);
        for _ in 0..literal_count {
            let literal_tag = reader.u8().map_err(protocol_codec)?;
            literal_fields.push(match literal_tag {
                1 => LiteralField::I64Value,
                2 => LiteralField::BoolValue,
                3 => LiteralField::ExpectedType,
                _ => {
                    return Err(protocol_codec(
                        reader.unknown_tag(TagDomain::ProtocolMessage, literal_tag),
                    ));
                }
            });
        }
        operations.push(OperationDescription {
            name,
            tag,
            operands,
            results,
            literal_fields,
            complete: reader.bool().map_err(protocol_codec)?,
            terminator: reader.bool().map_err(protocol_codec)?,
        });
    }
    let transaction_operations = read_code_descriptions(reader)?;
    let queries = read_code_descriptions(reader)?;
    let errors = read_code_descriptions(reader)?;
    let requests = read_code_descriptions(reader)?;
    let responses = read_code_descriptions(reader)?;
    let limits = BoundaryLimits {
        maximum_frame_bytes: reader.u64().map_err(protocol_codec)?,
        maximum_frame_items: reader.u64().map_err(protocol_codec)?,
        maximum_json_input_bytes: reader.u64().map_err(protocol_codec)?,
        maximum_json_output_bytes: reader.u64().map_err(protocol_codec)?,
        maximum_page_items: reader.u32().map_err(protocol_codec)?,
        maximum_batch_queries: reader.u32().map_err(protocol_codec)?,
        maximum_batch_items: reader.u32().map_err(protocol_codec)?,
        maximum_context_items_per_category: reader.u32().map_err(protocol_codec)?,
        maximum_returned_bindings: reader.u32().map_err(protocol_codec)?,
        maximum_persistence_head_bytes: reader.u64().map_err(protocol_codec)?,
    };
    let id_formats = IdFormats {
        workspace: read_protocol_string(reader)?,
        idempotency_key: read_protocol_string(reader)?,
        node: read_protocol_string(reader)?,
        snapshot_hash: read_protocol_string(reader)?,
        change_digest: read_protocol_string(reader)?,
        revision: read_protocol_string(reader)?,
        request_id: read_protocol_string(reader)?,
        query_id: read_protocol_string(reader)?,
        local_handle: read_protocol_string(reader)?,
    };
    Ok(SchemaDescription {
        binary_protocol_version,
        json_envelope_version,
        semantic_types,
        node_kinds,
        operations,
        transaction_operations,
        queries,
        errors,
        requests,
        responses,
        limits,
        id_formats,
    })
}

fn put_code_descriptions(writer: &mut Writer, codes: &[CodeDescription]) -> Result<()> {
    put_count(writer, codes.len())?;
    for code in codes {
        writer.string(&code.name).map_err(protocol_codec)?;
        writer.u8(code.tag);
    }
    Ok(())
}

fn read_code_descriptions(reader: &mut Reader<'_>) -> Result<Vec<CodeDescription>> {
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut codes = Vec::with_capacity(count);
    for _ in 0..count {
        codes.push(CodeDescription {
            name: read_protocol_string(reader)?,
            tag: reader.u8().map_err(protocol_codec)?,
        });
    }
    Ok(codes)
}

fn put_type_rule(writer: &mut Writer, rule: TypeRule) {
    match rule {
        TypeRule::Fixed(ty) => {
            writer.u8(1);
            put_type(writer, ty);
        }
        TypeRule::PayloadExpected => writer.u8(2),
        TypeRule::OwnerFunctionResult => writer.u8(3),
    }
}

fn read_type_rule(reader: &mut Reader<'_>) -> Result<TypeRule> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(TypeRule::Fixed(read_type(reader)?)),
        2 => Ok(TypeRule::PayloadExpected),
        3 => Ok(TypeRule::OwnerFunctionResult),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}

fn put_apply_transaction_request(
    writer: &mut Writer,
    request: &ApplyTransactionRequest,
) -> Result<()> {
    if request.response.return_handles.len() > MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "selected return handles exceed transaction response policy",
        ));
    }
    put_transaction(writer, &request.transaction)?;
    put_count(writer, request.response.return_handles.len())?;
    for handle in &request.response.return_handles {
        writer.u32(handle.get());
    }
    Ok(())
}

fn read_apply_transaction_request(reader: &mut Reader<'_>) -> Result<ApplyTransactionRequest> {
    let transaction = read_transaction(reader)?;
    let count = reader
        .count(MAX_RETURNED_BINDINGS)
        .map_err(protocol_codec)?;
    let mut return_handles = Vec::with_capacity(count);
    for _ in 0..count {
        return_handles.push(read_handle(reader)?);
    }
    Ok(ApplyTransactionRequest {
        transaction,
        response: TransactionResponseSpec { return_handles },
    })
}

fn put_transaction(writer: &mut Writer, transaction: &Transaction) -> Result<()> {
    put_workspace(writer, transaction.workspace);
    writer.u64(transaction.base_revision.get());
    put_optional_idempotency(writer, transaction.idempotency_key);
    writer.u8(match transaction.mode {
        TransactionMode::Commit => 1,
        TransactionMode::ValidateOnly => 2,
    });
    put_count(writer, transaction.operations.len())?;
    for operation in &transaction.operations {
        put_transaction_operation(writer, operation)?;
    }
    Ok(())
}

fn read_transaction(reader: &mut Reader<'_>) -> Result<Transaction> {
    let workspace = read_workspace(reader)?;
    let base_revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let idempotency_key = read_optional_idempotency(reader)?;
    let mode_tag = reader.u8().map_err(protocol_codec)?;
    let mode = match mode_tag {
        1 => TransactionMode::Commit,
        2 => TransactionMode::ValidateOnly,
        _ => {
            return Err(protocol_codec(
                reader.unknown_tag(TagDomain::ProtocolMessage, mode_tag),
            ));
        }
    };
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut operations = Vec::with_capacity(count);
    for _ in 0..count {
        operations.push(read_transaction_operation(reader)?);
    }
    Ok(Transaction {
        workspace,
        base_revision,
        idempotency_key,
        mode,
        operations,
    })
}

fn put_transaction_operation(writer: &mut Writer, operation: &TransactionOp) -> Result<()> {
    writer.u8(operation.code().stable_tag());
    match operation {
        TransactionOp::CreatePackage { handle, name } => {
            writer.u32(handle.get());
            writer.string(name).map_err(protocol_codec)?;
        }
        TransactionOp::CreateModule {
            handle,
            package,
            name,
        } => {
            writer.u32(handle.get());
            put_node_target(writer, *package);
            writer.string(name).map_err(protocol_codec)?;
        }
        TransactionOp::CreateFunction {
            handle,
            module,
            name,
            result,
        } => {
            writer.u32(handle.get());
            put_node_target(writer, *module);
            writer.string(name).map_err(protocol_codec)?;
            put_type(writer, *result);
        }
        TransactionOp::CreateParameter {
            handle,
            function,
            name,
            ty,
        } => {
            writer.u32(handle.get());
            put_node_target(writer, *function);
            writer.string(name).map_err(protocol_codec)?;
            put_type(writer, *ty);
        }
        TransactionOp::CreateRegion { handle, function } => {
            writer.u32(handle.get());
            put_node_target(writer, *function);
        }
        TransactionOp::CreateBlock { handle, region } => {
            writer.u32(handle.get());
            put_node_target(writer, *region);
        }
        TransactionOp::CreateOperation {
            handle,
            block,
            before,
            operation,
        } => {
            writer.u32(handle.get());
            put_node_target(writer, *block);
            put_optional_node_target(writer, *before);
            put_operation_draft(writer, operation);
        }
        TransactionOp::SetFunctionBody { function, region } => {
            put_node_target(writer, *function);
            put_node_target(writer, *region);
        }
        TransactionOp::SetEntryFunction { package, function } => {
            put_node_target(writer, *package);
            put_node_target(writer, *function);
        }
        TransactionOp::RenameNode { node, name } => {
            put_node_target(writer, *node);
            writer.string(name).map_err(protocol_codec)?;
        }
        TransactionOp::ReplaceOperation {
            operation,
            replacement,
        } => {
            put_node_target(writer, *operation);
            put_operation_draft(writer, replacement);
        }
        TransactionOp::ReplaceOperand {
            operation,
            index,
            value,
        } => {
            put_node_target(writer, *operation);
            writer.u8(*index);
            put_value_draft(writer, *value);
        }
        TransactionOp::DeleteOwnedSubtree { root } => {
            put_node_target(writer, *root);
        }
        TransactionOp::RefineHole { hole, replacement } => {
            put_node_target(writer, *hole);
            put_operation_draft(writer, replacement);
        }
    }
    Ok(())
}

fn read_transaction_operation(reader: &mut Reader<'_>) -> Result<TransactionOp> {
    let tag = reader.u8().map_err(protocol_codec)?;
    let code = TransactionOpCode::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(reader.unknown_tag(TagDomain::TransactionOperation, tag)))?;
    match code {
        TransactionOpCode::CreatePackage => Ok(TransactionOp::CreatePackage {
            handle: read_handle(reader)?,
            name: read_protocol_string(reader)?,
        }),
        TransactionOpCode::CreateModule => Ok(TransactionOp::CreateModule {
            handle: read_handle(reader)?,
            package: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
        }),
        TransactionOpCode::CreateFunction => Ok(TransactionOp::CreateFunction {
            handle: read_handle(reader)?,
            module: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
            result: read_type(reader)?,
        }),
        TransactionOpCode::CreateParameter => Ok(TransactionOp::CreateParameter {
            handle: read_handle(reader)?,
            function: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
            ty: read_type(reader)?,
        }),
        TransactionOpCode::CreateRegion => Ok(TransactionOp::CreateRegion {
            handle: read_handle(reader)?,
            function: read_node_target(reader)?,
        }),
        TransactionOpCode::CreateBlock => Ok(TransactionOp::CreateBlock {
            handle: read_handle(reader)?,
            region: read_node_target(reader)?,
        }),
        TransactionOpCode::CreateOperation => Ok(TransactionOp::CreateOperation {
            handle: read_handle(reader)?,
            block: read_node_target(reader)?,
            before: read_optional_node_target(reader)?,
            operation: read_operation_draft(reader)?,
        }),
        TransactionOpCode::SetFunctionBody => Ok(TransactionOp::SetFunctionBody {
            function: read_node_target(reader)?,
            region: read_node_target(reader)?,
        }),
        TransactionOpCode::SetEntryFunction => Ok(TransactionOp::SetEntryFunction {
            package: read_node_target(reader)?,
            function: read_node_target(reader)?,
        }),
        TransactionOpCode::RenameNode => Ok(TransactionOp::RenameNode {
            node: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
        }),
        TransactionOpCode::ReplaceOperation => Ok(TransactionOp::ReplaceOperation {
            operation: read_node_target(reader)?,
            replacement: read_operation_draft(reader)?,
        }),
        TransactionOpCode::ReplaceOperand => Ok(TransactionOp::ReplaceOperand {
            operation: read_node_target(reader)?,
            index: reader.u8().map_err(protocol_codec)?,
            value: read_value_draft(reader)?,
        }),
        TransactionOpCode::DeleteOwnedSubtree => Ok(TransactionOp::DeleteOwnedSubtree {
            root: read_node_target(reader)?,
        }),
        TransactionOpCode::RefineHole => Ok(TransactionOp::RefineHole {
            hole: read_node_target(reader)?,
            replacement: read_operation_draft(reader)?,
        }),
    }
}

fn put_operation_draft(writer: &mut Writer, operation: &OperationDraft) {
    writer.u8(operation.code().stable_tag());
    match operation {
        OperationDraft::ConstI64(value) => writer.i64(*value),
        OperationDraft::ConstBool(value) => writer.bool(*value),
        OperationDraft::AddI64 { lhs, rhs } => {
            put_value_draft(writer, *lhs);
            put_value_draft(writer, *rhs);
        }
        OperationDraft::Hole { expected } => put_type(writer, *expected),
        OperationDraft::Return { value } => put_value_draft(writer, *value),
    }
}

fn read_operation_draft(reader: &mut Reader<'_>) -> Result<OperationDraft> {
    let tag = reader.u8().map_err(protocol_codec)?;
    let code = OperationCode::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(reader.unknown_tag(TagDomain::Operation, tag)))?;
    match code {
        OperationCode::ConstI64 => Ok(OperationDraft::ConstI64(
            reader.i64().map_err(protocol_codec)?,
        )),
        OperationCode::ConstBool => Ok(OperationDraft::ConstBool(
            reader.bool().map_err(protocol_codec)?,
        )),
        OperationCode::AddI64 => Ok(OperationDraft::AddI64 {
            lhs: read_value_draft(reader)?,
            rhs: read_value_draft(reader)?,
        }),
        OperationCode::Hole => Ok(OperationDraft::Hole {
            expected: read_type(reader)?,
        }),
        OperationCode::Return => Ok(OperationDraft::Return {
            value: read_value_draft(reader)?,
        }),
    }
}

fn put_value_draft(writer: &mut Writer, value: ValueDraft) {
    match value {
        ValueDraft::FunctionParameter(parameter) => {
            writer.u8(1);
            put_node_target(writer, parameter);
        }
        ValueDraft::OperationResult { operation, output } => {
            writer.u8(2);
            put_node_target(writer, operation);
            writer.u8(output);
        }
    }
}

fn read_value_draft(reader: &mut Reader<'_>) -> Result<ValueDraft> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(ValueDraft::FunctionParameter(read_node_target(reader)?)),
        2 => Ok(ValueDraft::OperationResult {
            operation: read_node_target(reader)?,
            output: reader.u8().map_err(protocol_codec)?,
        }),
        _ => Err(protocol_codec(reader.unknown_tag(TagDomain::Value, tag))),
    }
}

fn put_node_target(writer: &mut Writer, target: NodeTarget) {
    match target {
        NodeTarget::Existing(node) => {
            writer.u8(1);
            put_node_id(writer, node);
        }
        NodeTarget::Local(handle) => {
            writer.u8(2);
            writer.u32(handle.get());
        }
    }
}

fn read_node_target(reader: &mut Reader<'_>) -> Result<NodeTarget> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(NodeTarget::Existing(read_node_id(reader)?)),
        2 => Ok(NodeTarget::Local(read_handle(reader)?)),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::NodeTarget, tag),
        )),
    }
}

fn put_optional_node_target(writer: &mut Writer, target: Option<NodeTarget>) {
    writer.bool(target.is_some());
    if let Some(target) = target {
        put_node_target(writer, target);
    }
}

fn read_optional_node_target(reader: &mut Reader<'_>) -> Result<Option<NodeTarget>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_node_target(reader)?))
    } else {
        Ok(None)
    }
}

fn put_query_batch_request(w: &mut Writer, v: &QueryBatchRequest) -> Result<()> {
    put_workspace(w, v.workspace);
    w.u64(v.revision.get());
    put_count(w, v.queries.len())?;
    for i in &v.queries {
        w.u64(i.id.get());
        put_query(w, &i.query)?
    }
    Ok(())
}
fn read_query_batch_request(r: &mut Reader<'_>) -> Result<QueryBatchRequest> {
    let workspace = read_workspace(r)?;
    let revision = Revision::new(r.u64().map_err(protocol_codec)?);
    let n = r.count(MAX_BATCH_QUERIES).map_err(protocol_codec)?;
    let mut queries = Vec::with_capacity(n);
    for _ in 0..n {
        queries.push(QueryItem {
            id: crate::ids::QueryId::new(r.u64().map_err(protocol_codec)?),
            query: read_query(r)?,
        });
    }
    Ok(QueryBatchRequest {
        workspace,
        revision,
        queries,
    })
}
fn put_page_request(w: &mut Writer, p: PageRequest) {
    w.bool(p.after.is_some());
    if let Some(c) = p.after {
        put_cursor(w, c)
    }
    w.u32(p.limit)
}
fn read_page_request(r: &mut Reader<'_>) -> Result<PageRequest> {
    let after = if r.bool().map_err(protocol_codec)? {
        Some(read_cursor(r)?)
    } else {
        None
    };
    Ok(PageRequest {
        after,
        limit: r.u32().map_err(protocol_codec)?,
    })
}
fn put_repair_target(w: &mut Writer, t: RepairTarget) {
    match t {
        RepairTarget::Hole(n) => {
            w.u8(1);
            put_node_id(w, n)
        }
        RepairTarget::Operand { operation, index } => {
            w.u8(2);
            put_node_id(w, operation);
            w.u8(index)
        }
    }
}
fn read_repair_target(r: &mut Reader<'_>) -> Result<RepairTarget> {
    match r.u8().map_err(protocol_codec)? {
        1 => Ok(RepairTarget::Hole(read_node_id(r)?)),
        2 => Ok(RepairTarget::Operand {
            operation: read_node_id(r)?,
            index: r.u8().map_err(protocol_codec)?,
        }),
        tag => Err(protocol_codec(
            r.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}
fn put_query(w: &mut Writer, q: &Query) -> Result<()> {
    w.u8(q.code().stable_tag());
    match q {
        Query::WorkspaceSummary => {}
        Query::Node { node, expand } => {
            put_node_id(w, *node);
            w.bool(*expand)
        }
        Query::Blockers { page } => put_page_request(w, *page),
        Query::OwnerChain { node, page } => {
            put_node_id(w, *node);
            put_page_request(w, *page)
        }
        Query::Body { block, page } => {
            put_node_id(w, *block);
            put_page_request(w, *page)
        }
        Query::IncomingUses { value, page } => {
            put_value_ref(w, *value);
            put_page_request(w, *page)
        }
        Query::DefinitionReferences { target, page } => {
            put_node_id(w, *target);
            put_page_request(w, *page)
        }
        Query::Dependencies { node, page } => {
            put_node_id(w, *node);
            put_page_request(w, *page)
        }
        Query::VisibleValues {
            purpose,
            target,
            include_incompatible,
            page,
        } => {
            w.u8(purpose.stable_tag());
            put_repair_target(w, *target);
            w.bool(*include_incompatible);
            put_page_request(w, *page)
        }
        Query::LegalConstructors {
            target,
            include_incompatible,
            values,
        } => {
            put_repair_target(w, *target);
            w.bool(*include_incompatible);
            put_page_request(w, *values)
        }
        Query::SemanticDiff { from, page } => {
            w.u64(from.get());
            put_page_request(w, *page)
        }
        Query::RepairContext { target, budget } => {
            put_repair_target(w, *target);
            put_budget(w, *budget)
        }
    }
    Ok(())
}
fn read_query(r: &mut Reader<'_>) -> Result<Query> {
    let tag = r.u8().map_err(protocol_codec)?;
    let code = QueryCode::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Query, tag)))?;
    Ok(match code {
        QueryCode::WorkspaceSummary => Query::WorkspaceSummary,
        QueryCode::Node => Query::Node {
            node: read_node_id(r)?,
            expand: r.bool().map_err(protocol_codec)?,
        },
        QueryCode::Blockers => Query::Blockers {
            page: read_page_request(r)?,
        },
        QueryCode::OwnerChain => Query::OwnerChain {
            node: read_node_id(r)?,
            page: read_page_request(r)?,
        },
        QueryCode::Body => Query::Body {
            block: read_node_id(r)?,
            page: read_page_request(r)?,
        },
        QueryCode::IncomingUses => Query::IncomingUses {
            value: read_value_ref(r)?,
            page: read_page_request(r)?,
        },
        QueryCode::DefinitionReferences => Query::DefinitionReferences {
            target: read_node_id(r)?,
            page: read_page_request(r)?,
        },
        QueryCode::Dependencies => Query::Dependencies {
            node: read_node_id(r)?,
            page: read_page_request(r)?,
        },
        QueryCode::VisibleValues => Query::VisibleValues {
            purpose: read_visible_cursor_purpose(r)?,
            target: read_repair_target(r)?,
            include_incompatible: r.bool().map_err(protocol_codec)?,
            page: read_page_request(r)?,
        },
        QueryCode::LegalConstructors => Query::LegalConstructors {
            target: read_repair_target(r)?,
            include_incompatible: r.bool().map_err(protocol_codec)?,
            values: read_page_request(r)?,
        },
        QueryCode::SemanticDiff => Query::SemanticDiff {
            from: Revision::new(r.u64().map_err(protocol_codec)?),
            page: read_page_request(r)?,
        },
        QueryCode::RepairContext => Query::RepairContext {
            target: read_repair_target(r)?,
            budget: read_budget(r)?,
        },
    })
}
fn put_budget(w: &mut Writer, b: ContextBudget) {
    w.u32(b.body_before);
    w.u32(b.body_after);
    w.u32(b.visible_values);
    w.u32(b.incoming_uses);
    w.bool(b.include_incompatible)
}
fn read_budget(r: &mut Reader<'_>) -> Result<ContextBudget> {
    Ok(ContextBudget {
        body_before: r.u32().map_err(protocol_codec)?,
        body_after: r.u32().map_err(protocol_codec)?,
        visible_values: r.u32().map_err(protocol_codec)?,
        incoming_uses: r.u32().map_err(protocol_codec)?,
        include_incompatible: r.bool().map_err(protocol_codec)?,
    })
}
fn read_visible_cursor_purpose(r: &mut Reader<'_>) -> Result<VisibleCursorPurpose> {
    let tag = r.u8().map_err(protocol_codec)?;
    VisibleCursorPurpose::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Cursor, tag)))
}
fn put_cursor(w: &mut Writer, c: PageCursor) {
    match c {
        PageCursor::Blockers {
            workspace,
            revision,
            next,
        } => {
            w.u8(1);
            put_workspace(w, workspace);
            w.u64(revision.get());
            w.u64(next)
        }
        PageCursor::OwnerChain {
            workspace,
            revision,
            node,
            next,
        } => {
            w.u8(2);
            put_workspace(w, workspace);
            w.u64(revision.get());
            put_node_id(w, node);
            w.u64(next)
        }
        PageCursor::Body {
            workspace,
            revision,
            block,
            next,
        } => {
            w.u8(3);
            put_workspace(w, workspace);
            w.u64(revision.get());
            put_node_id(w, block);
            w.u64(next)
        }
        PageCursor::IncomingUses {
            workspace,
            revision,
            value,
            next,
        } => {
            w.u8(4);
            put_workspace(w, workspace);
            w.u64(revision.get());
            put_value_ref(w, value);
            w.u64(next)
        }
        PageCursor::DefinitionReferences {
            workspace,
            revision,
            target,
            next,
        } => {
            w.u8(5);
            put_workspace(w, workspace);
            w.u64(revision.get());
            put_node_id(w, target);
            w.u64(next)
        }
        PageCursor::Dependencies {
            workspace,
            revision,
            node,
            next,
        } => {
            w.u8(6);
            put_workspace(w, workspace);
            w.u64(revision.get());
            put_node_id(w, node);
            w.u64(next)
        }
        PageCursor::VisibleValues {
            workspace,
            revision,
            purpose,
            target,
            expected,
            include_incompatible,
            next,
        } => {
            w.u8(7);
            put_workspace(w, workspace);
            w.u64(revision.get());
            w.u8(purpose.stable_tag());
            put_repair_target(w, target);
            put_type(w, expected);
            w.bool(include_incompatible);
            w.u64(next)
        }
        PageCursor::Diff {
            workspace,
            from,
            to,
            next,
        } => {
            w.u8(8);
            put_workspace(w, workspace);
            w.u64(from.get());
            w.u64(to.get());
            w.u64(next)
        }
    }
}
fn read_cursor(r: &mut Reader<'_>) -> Result<PageCursor> {
    let tag = r.u8().map_err(protocol_codec)?;
    Ok(match tag {
        1 => PageCursor::Blockers {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            next: r.u64().map_err(protocol_codec)?,
        },
        2 => PageCursor::OwnerChain {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            node: read_node_id(r)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        3 => PageCursor::Body {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            block: read_node_id(r)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        4 => PageCursor::IncomingUses {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            value: read_value_ref(r)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        5 => PageCursor::DefinitionReferences {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            target: read_node_id(r)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        6 => PageCursor::Dependencies {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            node: read_node_id(r)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        7 => PageCursor::VisibleValues {
            workspace: read_workspace(r)?,
            revision: Revision::new(r.u64().map_err(protocol_codec)?),
            purpose: read_visible_cursor_purpose(r)?,
            target: read_repair_target(r)?,
            expected: read_type(r)?,
            include_incompatible: r.bool().map_err(protocol_codec)?,
            next: r.u64().map_err(protocol_codec)?,
        },
        8 => PageCursor::Diff {
            workspace: read_workspace(r)?,
            from: Revision::new(r.u64().map_err(protocol_codec)?),
            to: Revision::new(r.u64().map_err(protocol_codec)?),
            next: r.u64().map_err(protocol_codec)?,
        },
        _ => {
            return Err(protocol_codec(r.unknown_tag(TagDomain::Cursor, tag)));
        }
    })
}
fn put_query_batch_result(w: &mut Writer, v: &QueryBatchResult) -> Result<()> {
    put_workspace(w, v.workspace);
    w.u64(v.revision.get());
    put_count(w, v.results.len())?;
    for i in &v.results {
        w.u64(i.id.get());
        match &i.outcome {
            QueryOutcome::Success(v) => {
                w.u8(1);
                put_query_result(w, v)?
            }
            QueryOutcome::Error(e) => {
                w.u8(2);
                put_error(w, e)?
            }
        }
    }
    Ok(())
}
fn read_query_batch_result(r: &mut Reader<'_>) -> Result<QueryBatchResult> {
    let workspace = read_workspace(r)?;
    let revision = Revision::new(r.u64().map_err(protocol_codec)?);
    let n = r.count(MAX_BATCH_QUERIES).map_err(protocol_codec)?;
    let mut results = Vec::with_capacity(n);
    for _ in 0..n {
        let id = crate::ids::QueryId::new(r.u64().map_err(protocol_codec)?);
        let outcome = match r.u8().map_err(protocol_codec)? {
            1 => QueryOutcome::Success(Box::new(read_query_result(r)?)),
            2 => QueryOutcome::Error(read_error(r)?),
            tag => {
                return Err(protocol_codec(
                    r.unknown_tag(TagDomain::ProtocolMessage, tag),
                ));
            }
        };
        results.push(QueryItemResult { id, outcome });
    }
    Ok(QueryBatchResult {
        workspace,
        revision,
        results,
    })
}
fn put_query_result(w: &mut Writer, v: &QueryResult) -> Result<()> {
    match v {
        QueryResult::WorkspaceSummary(x) => {
            w.u8(1);
            put_workspace_summary(w, x)?
        }
        QueryResult::Node(x) => {
            w.u8(2);
            put_node_view(w, x)?
        }
        QueryResult::Blockers(x) => {
            w.u8(3);
            put_page_blocker(w, x)?
        }
        QueryResult::OwnerChain(x) => {
            w.u8(4);
            put_page_owner(w, x)?
        }
        QueryResult::Body(x) => {
            w.u8(5);
            put_page_body(w, x)?
        }
        QueryResult::IncomingUses(x) => {
            w.u8(6);
            put_page_use(w, x)?
        }
        QueryResult::DefinitionReferences(x) => {
            w.u8(7);
            put_page_def(w, x)?
        }
        QueryResult::Dependencies(x) => {
            w.u8(8);
            put_page_dep(w, x)?
        }
        QueryResult::VisibleValues(x) => {
            w.u8(9);
            put_page_visible(w, x)?
        }
        QueryResult::LegalConstructors(x) => {
            w.u8(10);
            put_legal(w, x)?
        }
        QueryResult::SemanticDiff(x) => {
            w.u8(11);
            put_diff_page(w, x)?
        }
        QueryResult::RepairContext(x) => {
            w.u8(12);
            put_context(w, x)?
        }
    }
    Ok(())
}
fn read_query_result(r: &mut Reader<'_>) -> Result<QueryResult> {
    Ok(match r.u8().map_err(protocol_codec)? {
        1 => QueryResult::WorkspaceSummary(read_workspace_summary(r)?),
        2 => QueryResult::Node(read_node_view(r)?),
        3 => QueryResult::Blockers(read_page_blocker(r)?),
        4 => QueryResult::OwnerChain(read_page_owner(r)?),
        5 => QueryResult::Body(read_page_body(r)?),
        6 => QueryResult::IncomingUses(read_page_use(r)?),
        7 => QueryResult::DefinitionReferences(read_page_def(r)?),
        8 => QueryResult::Dependencies(read_page_dep(r)?),
        9 => QueryResult::VisibleValues(read_page_visible(r)?),
        10 => QueryResult::LegalConstructors(read_legal(r)?),
        11 => QueryResult::SemanticDiff(read_diff_page(r)?),
        12 => QueryResult::RepairContext(Box::new(read_context(r)?)),
        tag => {
            return Err(protocol_codec(
                r.unknown_tag(TagDomain::ProtocolMessage, tag),
            ));
        }
    })
}

fn put_page_head<T>(w: &mut Writer, p: &Page<T>) -> Result<()> {
    put_count(w, p.items.len())?;
    w.bool(p.next.is_some());
    if let Some(c) = p.next {
        put_cursor(w, c)
    }
    w.bool(p.total.is_some());
    if let Some(t) = p.total {
        w.u64(t)
    }
    Ok(())
}
fn read_page_head(r: &mut Reader<'_>) -> Result<(usize, Option<PageCursor>, Option<u64>)> {
    let n = r.count(MAX_PAGE_ITEMS as usize).map_err(protocol_codec)?;
    let next = if r.bool().map_err(protocol_codec)? {
        Some(read_cursor(r)?)
    } else {
        None
    };
    let total = if r.bool().map_err(protocol_codec)? {
        Some(r.u64().map_err(protocol_codec)?)
    } else {
        None
    };
    Ok((n, next, total))
}
fn put_blocker(w: &mut Writer, b: &CompletenessBlocker) {
    put_node_id(w, b.owner);
    put_optional_node_id(w, b.target);
    w.u8(match b.category {
        ExpectedCategory::EntryFunction => 1,
        ExpectedCategory::FunctionBody => 2,
        ExpectedCategory::Expression => 3,
    });
    put_optional_type(w, b.expected_type)
}
fn read_blocker(r: &mut Reader<'_>) -> Result<CompletenessBlocker> {
    let owner = read_node_id(r)?;
    let target = read_optional_node_id(r)?;
    let category = match r.u8().map_err(protocol_codec)? {
        1 => ExpectedCategory::EntryFunction,
        2 => ExpectedCategory::FunctionBody,
        3 => ExpectedCategory::Expression,
        tag => {
            return Err(protocol_codec(
                r.unknown_tag(TagDomain::ProtocolMessage, tag),
            ));
        }
    };
    Ok(CompletenessBlocker {
        owner,
        target,
        category,
        expected_type: read_optional_type(r)?,
    })
}
fn put_page_blocker(w: &mut Writer, p: &Page<CompletenessBlocker>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_blocker(w, x)
    }
    Ok(())
}
fn read_page_blocker(r: &mut Reader<'_>) -> Result<Page<CompletenessBlocker>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_blocker(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_owner(w: &mut Writer, x: &OwnerFact) -> Result<()> {
    put_node_id(w, x.node);
    w.u8(x.kind.stable_tag());
    put_name(w, &x.name)
}
fn read_owner(r: &mut Reader<'_>) -> Result<OwnerFact> {
    Ok(OwnerFact {
        node: read_node_id(r)?,
        kind: read_node_kind(r)?,
        name: read_name(r)?,
    })
}
fn put_page_owner(w: &mut Writer, p: &Page<OwnerFact>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_owner(w, x)?
    }
    Ok(())
}
fn read_page_owner(r: &mut Reader<'_>) -> Result<Page<OwnerFact>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_owner(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_value_ref(w: &mut Writer, v: ValueRef) {
    match v {
        ValueRef::FunctionParameter(n) => {
            w.u8(1);
            put_node_id(w, n)
        }
        ValueRef::OperationResult { operation, output } => {
            w.u8(2);
            put_node_id(w, operation);
            w.u8(output)
        }
    }
}
fn read_value_ref(r: &mut Reader<'_>) -> Result<ValueRef> {
    match r.u8().map_err(protocol_codec)? {
        1 => Ok(ValueRef::FunctionParameter(read_node_id(r)?)),
        2 => Ok(ValueRef::OperationResult {
            operation: read_node_id(r)?,
            output: r.u8().map_err(protocol_codec)?,
        }),
        tag => Err(protocol_codec(r.unknown_tag(TagDomain::Value, tag))),
    }
}
fn put_literal(w: &mut Writer, v: &Option<LiteralValue>) {
    w.bool(v.is_some());
    if let Some(v) = v {
        match v {
            LiteralValue::I64(x) => {
                w.u8(1);
                w.i64(*x)
            }
            LiteralValue::Bool(x) => {
                w.u8(2);
                w.bool(*x)
            }
            LiteralValue::ExpectedType(x) => {
                w.u8(3);
                put_type(w, *x)
            }
        }
    }
}
fn read_literal(r: &mut Reader<'_>) -> Result<Option<LiteralValue>> {
    if !r.bool().map_err(protocol_codec)? {
        return Ok(None);
    }
    Ok(Some(match r.u8().map_err(protocol_codec)? {
        1 => LiteralValue::I64(r.i64().map_err(protocol_codec)?),
        2 => LiteralValue::Bool(r.bool().map_err(protocol_codec)?),
        3 => LiteralValue::ExpectedType(read_type(r)?),
        tag => {
            return Err(protocol_codec(
                r.unknown_tag(TagDomain::ProtocolMessage, tag),
            ));
        }
    }))
}
fn put_body(w: &mut Writer, x: &BodyItem) -> Result<()> {
    put_node_id(w, x.operation);
    w.u64(x.ordinal);
    w.u8(x.code.stable_tag());
    put_count(w, x.result_types.len())?;
    for t in &x.result_types {
        put_type(w, *t)
    }
    put_count(w, x.operands.len())?;
    for v in &x.operands {
        put_value_ref(w, *v)
    }
    w.bool(x.complete);
    w.bool(x.terminator);
    put_literal(w, &x.literal);
    Ok(())
}
fn read_body(r: &mut Reader<'_>) -> Result<BodyItem> {
    let operation = read_node_id(r)?;
    let ordinal = r.u64().map_err(protocol_codec)?;
    let tag = r.u8().map_err(protocol_codec)?;
    let code = OperationCode::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, tag)))?;
    let n = r.count(8).map_err(protocol_codec)?;
    let mut result_types = Vec::with_capacity(n);
    for _ in 0..n {
        result_types.push(read_type(r)?)
    }
    let n = r.count(8).map_err(protocol_codec)?;
    let mut operands = Vec::with_capacity(n);
    for _ in 0..n {
        operands.push(read_value_ref(r)?)
    }
    Ok(BodyItem {
        operation,
        ordinal,
        code,
        result_types,
        operands,
        complete: r.bool().map_err(protocol_codec)?,
        terminator: r.bool().map_err(protocol_codec)?,
        literal: read_literal(r)?,
    })
}
fn put_page_body(w: &mut Writer, p: &Page<BodyItem>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_body(w, x)?
    }
    Ok(())
}
fn read_page_body(r: &mut Reader<'_>) -> Result<Page<BodyItem>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_body(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_use(w: &mut Writer, x: &UseSite) {
    put_node_id(w, x.source);
    w.u8(x.operand_index);
    put_value_ref(w, x.target);
    put_node_id(w, x.owner_block);
    put_node_id(w, x.owner_function);
    put_type(w, x.expected_type);
    w.u8(1)
}
fn read_use(r: &mut Reader<'_>) -> Result<UseSite> {
    let source = read_node_id(r)?;
    let operand_index = r.u8().map_err(protocol_codec)?;
    let target = read_value_ref(r)?;
    let owner_block = read_node_id(r)?;
    let owner_function = read_node_id(r)?;
    let expected_type = read_type(r)?;
    if r.u8().map_err(protocol_codec)? != 1 {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "unknown operand use",
        ));
    }
    Ok(UseSite {
        source,
        operand_index,
        target,
        owner_block,
        owner_function,
        expected_type,
        use_mode: OperandUse::Copy,
    })
}
fn put_page_use(w: &mut Writer, p: &Page<UseSite>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_use(w, x)
    }
    Ok(())
}
fn read_page_use(r: &mut Reader<'_>) -> Result<Page<UseSite>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_use(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_def(w: &mut Writer, x: &DefinitionReferenceSite) {
    put_node_id(w, x.source);
    w.u8(1);
    put_node_id(w, x.target)
}
fn read_def(r: &mut Reader<'_>) -> Result<DefinitionReferenceSite> {
    let source = read_node_id(r)?;
    if r.u8().map_err(protocol_codec)? != 1 {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "unknown definition slot",
        ));
    }
    Ok(DefinitionReferenceSite {
        source,
        slot: DefinitionSlot::PackageEntry,
        target: read_node_id(r)?,
    })
}
fn put_page_def(w: &mut Writer, p: &Page<DefinitionReferenceSite>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_def(w, x)
    }
    Ok(())
}
fn read_page_def(r: &mut Reader<'_>) -> Result<Page<DefinitionReferenceSite>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_def(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_dep(w: &mut Writer, x: &DependencyFact) {
    match x {
        DependencyFact::ValueOperand { index, value } => {
            w.u8(1);
            w.u8(*index);
            put_value_ref(w, *value)
        }
        DependencyFact::Definition { target, .. } => {
            w.u8(2);
            w.u8(1);
            put_node_id(w, *target)
        }
    }
}
fn read_dep(r: &mut Reader<'_>) -> Result<DependencyFact> {
    match r.u8().map_err(protocol_codec)? {
        1 => Ok(DependencyFact::ValueOperand {
            index: r.u8().map_err(protocol_codec)?,
            value: read_value_ref(r)?,
        }),
        2 => {
            if r.u8().map_err(protocol_codec)? != 1 {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "unknown definition slot",
                ));
            }
            Ok(DependencyFact::Definition {
                slot: DefinitionSlot::PackageEntry,
                target: read_node_id(r)?,
            })
        }
        tag => Err(protocol_codec(
            r.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}
fn put_page_dep(w: &mut Writer, p: &Page<DependencyFact>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_dep(w, x)
    }
    Ok(())
}
fn read_page_dep(r: &mut Reader<'_>) -> Result<Page<DependencyFact>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_dep(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_visible(w: &mut Writer, x: &VisibleValue) -> Result<()> {
    put_value_ref(w, x.value);
    put_type(w, x.ty);
    w.bool(x.compatible);
    put_node_id(w, x.producer);
    w.bool(x.producer_code.is_some());
    if let Some(c) = x.producer_code {
        w.u8(c.stable_tag())
    }
    put_node_id(w, x.owner_function);
    w.bool(x.ordinal.is_some());
    if let Some(v) = x.ordinal {
        w.u64(v)
    }
    put_name(w, &x.name)
}
fn read_visible(r: &mut Reader<'_>) -> Result<VisibleValue> {
    let value = read_value_ref(r)?;
    let ty = read_type(r)?;
    let compatible = r.bool().map_err(protocol_codec)?;
    let producer = read_node_id(r)?;
    let producer_code = if r.bool().map_err(protocol_codec)? {
        let t = r.u8().map_err(protocol_codec)?;
        Some(
            OperationCode::from_stable_tag(t)
                .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, t)))?,
        )
    } else {
        None
    };
    let owner_function = read_node_id(r)?;
    let ordinal = if r.bool().map_err(protocol_codec)? {
        Some(r.u64().map_err(protocol_codec)?)
    } else {
        None
    };
    Ok(VisibleValue {
        value,
        ty,
        compatible,
        producer,
        producer_code,
        owner_function,
        ordinal,
        name: read_name(r)?,
    })
}
fn put_page_visible(w: &mut Writer, p: &Page<VisibleValue>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_visible(w, x)?
    }
    Ok(())
}
fn read_page_visible(r: &mut Reader<'_>) -> Result<Page<VisibleValue>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_visible(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_constructor(w: &mut Writer, x: &ConstructorDescriptor) -> Result<()> {
    w.u8(x.code.stable_tag());
    put_type(w, x.result_type);
    put_count(w, x.operand_types.len())?;
    for t in &x.operand_types {
        put_type(w, *t)
    }
    put_count(w, x.operand_uses.len())?;
    for _ in &x.operand_uses {
        w.u8(1)
    }
    put_count(w, x.literal_fields.len())?;
    for f in &x.literal_fields {
        w.u8(match f {
            LiteralField::I64Value => 1,
            LiteralField::BoolValue => 2,
            LiteralField::ExpectedType => 3,
        })
    }
    w.bool(x.complete);
    w.bool(x.terminator);
    Ok(())
}
fn read_constructor(r: &mut Reader<'_>) -> Result<ConstructorDescriptor> {
    let t = r.u8().map_err(protocol_codec)?;
    let code = OperationCode::from_stable_tag(t)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, t)))?;
    let result_type = read_type(r)?;
    let n = r.count(8).map_err(protocol_codec)?;
    let mut operand_types = Vec::with_capacity(n);
    for _ in 0..n {
        operand_types.push(read_type(r)?)
    }
    let n = r.count(8).map_err(protocol_codec)?;
    let mut operand_uses = Vec::with_capacity(n);
    for _ in 0..n {
        if r.u8().map_err(protocol_codec)? != 1 {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "unknown operand use",
            ));
        }
        operand_uses.push(OperandUse::Copy)
    }
    let n = r.count(8).map_err(protocol_codec)?;
    let mut literal_fields = Vec::with_capacity(n);
    for _ in 0..n {
        literal_fields.push(match r.u8().map_err(protocol_codec)? {
            1 => LiteralField::I64Value,
            2 => LiteralField::BoolValue,
            3 => LiteralField::ExpectedType,
            tag => {
                return Err(protocol_codec(
                    r.unknown_tag(TagDomain::ProtocolMessage, tag),
                ));
            }
        })
    }
    Ok(ConstructorDescriptor {
        code,
        result_type,
        operand_types,
        operand_uses,
        literal_fields,
        complete: r.bool().map_err(protocol_codec)?,
        terminator: r.bool().map_err(protocol_codec)?,
    })
}
fn put_legal(w: &mut Writer, x: &LegalConstructorsResult) -> Result<()> {
    put_repair_target(w, x.target);
    put_type(w, x.expected_type);
    put_count(w, x.constructors.len())?;
    for c in &x.constructors {
        put_constructor(w, c)?
    }
    put_page_visible(w, &x.visible_values)
}
fn read_legal(r: &mut Reader<'_>) -> Result<LegalConstructorsResult> {
    let target = read_repair_target(r)?;
    let expected_type = read_type(r)?;
    let n = r.count(8).map_err(protocol_codec)?;
    let mut constructors = Vec::with_capacity(n);
    for _ in 0..n {
        constructors.push(read_constructor(r)?)
    }
    Ok(LegalConstructorsResult {
        target,
        expected_type,
        constructors,
        visible_values: read_page_visible(r)?,
    })
}

fn put_operation_kind(w: &mut Writer, o: &OperationKind) {
    w.u8(o.code().stable_tag());
    match o {
        OperationKind::ConstI64(v) => w.i64(*v),
        OperationKind::ConstBool(v) => w.bool(*v),
        OperationKind::AddI64 { lhs, rhs } => {
            put_value_ref(w, *lhs);
            put_value_ref(w, *rhs)
        }
        OperationKind::Hole { expected } => put_type(w, *expected),
        OperationKind::Return { value } => put_value_ref(w, *value),
    }
}
fn read_operation_kind(r: &mut Reader<'_>) -> Result<OperationKind> {
    let t = r.u8().map_err(protocol_codec)?;
    let c = OperationCode::from_stable_tag(t)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, t)))?;
    Ok(match c {
        OperationCode::ConstI64 => OperationKind::ConstI64(r.i64().map_err(protocol_codec)?),
        OperationCode::ConstBool => OperationKind::ConstBool(r.bool().map_err(protocol_codec)?),
        OperationCode::AddI64 => OperationKind::AddI64 {
            lhs: read_value_ref(r)?,
            rhs: read_value_ref(r)?,
        },
        OperationCode::Hole => OperationKind::Hole {
            expected: read_type(r)?,
        },
        OperationCode::Return => OperationKind::Return {
            value: read_value_ref(r)?,
        },
    })
}
fn put_opt_value(w: &mut Writer, v: Option<ValueRef>) {
    w.bool(v.is_some());
    if let Some(v) = v {
        put_value_ref(w, v)
    }
}
fn read_opt_value(r: &mut Reader<'_>) -> Result<Option<ValueRef>> {
    if r.bool().map_err(protocol_codec)? {
        Ok(Some(read_value_ref(r)?))
    } else {
        Ok(None)
    }
}
fn put_scalar(w: &mut Writer, v: &ScalarValue) {
    match v {
        ScalarValue::I64(x) => {
            w.u8(1);
            w.i64(*x)
        }
        ScalarValue::Bool(x) => {
            w.u8(2);
            w.bool(*x)
        }
        ScalarValue::Type(x) => {
            w.u8(3);
            put_type(w, *x)
        }
    }
}
fn read_scalar(r: &mut Reader<'_>) -> Result<ScalarValue> {
    match r.u8().map_err(protocol_codec)? {
        1 => Ok(ScalarValue::I64(r.i64().map_err(protocol_codec)?)),
        2 => Ok(ScalarValue::Bool(r.bool().map_err(protocol_codec)?)),
        3 => Ok(ScalarValue::Type(read_type(r)?)),
        tag => Err(protocol_codec(r.unknown_tag(TagDomain::Change, tag))),
    }
}
fn put_change(w: &mut Writer, x: &Change) -> Result<()> {
    put_node_id(w, x.node);
    w.u8(x.kind.stable_tag());
    match &x.kind {
        ChangeKind::Created { kind } | ChangeKind::Deleted { kind } => w.u8(kind.stable_tag()),
        ChangeKind::Renamed { before, after } => {
            w.string(before).map_err(protocol_codec)?;
            w.string(after).map_err(protocol_codec)?
        }
        ChangeKind::ScalarAttributeChanged { before, after } => {
            put_scalar(w, before);
            put_scalar(w, after)
        }
        ChangeKind::ContainmentChanged {
            before_count,
            after_count,
        } => {
            w.u64(*before_count);
            w.u64(*after_count)
        }
        ChangeKind::OperandChanged {
            index,
            before,
            after,
        } => {
            w.u8(*index);
            put_opt_value(w, *before);
            put_opt_value(w, *after)
        }
        ChangeKind::EntryFunctionChanged { before, after } => {
            put_optional_node_id(w, *before);
            put_optional_node_id(w, *after)
        }
        ChangeKind::CompletenessChanged { complete } => w.bool(*complete),
        ChangeKind::OperationRefined {
            before,
            after,
            result_type,
            replacement,
        } => {
            w.u8(before.stable_tag());
            w.u8(after.stable_tag());
            put_type(w, *result_type);
            put_operation_kind(w, replacement)
        }
        ChangeKind::AllocatedAndTombstoned => {}
    }
    Ok(())
}
fn read_change(r: &mut Reader<'_>) -> Result<Change> {
    let node = read_node_id(r)?;
    let tag = r.u8().map_err(protocol_codec)?;
    let kind = match tag {
        1 => ChangeKind::Created {
            kind: read_node_kind(r)?,
        },
        2 => ChangeKind::Deleted {
            kind: read_node_kind(r)?,
        },
        3 => ChangeKind::Renamed {
            before: read_protocol_string(r)?,
            after: read_protocol_string(r)?,
        },
        4 => ChangeKind::ScalarAttributeChanged {
            before: read_scalar(r)?,
            after: read_scalar(r)?,
        },
        5 => ChangeKind::ContainmentChanged {
            before_count: r.u64().map_err(protocol_codec)?,
            after_count: r.u64().map_err(protocol_codec)?,
        },
        6 => ChangeKind::OperandChanged {
            index: r.u8().map_err(protocol_codec)?,
            before: read_opt_value(r)?,
            after: read_opt_value(r)?,
        },
        8 => ChangeKind::EntryFunctionChanged {
            before: read_optional_node_id(r)?,
            after: read_optional_node_id(r)?,
        },
        9 => ChangeKind::CompletenessChanged {
            complete: r.bool().map_err(protocol_codec)?,
        },
        10 => {
            let b = r.u8().map_err(protocol_codec)?;
            let a = r.u8().map_err(protocol_codec)?;
            ChangeKind::OperationRefined {
                before: OperationCode::from_stable_tag(b)
                    .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, b)))?,
                after: OperationCode::from_stable_tag(a)
                    .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, a)))?,
                result_type: read_type(r)?,
                replacement: read_operation_kind(r)?,
            }
        }
        11 => ChangeKind::AllocatedAndTombstoned,
        _ => return Err(protocol_codec(r.unknown_tag(TagDomain::Change, tag))),
    };
    Ok(Change { node, kind })
}
fn put_page_change(w: &mut Writer, p: &Page<Change>) -> Result<()> {
    put_page_head(w, p)?;
    for x in &p.items {
        put_change(w, x)?
    }
    Ok(())
}
fn read_page_change(r: &mut Reader<'_>) -> Result<Page<Change>> {
    let (n, next, total) = read_page_head(r)?;
    let mut items = Vec::with_capacity(n);
    for _ in 0..n {
        items.push(read_change(r)?)
    }
    Ok(Page { items, next, total })
}
fn put_diff_page(w: &mut Writer, x: &SemanticDiffPage) -> Result<()> {
    w.u64(x.from.get());
    w.u64(x.to.get());
    w.u64(x.change_count);
    w.fixed(&x.change_digest.as_bytes());
    put_page_change(w, &x.page)
}
fn read_diff_page(r: &mut Reader<'_>) -> Result<SemanticDiffPage> {
    Ok(SemanticDiffPage {
        from: Revision::new(r.u64().map_err(protocol_codec)?),
        to: Revision::new(r.u64().map_err(protocol_codec)?),
        change_count: r.u64().map_err(protocol_codec)?,
        change_digest: read_change_digest(r)?,
        page: read_page_change(r)?,
    })
}
fn put_signature(w: &mut Writer, x: FunctionSignatureSummary) {
    w.u64(x.parameter_count);
    put_type(w, x.result)
}
fn read_signature(r: &mut Reader<'_>) -> Result<FunctionSignatureSummary> {
    Ok(FunctionSignatureSummary {
        parameter_count: r.u64().map_err(protocol_codec)?,
        result: read_type(r)?,
    })
}
fn put_context(w: &mut Writer, x: &RepairContext) -> Result<()> {
    put_workspace(w, x.workspace);
    w.u64(x.revision.get());
    put_repair_target(w, x.target);
    put_node_id(w, x.operation);
    w.u8(x.operation_code.stable_tag());
    w.bool(x.operand_index.is_some());
    if let Some(i) = x.operand_index {
        w.u8(i)
    }
    put_type(w, x.expected_type);
    w.bool(x.use_mode.is_some());
    if x.use_mode.is_some() {
        w.u8(1)
    }
    put_opt_value(w, x.current_value);
    put_optional_type(w, x.current_actual_type);
    put_node_id(w, x.owner_block);
    put_node_id(w, x.owner_function);
    w.u64(x.ordinal);
    put_signature(w, x.function_signature);
    put_count(w, x.owner_chain.len())?;
    for o in &x.owner_chain {
        put_owner(w, o)?
    }
    put_count(w, x.body_window.len())?;
    for b in &x.body_window {
        put_body(w, b)?
    }
    put_page_visible(w, &x.visible_values)?;
    put_page_use(w, &x.incoming_uses)?;
    put_count(w, x.legal_constructors.len())?;
    for c in &x.legal_constructors {
        put_constructor(w, c)?
    }
    w.bool(x.blocker.is_some());
    if let Some(b) = &x.blocker {
        put_blocker(w, b)
    }
    w.bool(x.refinement_operation.is_some());
    if let Some(c) = x.refinement_operation {
        w.u8(c.stable_tag())
    }
    Ok(())
}
fn read_context(r: &mut Reader<'_>) -> Result<RepairContext> {
    let workspace = read_workspace(r)?;
    let revision = Revision::new(r.u64().map_err(protocol_codec)?);
    let target = read_repair_target(r)?;
    let operation = read_node_id(r)?;
    let t = r.u8().map_err(protocol_codec)?;
    let operation_code = OperationCode::from_stable_tag(t)
        .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::Operation, t)))?;
    let operand_index = if r.bool().map_err(protocol_codec)? {
        Some(r.u8().map_err(protocol_codec)?)
    } else {
        None
    };
    let expected_type = read_type(r)?;
    let use_mode = if r.bool().map_err(protocol_codec)? {
        if r.u8().map_err(protocol_codec)? != 1 {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "unknown operand use",
            ));
        }
        Some(OperandUse::Copy)
    } else {
        None
    };
    let current_value = read_opt_value(r)?;
    let current_actual_type = read_optional_type(r)?;
    let owner_block = read_node_id(r)?;
    let owner_function = read_node_id(r)?;
    let ordinal = r.u64().map_err(protocol_codec)?;
    let function_signature = read_signature(r)?;
    let n = r.count(16).map_err(protocol_codec)?;
    let mut owner_chain = Vec::with_capacity(n);
    for _ in 0..n {
        owner_chain.push(read_owner(r)?)
    }
    let n = r
        .count((MAX_CONTEXT_ITEMS * 2 + 1) as usize)
        .map_err(protocol_codec)?;
    let mut body_window = Vec::with_capacity(n);
    for _ in 0..n {
        body_window.push(read_body(r)?)
    }
    let visible_values = read_page_visible(r)?;
    let incoming_uses = read_page_use(r)?;
    let n = r.count(8).map_err(protocol_codec)?;
    let mut legal_constructors = Vec::with_capacity(n);
    for _ in 0..n {
        legal_constructors.push(read_constructor(r)?)
    }
    let blocker = if r.bool().map_err(protocol_codec)? {
        Some(read_blocker(r)?)
    } else {
        None
    };
    let refinement_operation = if r.bool().map_err(protocol_codec)? {
        let t = r.u8().map_err(protocol_codec)?;
        Some(
            TransactionOpCode::from_stable_tag(t)
                .ok_or_else(|| protocol_codec(r.unknown_tag(TagDomain::TransactionOperation, t)))?,
        )
    } else {
        None
    };
    Ok(RepairContext {
        workspace,
        revision,
        target,
        operation,
        operation_code,
        operand_index,
        expected_type,
        use_mode,
        current_value,
        current_actual_type,
        owner_block,
        owner_function,
        ordinal,
        function_signature,
        owner_chain,
        body_window,
        visible_values,
        incoming_uses,
        legal_constructors,
        blocker,
        refinement_operation,
    })
}

fn put_workspace_summary(w: &mut Writer, s: &WorkspaceSummary) -> Result<()> {
    put_workspace(w, s.workspace);
    w.u64(s.revision.get());
    w.fixed(&s.hash.as_bytes());
    put_node_id(w, s.root);
    w.u64(s.node_count);
    w.bool(s.complete);
    w.u64(s.blocker_count);
    w.u64(s.entry_count);
    Ok(())
}
fn read_workspace_summary(r: &mut Reader<'_>) -> Result<WorkspaceSummary> {
    Ok(WorkspaceSummary {
        workspace: read_workspace(r)?,
        revision: Revision::new(r.u64().map_err(protocol_codec)?),
        hash: read_hash(r)?,
        root: read_node_id(r)?,
        node_count: r.u64().map_err(protocol_codec)?,
        complete: r.bool().map_err(protocol_codec)?,
        blocker_count: r.u64().map_err(protocol_codec)?,
        entry_count: r.u64().map_err(protocol_codec)?,
    })
}

fn put_node_view(writer: &mut Writer, view: &NodeView) -> Result<()> {
    put_node_summary(writer, &view.summary)?;
    writer.bool(view.record.is_some());
    if let Some(record) = &view.record {
        crate::artifact::put_node(writer, record)?;
    }
    Ok(())
}

fn read_node_view(reader: &mut Reader<'_>) -> Result<NodeView> {
    let summary = read_node_summary(reader)?;
    let record = if reader.bool().map_err(protocol_codec)? {
        Some(crate::artifact::read_node(
            reader,
            summary.workspace,
            DecodePolicy::default(),
        )?)
    } else {
        None
    };
    Ok(NodeView { summary, record })
}

fn put_name(w: &mut Writer, v: &Option<NamePreview>) -> Result<()> {
    w.bool(v.is_some());
    if let Some(v) = v {
        w.string(&v.value).map_err(protocol_codec)?;
        w.bool(v.truncated)
    }
    Ok(())
}
fn read_name(r: &mut Reader<'_>) -> Result<Option<NamePreview>> {
    if !r.bool().map_err(protocol_codec)? {
        return Ok(None);
    }
    Ok(Some(NamePreview {
        value: read_protocol_string(r)?,
        truncated: r.bool().map_err(protocol_codec)?,
    }))
}
fn put_node_summary(w: &mut Writer, s: &NodeSummary) -> Result<()> {
    put_workspace(w, s.workspace);
    w.u64(s.revision.get());
    put_node_id(w, s.node);
    w.u8(s.kind.stable_tag());
    put_optional_node_id(w, s.owner);
    put_name(w, &s.display_name)?;
    w.bool(s.signature.is_some());
    if let Some(v) = s.signature {
        w.u64(v.parameter_count);
        put_type(w, v.result)
    }
    put_optional_type(w, s.value_type);
    w.bool(s.complete);
    w.u64(s.blocker_count);
    w.u64(s.child_count);
    w.u64(s.outgoing_reference_count);
    Ok(())
}
fn read_node_summary(r: &mut Reader<'_>) -> Result<NodeSummary> {
    let workspace = read_workspace(r)?;
    let revision = Revision::new(r.u64().map_err(protocol_codec)?);
    let node = read_node_id(r)?;
    let kind = read_node_kind(r)?;
    let owner = read_optional_node_id(r)?;
    let display_name = read_name(r)?;
    let signature = if r.bool().map_err(protocol_codec)? {
        Some(FunctionSignatureSummary {
            parameter_count: r.u64().map_err(protocol_codec)?,
            result: read_type(r)?,
        })
    } else {
        None
    };
    Ok(NodeSummary {
        workspace,
        revision,
        node,
        kind,
        owner,
        display_name,
        signature,
        value_type: read_optional_type(r)?,
        complete: r.bool().map_err(protocol_codec)?,
        blocker_count: r.u64().map_err(protocol_codec)?,
        child_count: r.u64().map_err(protocol_codec)?,
        outgoing_reference_count: r.u64().map_err(protocol_codec)?,
    })
}

fn put_run_result(writer: &mut Writer, result: &RunResult) {
    match result.value {
        RuntimeValue::Unit => writer.u8(1),
        RuntimeValue::Bool(value) => {
            writer.u8(2);
            writer.bool(value);
        }
        RuntimeValue::I64(value) => {
            writer.u8(3);
            writer.i64(value);
        }
    }
    writer.u64(result.compile_nanoseconds);
    writer.u64(result.execute_nanoseconds);
}

fn read_run_result(reader: &mut Reader<'_>) -> Result<RunResult> {
    let tag = reader.u8().map_err(protocol_codec)?;
    let value = match tag {
        1 => RuntimeValue::Unit,
        2 => RuntimeValue::Bool(reader.bool().map_err(protocol_codec)?),
        3 => RuntimeValue::I64(reader.i64().map_err(protocol_codec)?),
        _ => {
            return Err(protocol_codec(
                reader.unknown_tag(TagDomain::RuntimeValue, tag),
            ));
        }
    };
    Ok(RunResult {
        value,
        compile_nanoseconds: reader.u64().map_err(protocol_codec)?,
        execute_nanoseconds: reader.u64().map_err(protocol_codec)?,
    })
}

fn put_error(writer: &mut Writer, error: &LkError) -> Result<()> {
    writer.u8(error.code.stable_tag());
    put_optional_workspace(writer, error.workspace);
    put_optional_revision(writer, error.revision);
    put_optional_u32(writer, error.operation_index);
    put_optional_node_id(writer, error.target);
    put_optional_kind(writer, error.expected_kind);
    put_optional_kind(writer, error.actual_kind);
    put_optional_type(writer, error.expected_type);
    put_optional_type(writer, error.actual_type);
    put_count(writer, error.related.len())?;
    for related in &error.related {
        put_node_id(writer, *related);
    }
    writer.bool(error.retryable);
    writer.string(&error.message).map_err(protocol_codec)?;
    Ok(())
}

fn read_error(reader: &mut Reader<'_>) -> Result<LkError> {
    let tag = reader.u8().map_err(protocol_codec)?;
    let code = ErrorCode::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(reader.unknown_tag(TagDomain::Error, tag)))?;
    let workspace = read_optional_workspace(reader)?;
    let revision = read_optional_revision(reader)?;
    let operation_index = read_optional_u32(reader)?;
    let target = read_optional_node_id(reader)?;
    let expected_kind = read_optional_kind(reader)?;
    let actual_kind = read_optional_kind(reader)?;
    let expected_type = read_optional_type(reader)?;
    let actual_type = read_optional_type(reader)?;
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut related = Vec::with_capacity(count);
    for _ in 0..count {
        related.push(read_node_id(reader)?);
    }
    let retryable = reader.bool().map_err(protocol_codec)?;
    let message = read_protocol_string(reader)?;
    Ok(LkError {
        code,
        workspace,
        revision,
        operation_index,
        target,
        expected_kind,
        actual_kind,
        expected_type,
        actual_type,
        related,
        retryable,
        message,
    })
}

fn write_frame(writer: &mut impl Write, body: &[u8]) -> Result<()> {
    if body.len() > MAXIMUM_FRAME_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "protocol frame exceeds boundary policy",
        ));
    }
    let length = u32::try_from(body.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "protocol frame length exceeds canonical u32 framing",
        )
    })?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(body)?;
    writer.flush()?;
    Ok(())
}

fn read_frame(reader: &mut impl Read) -> Result<Option<Vec<u8>>> {
    let mut header = [0_u8; 4];
    let mut read = 0;
    while read < header.len() {
        match reader.read(&mut header[read..]) {
            Ok(0) if read == 0 => return Ok(None),
            Ok(0) => {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "connection ended inside a frame header",
                ));
            }
            Ok(count) => read += count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error.into()),
        }
    }
    let length = usize::try_from(u32::from_le_bytes(header)).map_err(|_| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            "frame length overflows host indexes",
        )
    })?;
    if length > MAXIMUM_FRAME_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "protocol frame exceeds boundary policy",
        ));
    }
    let mut frame = vec![0_u8; length];
    reader.read_exact(&mut frame).map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "connection ended inside a protocol frame",
            )
        } else {
            error.into()
        }
    })?;
    Ok(Some(frame))
}

fn put_workspace(writer: &mut Writer, workspace: WorkspaceId) {
    writer.fixed(&workspace.as_bytes());
}

fn read_workspace(reader: &mut Reader<'_>) -> Result<WorkspaceId> {
    let mut bytes = [0_u8; WorkspaceId::BYTE_LEN];
    bytes.copy_from_slice(
        reader
            .fixed(WorkspaceId::BYTE_LEN)
            .map_err(protocol_codec)?,
    );
    Ok(WorkspaceId::from_bytes(bytes))
}

fn put_node_id(writer: &mut Writer, node: NodeId) {
    put_workspace(writer, node.workspace());
    writer.u64(node.serial());
}

fn read_node_id(reader: &mut Reader<'_>) -> Result<NodeId> {
    let workspace = read_workspace(reader)?;
    let serial = reader.u64().map_err(protocol_codec)?;
    NodeId::new(workspace, serial).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("protocol contains an invalid node identity: {error}"),
        )
    })
}

fn read_node_kind(reader: &mut Reader<'_>) -> Result<NodeKind> {
    let tag = reader.u8().map_err(protocol_codec)?;
    NodeKind::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(reader.unknown_tag(TagDomain::Node, tag)))
}

fn put_type(writer: &mut Writer, ty: SemanticType) {
    writer.u8(ty.stable_tag());
}

fn read_type(reader: &mut Reader<'_>) -> Result<SemanticType> {
    let tag = reader.u8().map_err(protocol_codec)?;
    SemanticType::from_stable_tag(tag)
        .ok_or_else(|| protocol_codec(reader.unknown_tag(TagDomain::SemanticType, tag)))
}

fn read_hash(reader: &mut Reader<'_>) -> Result<SnapshotHash> {
    let mut bytes = [0_u8; SnapshotHash::BYTE_LEN];
    bytes.copy_from_slice(
        reader
            .fixed(SnapshotHash::BYTE_LEN)
            .map_err(protocol_codec)?,
    );
    Ok(SnapshotHash::from_bytes(bytes))
}

fn read_change_digest(reader: &mut Reader<'_>) -> Result<ChangeDigest> {
    let mut bytes = [0_u8; ChangeDigest::BYTE_LEN];
    bytes.copy_from_slice(
        reader
            .fixed(ChangeDigest::BYTE_LEN)
            .map_err(protocol_codec)?,
    );
    Ok(ChangeDigest::from_bytes(bytes))
}

fn read_handle(reader: &mut Reader<'_>) -> Result<LocalHandle> {
    Ok(LocalHandle::new(reader.u32().map_err(protocol_codec)?))
}

fn put_optional_idempotency(writer: &mut Writer, value: Option<IdempotencyKey>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        writer.fixed(&value.as_bytes());
    }
}

fn read_optional_idempotency(reader: &mut Reader<'_>) -> Result<Option<IdempotencyKey>> {
    if !reader.bool().map_err(protocol_codec)? {
        return Ok(None);
    }
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(reader.fixed(16).map_err(protocol_codec)?);
    Ok(Some(IdempotencyKey::from_bytes(bytes)))
}

fn put_count(writer: &mut Writer, value: usize) -> Result<()> {
    if value > MAXIMUM_FRAME_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "protocol collection exceeds boundary item policy",
        ));
    }
    let value = u64::try_from(value).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "protocol collection length exceeds canonical u64 encoding",
        )
    })?;
    writer.u64(value);
    Ok(())
}

fn read_protocol_string(reader: &mut Reader<'_>) -> Result<String> {
    reader.string(MAXIMUM_FRAME_BYTES).map_err(protocol_codec)
}

fn put_optional_workspace(writer: &mut Writer, value: Option<WorkspaceId>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        put_workspace(writer, value);
    }
}

fn read_optional_workspace(reader: &mut Reader<'_>) -> Result<Option<WorkspaceId>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_workspace(reader)?))
    } else {
        Ok(None)
    }
}

fn put_optional_revision(writer: &mut Writer, value: Option<Revision>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        writer.u64(value.get());
    }
}

fn read_optional_revision(reader: &mut Reader<'_>) -> Result<Option<Revision>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(Revision::new(reader.u64().map_err(protocol_codec)?)))
    } else {
        Ok(None)
    }
}

fn put_optional_u32(writer: &mut Writer, value: Option<u32>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        writer.u32(value);
    }
}

fn read_optional_u32(reader: &mut Reader<'_>) -> Result<Option<u32>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(reader.u32().map_err(protocol_codec)?))
    } else {
        Ok(None)
    }
}

fn put_optional_node_id(writer: &mut Writer, value: Option<NodeId>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        put_node_id(writer, value);
    }
}

fn read_optional_node_id(reader: &mut Reader<'_>) -> Result<Option<NodeId>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_node_id(reader)?))
    } else {
        Ok(None)
    }
}

fn put_optional_kind(writer: &mut Writer, value: Option<NodeKind>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        writer.u8(value.stable_tag());
    }
}

fn read_optional_kind(reader: &mut Reader<'_>) -> Result<Option<NodeKind>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_node_kind(reader)?))
    } else {
        Ok(None)
    }
}

fn put_optional_type(writer: &mut Writer, value: Option<SemanticType>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        put_type(writer, value);
    }
}

fn read_optional_type(reader: &mut Reader<'_>) -> Result<Option<SemanticType>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_type(reader)?))
    } else {
        Ok(None)
    }
}

fn protocol_codec(error: CodecError) -> LkError {
    let code = if error.kind == CodecErrorKind::PolicyExceeded {
        ErrorCode::PolicyExceeded
    } else {
        ErrorCode::ProtocolMalformed
    };
    LkError::new(code, format!("typed protocol decoding failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::QueryId;

    #[test]
    fn schema_description_round_trips_through_binary_protocol() {
        let request_id = RequestId::new(77);
        let request = Request::DescribeSchema;
        let mut request_bytes = Vec::new();
        write_request(&mut request_bytes, request_id, &request).expect("schema request encode");
        assert_eq!(
            read_request(&mut request_bytes.as_slice()).expect("schema request decode"),
            Some((request_id, request))
        );

        let response = Response::SchemaDescription(Box::new(crate::machine::schema_description()));
        let mut response_bytes = Vec::new();
        write_response(&mut response_bytes, request_id, &response).expect("schema response encode");
        assert_eq!(
            read_response(&mut response_bytes.as_slice()).expect("schema response decode"),
            Some((request_id, response))
        );

        let mut request_names = std::collections::BTreeSet::new();
        let mut request_tags = std::collections::BTreeSet::new();
        assert!(RequestCode::ALL.into_iter().all(|code| {
            request_names.insert(code.machine_name()) && request_tags.insert(code.stable_tag())
        }));
        let mut response_names = std::collections::BTreeSet::new();
        let mut response_tags = std::collections::BTreeSet::new();
        assert!(ResponseCode::ALL.into_iter().all(|code| {
            response_names.insert(code.machine_name()) && response_tags.insert(code.stable_tag())
        }));
    }

    #[test]
    fn fabricated_schema_descriptions_reject_after_binary_decode() {
        let canonical = crate::machine::schema_description();
        let mut fabricated = Vec::new();

        let mut wrong_version = canonical.clone();
        wrong_version.json_envelope_version += 1;
        fabricated.push(wrong_version);
        let mut wrong_name = canonical.clone();
        wrong_name.requests[0].name = "fabricated".to_owned();
        fabricated.push(wrong_name);
        let mut wrong_tag = canonical.clone();
        wrong_tag.errors[0].tag = 99;
        fabricated.push(wrong_tag);
        let mut wrong_limit = canonical.clone();
        wrong_limit.limits.maximum_frame_items += 1;
        fabricated.push(wrong_limit);
        let mut duplicate = canonical;
        duplicate.queries.push(duplicate.queries[0].clone());
        fabricated.push(duplicate);

        for schema in fabricated {
            let mut bytes = Vec::new();
            write_response(
                &mut bytes,
                RequestId::new(78),
                &Response::SchemaDescription(Box::new(schema)),
            )
            .expect("fabricated schema encode");
            assert_eq!(
                read_response(&mut bytes.as_slice())
                    .expect_err("fabricated schema must reject")
                    .code,
                ErrorCode::ProtocolMalformed
            );
        }
    }

    #[test]
    fn request_id_zero_is_reserved_but_uncorrelated_response_zero_is_encodable() {
        assert_eq!(
            write_request(
                &mut Vec::new(),
                RequestId::new(0),
                &Request::CreateWorkspace
            )
            .expect_err("zero request ID")
            .code,
            ErrorCode::ProtocolMalformed
        );

        let mut body = Writer::new();
        body.u16(PROTOCOL_VERSION);
        body.u64(0);
        body.u8(RequestCode::CreateWorkspace.stable_tag());
        let body = body.finish();
        let mut frame = Vec::new();
        frame.extend_from_slice(
            &u32::try_from(body.len())
                .expect("frame length")
                .to_le_bytes(),
        );
        frame.extend_from_slice(&body);
        assert_eq!(
            read_request(&mut frame.as_slice())
                .expect_err("decoded zero request ID")
                .code,
            ErrorCode::ProtocolMalformed
        );

        let response = Response::Error(LkError::new(
            ErrorCode::ProtocolMalformed,
            "uncorrelated malformed request",
        ));
        let mut bytes = Vec::new();
        write_response(&mut bytes, RequestId::new(0), &response).expect("zero response ID");
        assert_eq!(
            read_response(&mut bytes.as_slice()).expect("zero response decode"),
            Some((RequestId::new(0), response))
        );
    }

    #[test]
    fn binary_query_codec_preserves_semantically_invalid_typed_requests() {
        let workspace = WorkspaceId::from_bytes([0x91; 16]);
        let request = Request::QueryBatch(QueryBatchRequest {
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
                    query: Query::WorkspaceSummary,
                },
            ],
        });
        let mut bytes = Vec::new();
        write_request(&mut bytes, RequestId::new(79), &request).expect("binary query encode");
        assert_eq!(
            read_request(&mut bytes.as_slice()).expect("binary query decode"),
            Some((RequestId::new(79), request))
        );
    }

    #[test]
    fn unknown_request_kind_and_trailing_bytes_reject() {
        let mut unknown = Vec::new();
        unknown.extend_from_slice(&11_u32.to_le_bytes());
        unknown.extend_from_slice(&PROTOCOL_VERSION.to_le_bytes());
        unknown.extend_from_slice(&1_u64.to_le_bytes());
        unknown.push(99);
        assert_eq!(
            read_request(&mut unknown.as_slice())
                .err()
                .map(|error| error.code),
            Some(ErrorCode::ProtocolMalformed)
        );

        let mut body = Writer::new();
        body.u16(PROTOCOL_VERSION);
        body.u64(1);
        body.u8(1);
        body.u8(0);
        let bytes = body.finish();
        let mut frame = Vec::new();
        frame.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(0).to_le_bytes());
        frame.extend_from_slice(&bytes);
        assert_eq!(
            read_request(&mut frame.as_slice())
                .err()
                .map(|error| error.code),
            Some(ErrorCode::ProtocolMalformed)
        );
    }

    #[test]
    fn operation_protocol_tags_round_trip_every_closed_code() {
        let handle = LocalHandle::new(7);
        let value = ValueDraft::OperationResult {
            operation: NodeTarget::Local(handle),
            output: 0,
        };
        let operations = [
            OperationDraft::ConstI64(-7),
            OperationDraft::ConstBool(true),
            OperationDraft::AddI64 {
                lhs: value,
                rhs: value,
            },
            OperationDraft::Hole {
                expected: SemanticType::Bool,
            },
            OperationDraft::Return { value },
        ];
        assert_eq!(operations.len(), OperationCode::ALL.len());
        for operation in operations {
            let mut writer = Writer::new();
            put_operation_draft(&mut writer, &operation);
            let bytes = writer.finish();
            let mut reader = Reader::new(&bytes);
            assert_eq!(
                read_operation_draft(&mut reader).expect("operation draft round trip"),
                operation
            );
            reader.finish().expect("complete operation draft payload");
        }
    }

    #[test]
    fn compact_receipt_maximum_projection_round_trips_below_frame_policy() {
        let workspace = WorkspaceId::from_bytes([0x62; 16]);
        let returned_bindings = (0..MAX_RETURNED_BINDINGS)
            .map(|index| {
                let handle = LocalHandle::new(u32::try_from(index).expect("handle"));
                let node = NodeId::new(workspace, u64::try_from(index).expect("serial") + 2)
                    .expect("node");
                (handle, node)
            })
            .collect();
        let receipt = TransactionReceipt {
            workspace,
            base_revision: Revision::new(7),
            revision: Revision::new(8),
            hash: SnapshotHash::from_bytes([0x11; 32]),
            published: true,
            created_count: 10_000,
            returned_bindings,
            change_count: 20_000,
            change_digest: ChangeDigest::from_bytes([0x22; 32]),
            complete_before: false,
            complete_after: true,
            blocker_count_before: 1,
            blocker_count_after: 0,
        };
        let response = Response::TransactionReceipt(receipt);
        let size = encoded_response_size(RequestId::new(4), &response).expect("receipt size");
        assert!(size < MAXIMUM_FRAME_BYTES);
        assert!(size < 4096);
        let mut bytes = Vec::new();
        write_response(&mut bytes, RequestId::new(4), &response).expect("encode receipt");
        assert_eq!(
            read_response(&mut bytes.as_slice()).expect("decode receipt"),
            Some((RequestId::new(4), response))
        );
    }

    #[test]
    fn apply_fingerprint_binds_projection_and_refinement_round_trips() {
        let workspace = WorkspaceId::from_bytes([0x63; 16]);
        let transaction = Transaction {
            workspace,
            base_revision: Revision::new(1),
            idempotency_key: Some(IdempotencyKey::from_bytes([3; 16])),
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(NodeId::new(workspace, 9).expect("hole")),
                replacement: OperationDraft::ConstI64(42),
            }],
        };
        let first = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec::default(),
        };
        let second = ApplyTransactionRequest {
            transaction,
            response: TransactionResponseSpec {
                return_handles: vec![LocalHandle::new(1)],
            },
        };
        assert_ne!(
            transaction_fingerprint(&first).expect("first fingerprint"),
            transaction_fingerprint(&second).expect("second fingerprint")
        );
        let request = Request::ApplyTransaction(first);
        let mut bytes = Vec::new();
        write_request(&mut bytes, RequestId::new(5), &request).expect("encode request");
        assert_eq!(
            read_request(&mut bytes.as_slice()).expect("decode request"),
            Some((RequestId::new(5), request))
        );
    }

    #[test]
    fn every_query_variant_and_maximum_batch_round_trip() {
        let workspace = WorkspaceId::from_bytes([0x62; 16]);
        let node = NodeId::new(workspace, 2).expect("node");
        let value = ValueRef::OperationResult {
            operation: node,
            output: 0,
        };
        let page = PageRequest {
            after: None,
            limit: 1,
        };
        let target = RepairTarget::Hole(node);
        let queries = vec![
            Query::WorkspaceSummary,
            Query::Node { node, expand: true },
            Query::Blockers { page },
            Query::OwnerChain { node, page },
            Query::Body { block: node, page },
            Query::IncomingUses { value, page },
            Query::DefinitionReferences { target: node, page },
            Query::Dependencies { node, page },
            Query::VisibleValues {
                purpose: VisibleCursorPurpose::VisibleValues,
                target,
                include_incompatible: true,
                page,
            },
            Query::LegalConstructors {
                target,
                include_incompatible: true,
                values: page,
            },
            Query::SemanticDiff {
                from: Revision::INITIAL,
                page,
            },
            Query::RepairContext {
                target,
                budget: ContextBudget {
                    body_before: 1,
                    body_after: 1,
                    visible_values: 1,
                    incoming_uses: 1,
                    include_incompatible: true,
                },
            },
        ];
        assert_eq!(queries.len(), QueryCode::ALL.len());
        for query in queries {
            let mut w = Writer::new();
            put_query(&mut w, &query).expect("encode query");
            let bytes = w.finish();
            let mut r = Reader::new(&bytes);
            assert_eq!(read_query(&mut r).expect("decode query"), query);
            r.finish().expect("query trailing");
        }
        let request = Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(1),
            queries: (0..MAX_BATCH_QUERIES)
                .map(|i| QueryItem {
                    id: crate::ids::QueryId::new(i as u64),
                    query: Query::Body {
                        block: node,
                        page: PageRequest {
                            after: None,
                            limit: 64,
                        },
                    },
                })
                .collect(),
        });
        let Request::QueryBatch(edge) = &request else {
            unreachable!()
        };
        validate_batch(edge).expect("actual 32 by 64 aggregate edge");
        let mut bytes = Vec::new();
        write_request(&mut bytes, RequestId::new(8), &request).expect("max batch encode");
        assert!(bytes.len() < MAXIMUM_FRAME_BYTES);
        assert_eq!(
            read_request(&mut bytes.as_slice()).expect("max batch decode"),
            Some((RequestId::new(8), request))
        );
        let body_item = BodyItem {
            operation: node,
            ordinal: 0,
            code: OperationCode::ConstI64,
            result_types: vec![SemanticType::I64],
            operands: Vec::new(),
            complete: true,
            terminator: false,
            literal: Some(LiteralValue::I64(1)),
        };
        let response = Response::QueryBatchResult(QueryBatchResult {
            workspace,
            revision: Revision::new(1),
            results: (0..MAX_BATCH_QUERIES)
                .map(|i| QueryItemResult {
                    id: crate::ids::QueryId::new(i as u64),
                    outcome: QueryOutcome::Success(Box::new(QueryResult::Body(Page {
                        items: vec![body_item.clone(); 64],
                        next: None,
                        total: Some(64),
                    }))),
                })
                .collect(),
        });
        let size = encoded_response_size(RequestId::new(9), &response)
            .expect("maximum legal aggregate response");
        assert!(size < MAXIMUM_FRAME_BYTES);
        let mut bytes = Vec::new();
        write_response(&mut bytes, RequestId::new(9), &response)
            .expect("encode legal aggregate response");
        assert_eq!(
            read_response(&mut bytes.as_slice()).expect("decode legal aggregate response"),
            Some((RequestId::new(9), response))
        );
    }

    #[test]
    fn unknown_query_result_and_cursor_tags_reject() {
        let workspace = WorkspaceId::from_bytes([0x64; 16]);
        let request_body = |query_bytes: &[u8]| {
            let mut body = Writer::new();
            body.u16(PROTOCOL_VERSION);
            body.u64(1);
            body.u8(3);
            put_workspace(&mut body, workspace);
            body.u64(0);
            body.u64(1);
            body.u64(1);
            body.fixed(query_bytes);
            let body = body.finish();
            let mut frame = Vec::new();
            frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
            frame.extend_from_slice(&body);
            frame
        };
        assert_eq!(
            read_request(&mut request_body(&[0xff]).as_slice())
                .expect_err("unknown query")
                .code,
            ErrorCode::ProtocolMalformed
        );
        let unknown_purpose = [QueryCode::VisibleValues.stable_tag(), 0xff];
        assert_eq!(
            read_request(&mut request_body(&unknown_purpose).as_slice())
                .expect_err("unknown visible cursor purpose")
                .code,
            ErrorCode::ProtocolMalformed
        );
        let mut query = Writer::new();
        query.u8(QueryCode::Body.stable_tag());
        put_node_id(&mut query, NodeId::new(workspace, 2).expect("block"));
        query.bool(true);
        query.u8(0xff);
        query.u32(1);
        assert_eq!(
            read_request(&mut request_body(&query.finish()).as_slice())
                .expect_err("unknown cursor")
                .code,
            ErrorCode::ProtocolMalformed
        );
        let mut body = Writer::new();
        body.u16(PROTOCOL_VERSION);
        body.u64(1);
        body.u8(103);
        put_workspace(&mut body, workspace);
        body.u64(0);
        body.u64(1);
        body.u64(1);
        body.u8(1);
        body.u8(0xff);
        let body = body.finish();
        let mut frame = Vec::new();
        frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
        frame.extend_from_slice(&body);
        assert_eq!(
            read_response(&mut frame.as_slice())
                .expect_err("unknown query result")
                .code,
            ErrorCode::ProtocolMalformed
        );
    }

    #[test]
    fn protocol_version_one_is_rejected_without_fallback() {
        let mut body = Vec::new();
        body.extend_from_slice(&1_u16.to_le_bytes());
        body.extend_from_slice(&7_u64.to_le_bytes());
        body.push(1);
        let mut frame = Vec::new();
        frame.extend_from_slice(&u32::try_from(body.len()).expect("length").to_le_bytes());
        frame.extend_from_slice(&body);
        assert_eq!(
            read_request(&mut frame.as_slice())
                .expect_err("version one must reject")
                .code,
            ErrorCode::ProtocolVersion
        );
    }

    #[test]
    fn unknown_transaction_operation_rejects_before_workspace_mutation() {
        let mut body = Writer::new();
        body.u16(PROTOCOL_VERSION);
        body.u64(7);
        body.u8(2);
        body.fixed(&WorkspaceId::from_bytes([0x61; 16]).as_bytes());
        body.u64(0);
        body.bool(false);
        body.u8(1);
        body.u64(1);
        body.u8(0xff);
        let body = body.finish();
        let mut frame = Vec::new();
        frame.extend_from_slice(
            &u32::try_from(body.len())
                .expect("frame length")
                .to_le_bytes(),
        );
        frame.extend_from_slice(&body);
        assert_eq!(
            read_request(&mut frame.as_slice())
                .expect_err("unknown transaction operation")
                .code,
            ErrorCode::ProtocolMalformed
        );
    }

    #[test]
    fn one_exact_frame_accepts_and_connection_trailing_bytes_reject() {
        let request_id = RequestId::new(91);
        let request = Request::CreateWorkspace;
        let mut exact_request = Vec::new();
        write_request(&mut exact_request, request_id, &request).expect("encode exact request");
        assert_eq!(
            read_request(&mut exact_request.as_slice()).expect("decode exact request"),
            Some((request_id, request))
        );

        let mut trailing_request = exact_request;
        trailing_request.push(0xff);
        assert_eq!(
            read_request(&mut trailing_request.as_slice())
                .expect_err("connection trailing request bytes must reject before dispatch")
                .code,
            ErrorCode::ProtocolMalformed
        );

        let response = Response::Acknowledged;
        let mut exact_response = Vec::new();
        write_response(&mut exact_response, request_id, &response).expect("encode exact response");
        assert_eq!(
            read_response(&mut exact_response.as_slice()).expect("decode exact response"),
            Some((request_id, response))
        );

        let mut trailing_response = exact_response;
        trailing_response.push(0xff);
        assert_eq!(
            read_response(&mut trailing_response.as_slice())
                .expect_err("connection trailing response bytes must reject")
                .code,
            ErrorCode::ProtocolMalformed
        );
    }

    #[test]
    fn truncated_and_oversized_frames_reject_at_the_protocol_boundary() {
        let truncated = [4_u8, 0, 0, 0, 1, 2];
        assert_eq!(
            read_request(&mut truncated.as_slice())
                .expect_err("truncated frame")
                .code,
            ErrorCode::ProtocolMalformed
        );
        let oversized = u32::try_from(MAXIMUM_FRAME_BYTES + 1)
            .expect("frame policy fits u32")
            .to_le_bytes();
        assert_eq!(
            read_request(&mut oversized.as_slice())
                .expect_err("oversized frame")
                .code,
            ErrorCode::PolicyExceeded
        );
    }
}
