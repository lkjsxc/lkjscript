use crate::artifact::DecodePolicy;
use crate::codec::{CodecError, CodecErrorKind, Reader, TagDomain, Writer};
use crate::diff::{Change, ChangeKind, SemanticDiff};
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{
    IdempotencyKey, LocalHandle, NodeId, RequestId, Revision, SnapshotHash, WorkspaceId,
};
use crate::interpret::{RunResult, RuntimeValue};
use crate::query::{
    CompletenessBlocker, ExpectedCategory, FunctionSignature, NodeSummary, NodeView,
    WorkspaceSummary,
};
use crate::schema::{NodeKind, OperationDraft, SemanticType, ValueDraft};
use crate::transaction::{NodeTarget, Transaction, TransactionOp, TransactionResult};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const PROTOCOL_VERSION: u16 = 1;
pub const MAXIMUM_FRAME_BYTES: usize = 8 * 1024 * 1024;
pub const MAXIMUM_FRAME_ITEMS: usize = 100_000;
const CLIENT_IO_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Request {
    CreateWorkspace,
    ApplyTransaction(Transaction),
    WorkspaceSummary {
        workspace: WorkspaceId,
        revision: Revision,
    },
    Node {
        workspace: WorkspaceId,
        revision: Revision,
        node: NodeId,
        expand: bool,
    },
    Blockers {
        workspace: WorkspaceId,
        revision: Revision,
    },
    Run {
        workspace: WorkspaceId,
        revision: Revision,
        entry: NodeId,
    },
    Shutdown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Response {
    WorkspaceCreated(WorkspaceSummary),
    TransactionApplied(TransactionResult),
    WorkspaceSummary(WorkspaceSummary),
    Node(NodeView),
    Blockers {
        workspace: WorkspaceId,
        revision: Revision,
        blockers: Vec<CompletenessBlocker>,
    },
    Run(RunResult),
    Acknowledged,
    Error(LkError),
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
    let request = read_request_body(&mut body)?;
    body.finish().map_err(protocol_codec)?;
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
    Ok(Some((request_id, response)))
}

pub(crate) fn transaction_fingerprint(transaction: &Transaction) -> Result<[u8; 32]> {
    let mut writer = Writer::new();
    put_transaction(&mut writer, transaction)?;
    Ok(*blake3::hash(&writer.finish()).as_bytes())
}

pub(crate) fn put_transaction_result(
    writer: &mut Writer,
    result: &TransactionResult,
) -> Result<()> {
    put_workspace(writer, result.workspace);
    writer.u64(result.base_revision.get());
    writer.u64(result.revision.get());
    writer.fixed(&result.hash.as_bytes());
    put_count(writer, result.allocations.len())?;
    for (handle, node) in &result.allocations {
        writer.u32(handle.get());
        put_node_id(writer, *node);
    }
    put_diff(writer, &result.diff)?;
    writer.bool(result.published);
    Ok(())
}

pub(crate) fn read_transaction_result(reader: &mut Reader<'_>) -> Result<TransactionResult> {
    let workspace = read_workspace(reader)?;
    let base_revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let hash = read_hash(reader)?;
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut allocations = Vec::with_capacity(count);
    for _ in 0..count {
        let handle = LocalHandle::new(reader.u32().map_err(protocol_codec)?);
        let node = read_node_id(reader)?;
        allocations.push((handle, node));
    }
    let diff = read_diff(reader)?;
    let published = reader.bool().map_err(protocol_codec)?;
    Ok(TransactionResult {
        workspace,
        base_revision,
        revision,
        hash,
        allocations,
        diff,
        published,
    })
}

fn put_request(writer: &mut Writer, request: &Request) -> Result<()> {
    match request {
        Request::CreateWorkspace => writer.u8(1),
        Request::ApplyTransaction(transaction) => {
            writer.u8(2);
            put_transaction(writer, transaction)?;
        }
        Request::WorkspaceSummary {
            workspace,
            revision,
        } => {
            writer.u8(3);
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
        }
        Request::Node {
            workspace,
            revision,
            node,
            expand,
        } => {
            writer.u8(4);
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
            put_node_id(writer, *node);
            writer.bool(*expand);
        }
        Request::Blockers {
            workspace,
            revision,
        } => {
            writer.u8(5);
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
        }
        Request::Run {
            workspace,
            revision,
            entry,
        } => {
            writer.u8(6);
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
            put_node_id(writer, *entry);
        }
        Request::Shutdown => writer.u8(7),
    }
    Ok(())
}

fn read_request_body(reader: &mut Reader<'_>) -> Result<Request> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(Request::CreateWorkspace),
        2 => Ok(Request::ApplyTransaction(read_transaction(reader)?)),
        3 => Ok(Request::WorkspaceSummary {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
        }),
        4 => Ok(Request::Node {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
            node: read_node_id(reader)?,
            expand: reader.bool().map_err(protocol_codec)?,
        }),
        5 => Ok(Request::Blockers {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
        }),
        6 => Ok(Request::Run {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
            entry: read_node_id(reader)?,
        }),
        7 => Ok(Request::Shutdown),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}

fn put_response(writer: &mut Writer, response: &Response) -> Result<()> {
    match response {
        Response::WorkspaceCreated(summary) => {
            writer.u8(101);
            put_workspace_summary(writer, summary)?;
        }
        Response::TransactionApplied(result) => {
            writer.u8(102);
            put_transaction_result(writer, result)?;
        }
        Response::WorkspaceSummary(summary) => {
            writer.u8(103);
            put_workspace_summary(writer, summary)?;
        }
        Response::Node(view) => {
            writer.u8(104);
            put_node_view(writer, view)?;
        }
        Response::Blockers {
            workspace,
            revision,
            blockers,
        } => {
            writer.u8(105);
            put_workspace(writer, *workspace);
            writer.u64(revision.get());
            put_blockers(writer, blockers)?;
        }
        Response::Run(result) => {
            writer.u8(106);
            put_run_result(writer, result);
        }
        Response::Acknowledged => writer.u8(107),
        Response::Error(error) => {
            writer.u8(255);
            put_error(writer, error)?;
        }
    }
    Ok(())
}

fn read_response_body(reader: &mut Reader<'_>) -> Result<Response> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        101 => Ok(Response::WorkspaceCreated(read_workspace_summary(reader)?)),
        102 => Ok(Response::TransactionApplied(read_transaction_result(
            reader,
        )?)),
        103 => Ok(Response::WorkspaceSummary(read_workspace_summary(reader)?)),
        104 => Ok(Response::Node(read_node_view(reader)?)),
        105 => Ok(Response::Blockers {
            workspace: read_workspace(reader)?,
            revision: Revision::new(reader.u64().map_err(protocol_codec)?),
            blockers: read_blockers(reader)?,
        }),
        106 => Ok(Response::Run(read_run_result(reader)?)),
        107 => Ok(Response::Acknowledged),
        255 => Ok(Response::Error(read_error(reader)?)),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::ProtocolMessage, tag),
        )),
    }
}

