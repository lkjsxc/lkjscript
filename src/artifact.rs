use crate::codec::{CodecError, CodecErrorKind, Reader, TagDomain, Writer};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{ArtifactVersion, NodeId, Revision, SchemaId, SnapshotHash, WorkspaceId};
use crate::schema::{Node, OperationCode, OperationKind, SemanticType, ValueRef};
use std::collections::{BTreeMap, BTreeSet};

pub const MAGIC: [u8; 8] = *b"LKJSPG\0\x03";
pub const FORMAT_VERSION: ArtifactVersion = ArtifactVersion(3);
pub const SCHEMA_ID: SchemaId = SchemaId(*b"lkjscript-spg003");
pub const MAXIMUM_ARTIFACT_BYTES: usize = 64 * 1024 * 1024;
pub const MAXIMUM_ARTIFACT_NAME_BYTES: usize = 1024 * 1024;
const ENCODED_COUNT_BYTES: usize = 8;
const ENCODED_TOMBSTONE_BYTES: usize = 8;
const MINIMUM_ENCODED_NODE_RECORD_BYTES: usize = 17;
const ENCODED_SCOPED_NODE_ID_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodePolicy {
    pub maximum_artifact_bytes: usize,
    pub maximum_name_bytes: usize,
}

impl Default for DecodePolicy {
    fn default() -> Self {
        Self {
            maximum_artifact_bytes: MAXIMUM_ARTIFACT_BYTES,
            maximum_name_bytes: MAXIMUM_ARTIFACT_NAME_BYTES,
        }
    }
}

pub fn encode(snapshot: &Snapshot) -> Result<Vec<u8>> {
    let payload = encode_payload(snapshot)?;
    let payload_length = u64::try_from(payload.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "artifact payload length does not fit canonical u64 encoding",
        )
    })?;
    let hash = SnapshotHash::from_bytes(*blake3::hash(&payload).as_bytes());
    if hash != snapshot.hash() {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "snapshot hash does not match canonical graph bytes",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision()));
    }
    let mut writer = Writer::with_capacity(
        MAGIC.len() + 2 + SCHEMA_ID.0.len() + 8 + payload.len() + SnapshotHash::BYTE_LEN,
    );
    writer.fixed(&MAGIC);
    writer.u16(FORMAT_VERSION.0);
    writer.fixed(&SCHEMA_ID.0);
    writer.u64(payload_length);
    writer.fixed(&payload);
    writer.fixed(&hash.as_bytes());
    Ok(writer.finish())
}

pub fn decode(bytes: &[u8]) -> Result<Snapshot> {
    decode_with_policy(bytes, DecodePolicy::default())
}

pub fn decode_with_policy(bytes: &[u8], policy: DecodePolicy) -> Result<Snapshot> {
    if bytes.len() > policy.maximum_artifact_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "artifact exceeds decoder byte policy",
        ));
    }
    let mut reader = Reader::new(bytes);
    if reader.fixed(MAGIC.len()).map_err(artifact_codec)? != MAGIC {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "artifact magic is invalid",
        ));
    }
    let version = reader.u16().map_err(artifact_codec)?;
    if version != FORMAT_VERSION.0 {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "artifact format version is unsupported",
        ));
    }
    if reader.fixed(SCHEMA_ID.0.len()).map_err(artifact_codec)? != SCHEMA_ID.0 {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "artifact semantic schema identity is unsupported",
        ));
    }
    let length_offset = reader.position();
    let payload_length = reader.u64().map_err(artifact_codec)?;
    let payload_length = usize::try_from(payload_length).map_err(|_| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("artifact payload length overflows host indexes at byte {length_offset}"),
        )
    })?;
    if payload_length > policy.maximum_artifact_bytes {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "artifact payload exceeds decoder byte policy",
        ));
    }
    let payload = reader.fixed(payload_length).map_err(artifact_codec)?;
    let encoded_hash = reader
        .fixed(SnapshotHash::BYTE_LEN)
        .map_err(artifact_codec)?;
    reader.finish().map_err(artifact_codec)?;

    let snapshot = decode_payload(payload, policy)?;
    let computed = SnapshotHash::from_bytes(*blake3::hash(payload).as_bytes());
    let mut expected = [0_u8; SnapshotHash::BYTE_LEN];
    expected.copy_from_slice(encoded_hash);
    let expected = SnapshotHash::from_bytes(expected);
    if expected != computed || snapshot.hash() != computed {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "artifact snapshot hash is invalid",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision()));
    }
    Ok(snapshot)
}