fn put_transaction(writer: &mut Writer, transaction: &Transaction) -> Result<()> {
    put_workspace(writer, transaction.workspace);
    writer.u64(transaction.base_revision.get());
    put_optional_idempotency(writer, transaction.idempotency_key);
    writer.bool(transaction.dry_run);
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
    let dry_run = reader.bool().map_err(protocol_codec)?;
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut operations = Vec::with_capacity(count);
    for _ in 0..count {
        operations.push(read_transaction_operation(reader)?);
    }
    Ok(Transaction {
        workspace,
        base_revision,
        idempotency_key,
        dry_run,
        operations,
    })
}

fn put_transaction_operation(writer: &mut Writer, operation: &TransactionOp) -> Result<()> {
    match operation {
        TransactionOp::CreatePackage { handle, name } => {
            writer.u8(1);
            writer.u32(handle.get());
            writer.string(name).map_err(protocol_codec)?;
        }
        TransactionOp::CreateModule {
            handle,
            package,
            name,
        } => {
            writer.u8(2);
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
            writer.u8(3);
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
            writer.u8(4);
            writer.u32(handle.get());
            put_node_target(writer, *function);
            writer.string(name).map_err(protocol_codec)?;
            put_type(writer, *ty);
        }
        TransactionOp::CreateRegion { handle, function } => {
            writer.u8(5);
            writer.u32(handle.get());
            put_node_target(writer, *function);
        }
        TransactionOp::CreateBlock { handle, region } => {
            writer.u8(6);
            writer.u32(handle.get());
            put_node_target(writer, *region);
        }
        TransactionOp::CreateOperation {
            handle,
            block,
            before,
            operation,
        } => {
            writer.u8(7);
            writer.u32(handle.get());
            put_node_target(writer, *block);
            put_optional_node_target(writer, *before);
            put_operation_draft(writer, operation);
        }
        TransactionOp::SetFunctionBody { function, region } => {
            writer.u8(8);
            put_node_target(writer, *function);
            put_node_target(writer, *region);
        }
        TransactionOp::SetEntryFunction { package, function } => {
            writer.u8(9);
            put_node_target(writer, *package);
            put_node_target(writer, *function);
        }
        TransactionOp::RenameNode { node, name } => {
            writer.u8(10);
            put_node_target(writer, *node);
            writer.string(name).map_err(protocol_codec)?;
        }
        TransactionOp::ReplaceOperation {
            operation,
            replacement,
        } => {
            writer.u8(11);
            put_node_target(writer, *operation);
            put_operation_draft(writer, replacement);
        }
        TransactionOp::ReplaceOperand {
            operation,
            index,
            value,
        } => {
            writer.u8(12);
            put_node_target(writer, *operation);
            writer.u8(*index);
            put_value_draft(writer, *value);
        }
        TransactionOp::DeleteOwnedSubtree { root } => {
            writer.u8(13);
            put_node_target(writer, *root);
        }
    }
    Ok(())
}