pub(crate) fn compute_snapshot_hash(snapshot: &Snapshot) -> Result<SnapshotHash> {
    let payload = encode_payload(snapshot)?;
    Ok(SnapshotHash::from_bytes(*blake3::hash(&payload).as_bytes()))
}

fn encode_payload(snapshot: &Snapshot) -> Result<Vec<u8>> {
    let mut writer = Writer::new();
    writer.fixed(&snapshot.workspace.as_bytes());
    writer.u64(snapshot.revision.get());
    writer.u64(snapshot.next_serial);
    writer.u64(snapshot.root.serial());
    put_count(&mut writer, snapshot.tombstones.len())?;
    for serial in &snapshot.tombstones {
        writer.u64(*serial);
    }
    put_count(&mut writer, snapshot.nodes.len())?;
    for (id, node) in &snapshot.nodes {
        if id.workspace() != snapshot.workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "cannot encode a node from another workspace",
            )
            .for_node(*id));
        }
        writer.u64(id.serial());
        put_node(&mut writer, node)?;
    }
    Ok(writer.finish())
}

fn decode_payload(payload: &[u8], policy: DecodePolicy) -> Result<Snapshot> {
    let mut reader = Reader::new(payload);
    let mut workspace = [0_u8; WorkspaceId::BYTE_LEN];
    workspace.copy_from_slice(
        reader
            .fixed(WorkspaceId::BYTE_LEN)
            .map_err(artifact_codec)?,
    );
    let workspace = WorkspaceId::from_bytes(workspace);
    let revision = Revision::new(reader.u64().map_err(artifact_codec)?);
    let next_serial = reader.u64().map_err(artifact_codec)?;
    let root = read_node_id(&mut reader, workspace)?;
    let tombstone_count =
        read_byte_bounded_count(&mut reader, ENCODED_TOMBSTONE_BYTES, ENCODED_COUNT_BYTES)?;
    let mut tombstones = BTreeSet::new();
    for _ in 0..tombstone_count {
        let serial = reader.u64().map_err(artifact_codec)?;
        if !tombstones.insert(serial) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "artifact contains a duplicate tombstone",
            ));
        }
    }
    let node_count = read_byte_bounded_count(&mut reader, MINIMUM_ENCODED_NODE_RECORD_BYTES, 0)?;
    let mut nodes = BTreeMap::new();
    for _ in 0..node_count {
        let id = read_node_id(&mut reader, workspace)?;
        let node = read_node(&mut reader, workspace, policy)?;
        if nodes.insert(id, node).is_some() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "artifact contains a duplicate node identity",
            )
            .for_node(id));
        }
    }
    reader.finish().map_err(artifact_codec)?;
    Snapshot::from_parts(workspace, revision, root, next_serial, tombstones, nodes).map_err(
        |mut error| {
            if error.code != ErrorCode::WrongWorkspace {
                error.code = ErrorCode::ArtifactCorrupt;
            }
            error
        },
    )
}