fn read_transaction_operation(reader: &mut Reader<'_>) -> Result<TransactionOp> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(TransactionOp::CreatePackage {
            handle: read_handle(reader)?,
            name: read_protocol_string(reader)?,
        }),
        2 => Ok(TransactionOp::CreateModule {
            handle: read_handle(reader)?,
            package: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
        }),
        3 => Ok(TransactionOp::CreateFunction {
            handle: read_handle(reader)?,
            module: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
            result: read_type(reader)?,
        }),
        4 => Ok(TransactionOp::CreateParameter {
            handle: read_handle(reader)?,
            function: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
            ty: read_type(reader)?,
        }),
        5 => Ok(TransactionOp::CreateRegion {
            handle: read_handle(reader)?,
            function: read_node_target(reader)?,
        }),
        6 => Ok(TransactionOp::CreateBlock {
            handle: read_handle(reader)?,
            region: read_node_target(reader)?,
        }),
        7 => Ok(TransactionOp::CreateOperation {
            handle: read_handle(reader)?,
            block: read_node_target(reader)?,
            before: read_optional_node_target(reader)?,
            operation: read_operation_draft(reader)?,
        }),
        8 => Ok(TransactionOp::SetFunctionBody {
            function: read_node_target(reader)?,
            region: read_node_target(reader)?,
        }),
        9 => Ok(TransactionOp::SetEntryFunction {
            package: read_node_target(reader)?,
            function: read_node_target(reader)?,
        }),
        10 => Ok(TransactionOp::RenameNode {
            node: read_node_target(reader)?,
            name: read_protocol_string(reader)?,
        }),
        11 => Ok(TransactionOp::ReplaceOperation {
            operation: read_node_target(reader)?,
            replacement: read_operation_draft(reader)?,
        }),
        12 => Ok(TransactionOp::ReplaceOperand {
            operation: read_node_target(reader)?,
            index: reader.u8().map_err(protocol_codec)?,
            value: read_value_draft(reader)?,
        }),
        13 => Ok(TransactionOp::DeleteOwnedSubtree {
            root: read_node_target(reader)?,
        }),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::TransactionOperation, tag),
        )),
    }
}