pub(crate) fn put_node(writer: &mut Writer, node: &Node) -> Result<()> {
    writer.u8(node.kind().stable_tag());
    match node {
        Node::WorkspaceRoot { packages } => put_node_ids(writer, packages),
        Node::Package {
            owner,
            name,
            modules,
            entry,
        } => {
            put_node_id(writer, *owner);
            writer.string(name).map_err(artifact_codec)?;
            put_node_ids(writer, modules)?;
            put_optional_node_id(writer, *entry);
            Ok(())
        }
        Node::Module {
            owner,
            name,
            types,
            functions,
        } => {
            put_node_id(writer, *owner);
            writer.string(name).map_err(artifact_codec)?;
            put_node_ids(writer, types)?;
            put_node_ids(writer, functions)
        }
        Node::ProductType {
            owner,
            name,
            fields,
        } => {
            put_node_id(writer, *owner);
            writer.string(name).map_err(artifact_codec)?;
            put_node_ids(writer, fields)
        }
        Node::ProductField {
            owner,
            ordinal,
            name,
            ty,
        } => {
            put_node_id(writer, *owner);
            writer.u32(*ordinal);
            writer.string(name).map_err(artifact_codec)?;
            put_type(writer, *ty)
        }
        Node::SumType {
            owner,
            name,
            variants,
        } => {
            put_node_id(writer, *owner);
            writer.string(name).map_err(artifact_codec)?;
            put_node_ids(writer, variants)
        }
        Node::SumVariant {
            owner,
            ordinal,
            name,
            payload,
        } => {
            put_node_id(writer, *owner);
            writer.u32(*ordinal);
            writer.string(name).map_err(artifact_codec)?;
            writer.bool(payload.is_some());
            if let Some(payload) = payload {
                put_type(writer, *payload)?;
            }
            Ok(())
        }
        Node::Function {
            owner,
            name,
            parameters,
            result,
            body,
        } => {
            put_node_id(writer, *owner);
            writer.string(name).map_err(artifact_codec)?;
            put_node_ids(writer, parameters)?;
            put_type(writer, *result)?;
            put_optional_node_id(writer, *body);
            Ok(())
        }
        Node::Parameter {
            owner,
            ordinal,
            name,
            ty,
        } => {
            put_node_id(writer, *owner);
            writer.u32(*ordinal);
            writer.string(name).map_err(artifact_codec)?;
            put_type(writer, *ty)
        }
        Node::Region { owner, blocks } => {
            put_node_id(writer, *owner);
            put_node_ids(writer, blocks)
        }
        Node::Block {
            owner,
            arguments,
            operations,
            terminator,
        } => {
            put_node_id(writer, *owner);
            put_node_ids(writer, arguments)?;
            put_node_ids(writer, operations)?;
            put_optional_node_id(writer, *terminator);
            Ok(())
        }
        Node::BlockArgument { owner, ordinal, ty } => {
            put_node_id(writer, *owner);
            writer.u32(*ordinal);
            put_type(writer, *ty)
        }
        Node::Operation { owner, operation } => {
            put_node_id(writer, *owner);
            put_operation(writer, operation)
        }
    }
}

pub(crate) fn read_node(
    reader: &mut Reader<'_>,
    workspace: WorkspaceId,
    policy: DecodePolicy,
) -> Result<Node> {
    let tag = reader.u8().map_err(artifact_codec)?;
    let kind = crate::schema::NodeKind::from_stable_tag(tag)
        .ok_or_else(|| artifact_codec(reader.unknown_tag(TagDomain::Node, tag)))?;
    Ok(match kind {
        crate::schema::NodeKind::WorkspaceRoot => Node::WorkspaceRoot {
            packages: read_node_ids(reader, workspace)?,
        },
        crate::schema::NodeKind::Package => Node::Package {
            owner: read_node_id(reader, workspace)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            modules: read_node_ids(reader, workspace)?,
            entry: read_optional_node_id(reader, workspace)?,
        },
        crate::schema::NodeKind::Module => Node::Module {
            owner: read_node_id(reader, workspace)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            types: read_node_ids(reader, workspace)?,
            functions: read_node_ids(reader, workspace)?,
        },
        crate::schema::NodeKind::ProductType => Node::ProductType {
            owner: read_node_id(reader, workspace)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            fields: read_node_ids(reader, workspace)?,
        },
        crate::schema::NodeKind::ProductField => Node::ProductField {
            owner: read_node_id(reader, workspace)?,
            ordinal: reader.u32().map_err(artifact_codec)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            ty: read_type(reader, workspace)?,
        },
        crate::schema::NodeKind::SumType => Node::SumType {
            owner: read_node_id(reader, workspace)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            variants: read_node_ids(reader, workspace)?,
        },
        crate::schema::NodeKind::SumVariant => Node::SumVariant {
            owner: read_node_id(reader, workspace)?,
            ordinal: reader.u32().map_err(artifact_codec)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            payload: if reader.bool().map_err(artifact_codec)? {
                Some(read_type(reader, workspace)?)
            } else {
                None
            },
        },
        crate::schema::NodeKind::Function => Node::Function {
            owner: read_node_id(reader, workspace)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            parameters: read_node_ids(reader, workspace)?,
            result: read_type(reader, workspace)?,
            body: read_optional_node_id(reader, workspace)?,
        },
        crate::schema::NodeKind::Parameter => Node::Parameter {
            owner: read_node_id(reader, workspace)?,
            ordinal: reader.u32().map_err(artifact_codec)?,
            name: reader
                .string(policy.maximum_name_bytes)
                .map_err(artifact_codec)?,
            ty: read_type(reader, workspace)?,
        },
        crate::schema::NodeKind::Region => Node::Region {
            owner: read_node_id(reader, workspace)?,
            blocks: read_node_ids(reader, workspace)?,
        },
        crate::schema::NodeKind::Block => Node::Block {
            owner: read_node_id(reader, workspace)?,
            arguments: read_node_ids(reader, workspace)?,
            operations: read_node_ids(reader, workspace)?,
            terminator: read_optional_node_id(reader, workspace)?,
        },
        crate::schema::NodeKind::BlockArgument => Node::BlockArgument {
            owner: read_node_id(reader, workspace)?,
            ordinal: reader.u32().map_err(artifact_codec)?,
            ty: read_type(reader, workspace)?,
        },
        crate::schema::NodeKind::Operation => Node::Operation {
            owner: read_node_id(reader, workspace)?,
            operation: read_operation(reader, workspace)?,
        },
    })
}

pub(crate) fn put_operation(writer: &mut Writer, operation: &OperationKind) -> Result<()> {
    writer.u8(operation.stable_tag());
    match operation {
        OperationKind::ConstUnit => {}
        OperationKind::ConstI64(value) => writer.i64(*value),
        OperationKind::ConstBool(value) => writer.bool(*value),
        OperationKind::AddI64 { lhs, rhs } | OperationKind::LtI64 { lhs, rhs } => {
            put_value(writer, *lhs);
            put_value(writer, *rhs);
        }
        OperationKind::Call {
            function,
            arguments,
        } => {
            put_node_id(writer, *function);
            put_count(writer, arguments.len())?;
            for argument in arguments {
                put_value(writer, *argument);
            }
        }
        OperationKind::Hole { expected } => put_type(writer, *expected)?,
        OperationKind::If {
            condition,
            result,
            then_region,
            else_region,
        } => {
            put_value(writer, *condition);
            put_type(writer, *result)?;
            put_node_id(writer, *then_region);
            put_node_id(writer, *else_region);
        }
        OperationKind::ForI64 {
            start,
            end_exclusive,
            step,
            initial,
            carried,
            body_region,
        } => {
            put_value(writer, *start);
            put_value(writer, *end_exclusive);
            writer.i64(*step);
            put_value(writer, *initial);
            put_type(writer, *carried)?;
            put_node_id(writer, *body_region);
        }
        OperationKind::Return { value } | OperationKind::Yield { value } => {
            put_value(writer, *value)
        }
        OperationKind::ConstructProduct { product, fields } => {
            put_node_id(writer, *product);
            put_count(writer, fields.len())?;
            for field in fields {
                put_node_id(writer, field.field);
                put_value(writer, field.value);
            }
        }
        OperationKind::ProjectField { value, field } => {
            put_value(writer, *value);
            put_node_id(writer, *field);
        }
        OperationKind::ConstructVariant { variant, payload } => {
            put_node_id(writer, *variant);
            writer.bool(payload.is_some());
            if let Some(payload) = payload {
                put_value(writer, *payload);
            }
        }
        OperationKind::MatchSum {
            scrutinee,
            result,
            arms,
        } => {
            put_value(writer, *scrutinee);
            put_type(writer, *result)?;
            put_count(writer, arms.len())?;
            for arm in arms {
                put_node_id(writer, arm.variant);
                put_node_id(writer, arm.region);
            }
        }
    }
    Ok(())
}