fn put_operation_draft(writer: &mut Writer, operation: &OperationDraft) {
    match operation {
        OperationDraft::ConstI64(value) => {
            writer.u8(1);
            writer.i64(*value);
        }
        OperationDraft::ConstBool(value) => {
            writer.u8(2);
            writer.bool(*value);
        }
        OperationDraft::AddI64 { lhs, rhs } => {
            writer.u8(3);
            put_value_draft(writer, *lhs);
            put_value_draft(writer, *rhs);
        }
        OperationDraft::Hole { expected } => {
            writer.u8(4);
            put_type(writer, *expected);
        }
        OperationDraft::Return { value } => {
            writer.u8(5);
            put_value_draft(writer, *value);
        }
    }
}

fn read_operation_draft(reader: &mut Reader<'_>) -> Result<OperationDraft> {
    let tag = reader.u8().map_err(protocol_codec)?;
    match tag {
        1 => Ok(OperationDraft::ConstI64(
            reader.i64().map_err(protocol_codec)?,
        )),
        2 => Ok(OperationDraft::ConstBool(
            reader.bool().map_err(protocol_codec)?,
        )),
        3 => Ok(OperationDraft::AddI64 {
            lhs: read_value_draft(reader)?,
            rhs: read_value_draft(reader)?,
        }),
        4 => Ok(OperationDraft::Hole {
            expected: read_type(reader)?,
        }),
        5 => Ok(OperationDraft::Return {
            value: read_value_draft(reader)?,
        }),
        _ => Err(protocol_codec(
            reader.unknown_tag(TagDomain::Operation, tag),
        )),
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

fn put_diff(writer: &mut Writer, diff: &SemanticDiff) -> Result<()> {
    put_count(writer, diff.changes.len())?;
    for change in &diff.changes {
        put_node_id(writer, change.node);
        writer.u8(change.kind.stable_tag());
        match &change.kind {
            ChangeKind::Created { kind } | ChangeKind::Deleted { kind } => {
                writer.u8(kind.stable_tag());
            }
            ChangeKind::Renamed { before, after } => {
                writer.string(before).map_err(protocol_codec)?;
                writer.string(after).map_err(protocol_codec)?;
            }
            ChangeKind::CompletenessChanged { complete } => writer.bool(*complete),
            ChangeKind::ScalarAttributeChanged
            | ChangeKind::ContainmentChanged
            | ChangeKind::OperandChanged
            | ChangeKind::DirectReferenceChanged
            | ChangeKind::EntryFunctionChanged => {}
        }
    }
    Ok(())
}

fn read_diff(reader: &mut Reader<'_>) -> Result<SemanticDiff> {
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut changes = Vec::with_capacity(count);
    for _ in 0..count {
        let node = read_node_id(reader)?;
        let tag = reader.u8().map_err(protocol_codec)?;
        let kind = match tag {
            1 => ChangeKind::Created {
                kind: read_node_kind(reader)?,
            },
            2 => ChangeKind::Deleted {
                kind: read_node_kind(reader)?,
            },
            3 => ChangeKind::Renamed {
                before: read_protocol_string(reader)?,
                after: read_protocol_string(reader)?,
            },
            4 => ChangeKind::ScalarAttributeChanged,
            5 => ChangeKind::ContainmentChanged,
            6 => ChangeKind::OperandChanged,
            7 => ChangeKind::DirectReferenceChanged,
            8 => ChangeKind::EntryFunctionChanged,
            9 => ChangeKind::CompletenessChanged {
                complete: reader.bool().map_err(protocol_codec)?,
            },
            _ => return Err(protocol_codec(reader.unknown_tag(TagDomain::Change, tag))),
        };
        changes.push(Change { node, kind });
    }
    Ok(SemanticDiff { changes })
}

fn put_workspace_summary(writer: &mut Writer, summary: &WorkspaceSummary) -> Result<()> {
    put_workspace(writer, summary.workspace);
    writer.u64(summary.revision.get());
    writer.fixed(&summary.hash.as_bytes());
    put_node_id(writer, summary.root);
    writer.u64(summary.node_count);
    writer.bool(summary.complete);
    writer.u64(summary.blocker_count);
    put_count(writer, summary.entries.len())?;
    for entry in &summary.entries {
        put_node_id(writer, *entry);
    }
    Ok(())
}

fn read_workspace_summary(reader: &mut Reader<'_>) -> Result<WorkspaceSummary> {
    let workspace = read_workspace(reader)?;
    let revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let hash = read_hash(reader)?;
    let root = read_node_id(reader)?;
    let node_count = reader.u64().map_err(protocol_codec)?;
    let complete = reader.bool().map_err(protocol_codec)?;
    let blocker_count = reader.u64().map_err(protocol_codec)?;
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(read_node_id(reader)?);
    }
    Ok(WorkspaceSummary {
        workspace,
        revision,
        hash,
        root,
        node_count,
        complete,
        blocker_count,
        entries,
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

fn put_node_summary(writer: &mut Writer, summary: &NodeSummary) -> Result<()> {
    put_workspace(writer, summary.workspace);
    writer.u64(summary.revision.get());
    put_node_id(writer, summary.node);
    writer.u8(summary.kind.stable_tag());
    put_optional_node_id(writer, summary.owner);
    put_optional_string(writer, summary.display_name.as_deref())?;
    writer.bool(summary.signature.is_some());
    if let Some(signature) = &summary.signature {
        put_count(writer, signature.parameters.len())?;
        for (parameter, ty) in &signature.parameters {
            put_node_id(writer, *parameter);
            put_type(writer, *ty);
        }
        put_type(writer, signature.result);
    }
    put_optional_type(writer, summary.value_type);
    writer.bool(summary.complete);
    writer.u64(summary.diagnostic_count);
    writer.u64(summary.child_count);
    writer.u64(summary.reference_count);
    Ok(())
}

fn read_node_summary(reader: &mut Reader<'_>) -> Result<NodeSummary> {
    let workspace = read_workspace(reader)?;
    let revision = Revision::new(reader.u64().map_err(protocol_codec)?);
    let node = read_node_id(reader)?;
    let kind = read_node_kind(reader)?;
    let owner = read_optional_node_id(reader)?;
    let display_name = read_optional_string(reader)?;
    let signature = if reader.bool().map_err(protocol_codec)? {
        let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
        let mut parameters = Vec::with_capacity(count);
        for _ in 0..count {
            parameters.push((read_node_id(reader)?, read_type(reader)?));
        }
        Some(FunctionSignature {
            parameters,
            result: read_type(reader)?,
        })
    } else {
        None
    };
    let value_type = read_optional_type(reader)?;
    let complete = reader.bool().map_err(protocol_codec)?;
    let diagnostic_count = reader.u64().map_err(protocol_codec)?;
    let child_count = reader.u64().map_err(protocol_codec)?;
    let reference_count = reader.u64().map_err(protocol_codec)?;
    Ok(NodeSummary {
        workspace,
        revision,
        node,
        kind,
        owner,
        display_name,
        signature,
        value_type,
        complete,
        diagnostic_count,
        child_count,
        reference_count,
    })
}

fn put_blockers(writer: &mut Writer, blockers: &[CompletenessBlocker]) -> Result<()> {
    put_count(writer, blockers.len())?;
    for blocker in blockers {
        put_node_id(writer, blocker.owner);
        put_optional_node_id(writer, blocker.target);
        writer.u8(match blocker.category {
            ExpectedCategory::EntryFunction => 1,
            ExpectedCategory::FunctionBody => 2,
            ExpectedCategory::Expression => 3,
        });
        put_optional_type(writer, blocker.expected_type);
    }
    Ok(())
}

fn read_blockers(reader: &mut Reader<'_>) -> Result<Vec<CompletenessBlocker>> {
    let count = reader.count(MAXIMUM_FRAME_ITEMS).map_err(protocol_codec)?;
    let mut blockers = Vec::with_capacity(count);
    for _ in 0..count {
        let owner = read_node_id(reader)?;
        let target = read_optional_node_id(reader)?;
        let category = match reader.u8().map_err(protocol_codec)? {
            1 => ExpectedCategory::EntryFunction,
            2 => ExpectedCategory::FunctionBody,
            3 => ExpectedCategory::Expression,
            tag => {
                return Err(protocol_codec(
                    reader.unknown_tag(TagDomain::ProtocolMessage, tag),
                ));
            }
        };
        let expected_type = read_optional_type(reader)?;
        blockers.push(CompletenessBlocker {
            owner,
            target,
            category,
            expected_type,
        });
    }
    Ok(blockers)
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
    writer.u8(error_code_tag(error.code));
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
    let code = error_code_from_tag(tag)
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

fn error_code_tag(code: ErrorCode) -> u8 {
    match code {
        ErrorCode::ArtifactCorrupt => 1,
        ErrorCode::CompileIncomplete => 2,
        ErrorCode::CoreIrInvalid => 3,
        ErrorCode::DeleteBlocked => 4,
        ErrorCode::DuplicateHandle => 5,
        ErrorCode::DuplicateName => 6,
        ErrorCode::IdempotencyConflict => 7,
        ErrorCode::InvalidContainment => 8,
        ErrorCode::InvalidHandle => 9,
        ErrorCode::InvalidOperand => 10,
        ErrorCode::Io => 11,
        ErrorCode::NodeNotFound => 12,
        ErrorCode::NoChange => 13,
        ErrorCode::OwnerMismatch => 14,
        ErrorCode::PolicyExceeded => 15,
        ErrorCode::ProtocolMalformed => 16,
        ErrorCode::ProtocolVersion => 17,
        ErrorCode::RevisionConflict => 18,
        ErrorCode::RevisionNotFound => 19,
        ErrorCode::RuntimeTrap => 20,
        ErrorCode::TypeMismatch => 21,
        ErrorCode::WorkspaceExists => 22,
        ErrorCode::WorkspaceNotFound => 23,
        ErrorCode::WrongKind => 24,
        ErrorCode::WrongWorkspace => 25,
        ErrorCode::CommitOutcomeUnknown => 26,
    }
}

fn error_code_from_tag(tag: u8) -> Option<ErrorCode> {
    Some(match tag {
        1 => ErrorCode::ArtifactCorrupt,
        2 => ErrorCode::CompileIncomplete,
        3 => ErrorCode::CoreIrInvalid,
        4 => ErrorCode::DeleteBlocked,
        5 => ErrorCode::DuplicateHandle,
        6 => ErrorCode::DuplicateName,
        7 => ErrorCode::IdempotencyConflict,
        8 => ErrorCode::InvalidContainment,
        9 => ErrorCode::InvalidHandle,
        10 => ErrorCode::InvalidOperand,
        11 => ErrorCode::Io,
        12 => ErrorCode::NodeNotFound,
        13 => ErrorCode::NoChange,
        14 => ErrorCode::OwnerMismatch,
        15 => ErrorCode::PolicyExceeded,
        16 => ErrorCode::ProtocolMalformed,
        17 => ErrorCode::ProtocolVersion,
        18 => ErrorCode::RevisionConflict,
        19 => ErrorCode::RevisionNotFound,
        20 => ErrorCode::RuntimeTrap,
        21 => ErrorCode::TypeMismatch,
        22 => ErrorCode::WorkspaceExists,
        23 => ErrorCode::WorkspaceNotFound,
        24 => ErrorCode::WrongKind,
        25 => ErrorCode::WrongWorkspace,
        26 => ErrorCode::CommitOutcomeUnknown,
        _ => return None,
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

fn put_optional_string(writer: &mut Writer, value: Option<&str>) -> Result<()> {
    writer.bool(value.is_some());
    if let Some(value) = value {
        writer.string(value).map_err(protocol_codec)?;
    }
    Ok(())
}

fn read_optional_string(reader: &mut Reader<'_>) -> Result<Option<String>> {
    if reader.bool().map_err(protocol_codec)? {
        Ok(Some(read_protocol_string(reader)?))
    } else {
        Ok(None)
    }
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
    fn unknown_transaction_operation_rejects_before_workspace_mutation() {
        let mut body = Writer::new();
        body.u16(PROTOCOL_VERSION);
        body.u64(7);
        body.u8(2);
        body.fixed(&WorkspaceId::from_bytes([0x61; 16]).as_bytes());
        body.u64(0);
        body.bool(false);
        body.bool(false);
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