pub(crate) fn read_operation(
    reader: &mut Reader<'_>,
    workspace: WorkspaceId,
) -> Result<OperationKind> {
    let tag = reader.u8().map_err(artifact_codec)?;
    let code = OperationCode::from_stable_tag(tag)
        .ok_or_else(|| artifact_codec(reader.unknown_tag(TagDomain::Operation, tag)))?;
    match code {
        OperationCode::ConstUnit => Ok(OperationKind::ConstUnit),
        OperationCode::ConstI64 => Ok(OperationKind::ConstI64(
            reader.i64().map_err(artifact_codec)?,
        )),
        OperationCode::ConstBool => Ok(OperationKind::ConstBool(
            reader.bool().map_err(artifact_codec)?,
        )),
        OperationCode::AddI64 => Ok(OperationKind::AddI64 {
            lhs: read_value(reader, workspace)?,
            rhs: read_value(reader, workspace)?,
        }),
        OperationCode::LtI64 => Ok(OperationKind::LtI64 {
            lhs: read_value(reader, workspace)?,
            rhs: read_value(reader, workspace)?,
        }),
        OperationCode::Call => {
            let function = read_node_id(reader, workspace)?;
            let count = read_byte_bounded_count(reader, 9, 0)?;
            let mut arguments = Vec::with_capacity(count);
            for _ in 0..count {
                arguments.push(read_value(reader, workspace)?);
            }
            Ok(OperationKind::Call {
                function,
                arguments,
            })
        }
        OperationCode::Hole => Ok(OperationKind::Hole {
            expected: read_type(reader, workspace)?,
        }),
        OperationCode::If => Ok(OperationKind::If {
            condition: read_value(reader, workspace)?,
            result: read_type(reader, workspace)?,
            then_region: read_node_id(reader, workspace)?,
            else_region: read_node_id(reader, workspace)?,
        }),
        OperationCode::ForI64 => Ok(OperationKind::ForI64 {
            start: read_value(reader, workspace)?,
            end_exclusive: read_value(reader, workspace)?,
            step: reader.i64().map_err(artifact_codec)?,
            initial: read_value(reader, workspace)?,
            carried: read_type(reader, workspace)?,
            body_region: read_node_id(reader, workspace)?,
        }),
        OperationCode::Return => Ok(OperationKind::Return {
            value: read_value(reader, workspace)?,
        }),
        OperationCode::Yield => Ok(OperationKind::Yield {
            value: read_value(reader, workspace)?,
        }),
        OperationCode::ConstructProduct => {
            let product = read_node_id(reader, workspace)?;
            let count = read_byte_bounded_count(reader, 17, 0)?;
            let mut fields = Vec::with_capacity(count);
            for _ in 0..count {
                fields.push(crate::schema::ProductFieldValue {
                    field: read_node_id(reader, workspace)?,
                    value: read_value(reader, workspace)?,
                });
            }
            Ok(OperationKind::ConstructProduct { product, fields })
        }
        OperationCode::ProjectField => Ok(OperationKind::ProjectField {
            value: read_value(reader, workspace)?,
            field: read_node_id(reader, workspace)?,
        }),
        OperationCode::ConstructVariant => {
            let variant = read_node_id(reader, workspace)?;
            let payload = if reader.bool().map_err(artifact_codec)? {
                Some(read_value(reader, workspace)?)
            } else {
                None
            };
            Ok(OperationKind::ConstructVariant { variant, payload })
        }
        OperationCode::MatchSum => {
            let scrutinee = read_value(reader, workspace)?;
            let result = read_type(reader, workspace)?;
            let count = read_byte_bounded_count(reader, 16, 0)?;
            let mut arms = Vec::with_capacity(count);
            for _ in 0..count {
                arms.push(crate::schema::MatchArm {
                    variant: read_node_id(reader, workspace)?,
                    region: read_node_id(reader, workspace)?,
                });
            }
            Ok(OperationKind::MatchSum {
                scrutinee,
                result,
                arms,
            })
        }
    }
}

pub(crate) fn put_value(writer: &mut Writer, value: ValueRef) {
    match value {
        ValueRef::FunctionParameter(parameter) => {
            writer.u8(1);
            put_node_id(writer, parameter);
        }
        ValueRef::OperationResult { operation, output } => {
            writer.u8(2);
            put_node_id(writer, operation);
            writer.u8(output);
        }
        ValueRef::BlockArgument(argument) => {
            writer.u8(3);
            put_node_id(writer, argument);
        }
    }
}

pub(crate) fn read_value(reader: &mut Reader<'_>, workspace: WorkspaceId) -> Result<ValueRef> {
    let tag = reader.u8().map_err(artifact_codec)?;
    match tag {
        1 => Ok(ValueRef::FunctionParameter(read_node_id(
            reader, workspace,
        )?)),
        2 => Ok(ValueRef::OperationResult {
            operation: read_node_id(reader, workspace)?,
            output: reader.u8().map_err(artifact_codec)?,
        }),
        3 => Ok(ValueRef::BlockArgument(read_node_id(reader, workspace)?)),
        _ => Err(artifact_codec(reader.unknown_tag(TagDomain::Value, tag))),
    }
}

fn put_type(writer: &mut Writer, ty: SemanticType) -> Result<()> {
    writer.u8(ty.stable_tag());
    if let SemanticType::Nominal(target) = ty {
        put_node_id(writer, target);
    }
    Ok(())
}

fn read_type(reader: &mut Reader<'_>, workspace: WorkspaceId) -> Result<SemanticType> {
    let tag = reader.u8().map_err(artifact_codec)?;
    if tag == 4 {
        return Ok(SemanticType::Nominal(read_node_id(reader, workspace)?));
    }
    SemanticType::from_stable_tag(tag)
        .ok_or_else(|| artifact_codec(reader.unknown_tag(TagDomain::SemanticType, tag)))
}

fn put_count(writer: &mut Writer, count: usize) -> Result<()> {
    let count = u64::try_from(count).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "collection length does not fit canonical u64 encoding",
        )
    })?;
    writer.u64(count);
    Ok(())
}

fn put_node_ids(writer: &mut Writer, values: &[NodeId]) -> Result<()> {
    put_count(writer, values.len())?;
    for value in values {
        put_node_id(writer, *value);
    }
    Ok(())
}

fn read_byte_bounded_count(
    reader: &mut Reader<'_>,
    minimum_item_bytes: usize,
    reserved_trailing_bytes: usize,
) -> Result<usize> {
    let maximum = reader
        .remaining()
        .saturating_sub(ENCODED_COUNT_BYTES)
        .saturating_sub(reserved_trailing_bytes)
        / minimum_item_bytes;
    reader.count(maximum).map_err(artifact_codec)
}

fn read_node_ids(reader: &mut Reader<'_>, workspace: WorkspaceId) -> Result<Vec<NodeId>> {
    let count = read_byte_bounded_count(reader, ENCODED_SCOPED_NODE_ID_BYTES, 0)?;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(read_node_id(reader, workspace)?);
    }
    Ok(values)
}

fn put_node_id(writer: &mut Writer, value: NodeId) {
    writer.u64(value.serial());
}

fn read_node_id(reader: &mut Reader<'_>, workspace: WorkspaceId) -> Result<NodeId> {
    let serial = reader.u64().map_err(artifact_codec)?;
    NodeId::new(workspace, serial).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("artifact contains an invalid node identity: {error}"),
        )
    })
}

fn put_optional_node_id(writer: &mut Writer, value: Option<NodeId>) {
    writer.bool(value.is_some());
    if let Some(value) = value {
        put_node_id(writer, value);
    }
}

fn read_optional_node_id(
    reader: &mut Reader<'_>,
    workspace: WorkspaceId,
) -> Result<Option<NodeId>> {
    if reader.bool().map_err(artifact_codec)? {
        Ok(Some(read_node_id(reader, workspace)?))
    } else {
        Ok(None)
    }
}

fn artifact_codec(error: CodecError) -> LkError {
    let code = if error.kind == CodecErrorKind::PolicyExceeded {
        ErrorCode::PolicyExceeded
    } else {
        ErrorCode::ArtifactCorrupt
    };
    LkError::new(code, format!("canonical artifact decoding failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn initial() -> Snapshot {
        Snapshot::initial(WorkspaceId::from_bytes([9; 16])).expect("initial snapshot must be valid")
    }

    fn nominal_snapshot() -> Snapshot {
        let workspace = WorkspaceId::from_bytes([0x33; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                },
            ),
            (
                id(2),
                Node::Package {
                    owner: id(1),
                    name: "p".into(),
                    modules: vec![id(3)],
                    entry: None,
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "m".into(),
                    types: vec![id(4), id(6)],
                    functions: Vec::new(),
                },
            ),
            (
                id(4),
                Node::ProductType {
                    owner: id(3),
                    name: "Reading".into(),
                    fields: vec![id(5)],
                },
            ),
            (
                id(5),
                Node::ProductField {
                    owner: id(4),
                    ordinal: 0,
                    name: "value".into(),
                    ty: SemanticType::I64,
                },
            ),
            (
                id(6),
                Node::SumType {
                    owner: id(3),
                    name: "Input".into(),
                    variants: vec![id(7)],
                },
            ),
            (
                id(7),
                Node::SumVariant {
                    owner: id(6),
                    ordinal: 0,
                    name: "sample".into(),
                    payload: Some(SemanticType::Nominal(id(4))),
                },
            ),
        ]);
        Snapshot::from_parts(
            workspace,
            Revision::INITIAL,
            id(1),
            8,
            BTreeSet::new(),
            nodes,
        )
        .expect("nominal snapshot")
    }

    #[test]
    fn nominal_nodes_and_type_references_round_trip_without_layout_bytes() {
        let snapshot = nominal_snapshot();
        let bytes = encode(&snapshot).expect("encode nominal");
        let decoded = decode(&bytes).expect("decode nominal");
        assert_eq!(decoded, snapshot);
        assert_eq!(encode(&decoded).expect("reencode"), bytes);
        assert!(
            !bytes
                .windows(b"payload_offset".len())
                .any(|window| window == b"payload_offset")
        );
    }

    #[test]
    fn artifact_round_trip_is_byte_identical() {
        let snapshot = initial();
        let first = encode(&snapshot).expect("encode initial snapshot");
        let decoded = decode(&first).expect("decode initial snapshot");
        let second = encode(&decoded).expect("re-encode initial snapshot");
        assert_eq!(first, second);
        assert_eq!(snapshot, decoded);
    }

    #[test]
    fn operation_artifact_tags_round_trip_every_closed_code() {
        let workspace = WorkspaceId::from_bytes([0x5a; 16]);
        let first = NodeId::new(workspace, 2).expect("first operation");
        let second = NodeId::new(workspace, 3).expect("second operation");
        let operations = vec![
            OperationKind::ConstUnit,
            OperationKind::ConstI64(-7),
            OperationKind::ConstBool(true),
            OperationKind::AddI64 {
                lhs: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                rhs: ValueRef::OperationResult {
                    operation: second,
                    output: 0,
                },
            },
            OperationKind::LtI64 {
                lhs: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                rhs: ValueRef::OperationResult {
                    operation: second,
                    output: 0,
                },
            },
            OperationKind::Call {
                function: first,
                arguments: vec![ValueRef::BlockArgument(second)],
            },
            OperationKind::Hole {
                expected: SemanticType::Bool,
            },
            OperationKind::If {
                condition: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                result: SemanticType::I64,
                then_region: first,
                else_region: second,
            },
            OperationKind::ForI64 {
                start: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                end_exclusive: ValueRef::OperationResult {
                    operation: second,
                    output: 0,
                },
                step: 1,
                initial: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
                carried: SemanticType::I64,
                body_region: second,
            },
            OperationKind::Return {
                value: ValueRef::OperationResult {
                    operation: first,
                    output: 0,
                },
            },
            OperationKind::Yield {
                value: ValueRef::BlockArgument(second),
            },
            OperationKind::ConstructProduct {
                product: first,
                fields: vec![crate::schema::ProductFieldValue {
                    field: second,
                    value: ValueRef::BlockArgument(second),
                }],
            },
            OperationKind::ProjectField {
                value: ValueRef::BlockArgument(second),
                field: first,
            },
            OperationKind::ConstructVariant {
                variant: first,
                payload: Some(ValueRef::BlockArgument(second)),
            },
            OperationKind::MatchSum {
                scrutinee: ValueRef::BlockArgument(second),
                result: SemanticType::Nominal(first),
                arms: vec![crate::schema::MatchArm {
                    variant: first,
                    region: second,
                }],
            },
        ];
        assert_eq!(operations.len(), OperationCode::ALL.len());
        for operation in operations {
            let mut writer = Writer::new();
            put_operation(&mut writer, &operation).expect("operation encode");
            let bytes = writer.finish();
            let mut reader = Reader::new(&bytes);
            assert_eq!(
                read_operation(&mut reader, workspace).expect("operation round trip"),
                operation
            );
            reader.finish().expect("complete operation payload");
        }
    }

    #[test]
    fn artifact_format_two_rejects_without_compatibility_reader() {
        let mut bytes = encode(&initial()).expect("format three artifact");
        bytes[..MAGIC.len()].copy_from_slice(b"LKJSPG\0\x02");
        bytes[MAGIC.len()..MAGIC.len() + 2].copy_from_slice(&2_u16.to_le_bytes());
        assert_eq!(
            decode(&bytes).expect_err("format two must reject").code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn corrupt_truncated_and_trailing_artifacts_reject() {
        let bytes = encode(&initial()).expect("encode initial snapshot");
        let mut corrupt = bytes.clone();
        if let Some(last) = corrupt.last_mut() {
            *last ^= 1;
        }
        assert_eq!(
            decode(&corrupt).err().map(|error| error.code),
            Some(ErrorCode::ArtifactCorrupt)
        );
        assert_eq!(
            decode(&bytes[..bytes.len() - 1])
                .err()
                .map(|error| error.code),
            Some(ErrorCode::ArtifactCorrupt)
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            decode(&trailing).err().map(|error| error.code),
            Some(ErrorCode::ArtifactCorrupt)
        );
    }

    #[test]
    fn unknown_tags_duplicate_ids_invalid_roots_and_policy_limits_reject() {
        let encoded = encode(&initial()).expect("encode initial snapshot");
        let payload = payload(&encoded);

        let mut unknown = payload.to_vec();
        unknown[64] = 0xff;
        let unknown = artifact_from_payload(&unknown);
        let error = decode(&unknown).expect_err("unknown node tag");
        assert_eq!(error.code, ErrorCode::ArtifactCorrupt);
        assert!(error.message.contains("UnknownTag"));

        let mut duplicate = payload.to_vec();
        duplicate[48..56].copy_from_slice(&2_u64.to_le_bytes());
        duplicate.extend_from_slice(&payload[56..]);
        let duplicate = artifact_from_payload(&duplicate);
        let error = decode(&duplicate).expect_err("duplicate node identity");
        assert_eq!(error.code, ErrorCode::ArtifactCorrupt);
        assert!(error.message.contains("duplicate node identity"));

        let mut invalid_root = payload.to_vec();
        invalid_root[32..40].copy_from_slice(&2_u64.to_le_bytes());
        let invalid_root = artifact_from_payload(&invalid_root);
        assert_eq!(
            decode(&invalid_root).expect_err("invalid root").code,
            ErrorCode::ArtifactCorrupt
        );

        let restrictive = DecodePolicy {
            maximum_artifact_bytes: encoded.len() - 1,
            ..DecodePolicy::default()
        };
        assert_eq!(
            decode_with_policy(&encoded, restrictive)
                .expect_err("artifact byte policy")
                .code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn inflated_counts_reject_from_remaining_bytes_without_semantic_count_limits() {
        let encoded = encode(&initial()).expect("encode initial snapshot");
        let original = payload(&encoded);

        let mut tombstones = original.to_vec();
        tombstones[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            decode(&artifact_from_payload(&tombstones))
                .expect_err("inflated tombstone count")
                .code,
            ErrorCode::PolicyExceeded
        );

        let mut nodes = original.to_vec();
        nodes[48..56].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            decode(&artifact_from_payload(&nodes))
                .expect_err("inflated node count")
                .code,
            ErrorCode::PolicyExceeded
        );

        let mut child_ids = original.to_vec();
        child_ids[65..73].copy_from_slice(&u64::MAX.to_le_bytes());
        assert_eq!(
            decode(&artifact_from_payload(&child_ids))
                .expect_err("inflated child-list count")
                .code,
            ErrorCode::PolicyExceeded
        );
    }

    fn payload(encoded: &[u8]) -> &[u8] {
        let mut length = [0_u8; 8];
        length.copy_from_slice(&encoded[26..34]);
        let length = usize::try_from(u64::from_le_bytes(length)).expect("payload length");
        &encoded[34..34 + length]
    }

    fn artifact_from_payload(payload: &[u8]) -> Vec<u8> {
        let mut writer = Writer::new();
        writer.fixed(&MAGIC);
        writer.u16(FORMAT_VERSION.0);
        writer.fixed(&SCHEMA_ID.0);
        writer.u64(u64::try_from(payload.len()).expect("payload length"));
        writer.fixed(payload);
        writer.fixed(blake3::hash(payload).as_bytes());
        writer.finish()
    }
}
