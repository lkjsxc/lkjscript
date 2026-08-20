use super::{ContextPacket, ContextPacketDigest, MAX_WORKBENCH_INPUT_BYTES};
use crate::ids::{DraftSymbol, IdempotencyKey, NodeId, Revision, WorkspaceId};
use crate::interpret::{RunPolicy, RuntimeValue};
use crate::machine::{MachineSchemaDigest, active_machine_schema_digest};
use crate::schema::{
    MatchArm, Node, OperationKind, ProductFieldValue, ProductFieldValueDraft, TypeDraft, ValueRef,
};
use crate::transaction::{
    ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
    MatchArmDraft, NodeTarget, Transaction, TransactionMode, TransactionOp,
    TransactionResponseSpec, YieldingBodyDraft,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Number, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

const MAX_DOCUMENT_DEPTH: usize = 32;
const MAX_DOCUMENT_ITEMS: usize = 65_536;
const MAX_DOCUMENT_ERROR_BYTES: usize = 512;
pub const EDIT_DOCUMENT_VERSION: u16 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentErrorCode {
    InvalidUtf8,
    InputTooLarge,
    Syntax,
    DepthExceeded,
    ItemLimitExceeded,
    DuplicateField,
    UnknownAlias,
    PacketRequired,
    PacketMismatch,
    UnsupportedVersion,
    StaleSchema,
    ScopeMismatch,
    Shape,
}

impl DocumentErrorCode {
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::InvalidUtf8 => "invalid_utf8",
            Self::InputTooLarge => "input_too_large",
            Self::Syntax => "syntax",
            Self::DepthExceeded => "depth_exceeded",
            Self::ItemLimitExceeded => "item_limit_exceeded",
            Self::DuplicateField => "duplicate_field",
            Self::UnknownAlias => "unknown_alias",
            Self::PacketRequired => "packet_required",
            Self::PacketMismatch => "packet_mismatch",
            Self::UnsupportedVersion => "unsupported_version",
            Self::StaleSchema => "stale_schema",
            Self::ScopeMismatch => "scope_mismatch",
            Self::Shape => "shape",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentError {
    pub code: DocumentErrorCode,
    pub line: u32,
    pub column: u32,
    pub byte_offset: u64,
    pub message: String,
}

impl DocumentError {
    fn at(code: DocumentErrorCode, location: Location, message: impl Into<String>) -> Self {
        let mut message = message.into();
        if message.len() > MAX_DOCUMENT_ERROR_BYTES {
            let mut end = MAX_DOCUMENT_ERROR_BYTES;
            while !message.is_char_boundary(end) {
                end -= 1;
            }
            message.truncate(end);
        }
        Self {
            code,
            line: location.line,
            column: location.column,
            byte_offset: u64::try_from(location.offset).unwrap_or(u64::MAX),
            message,
        }
    }

    fn shape(message: impl Into<String>) -> Self {
        Self::at(DocumentErrorCode::Shape, Location::start(), message)
    }
}

impl fmt::Display for DocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} at line {}, column {}",
            self.message, self.line, self.column
        )
    }
}

impl std::error::Error for DocumentError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedEditDocument {
    pub packet: Option<ContextPacketDigest>,
    pub request: ApplyTransactionRequest,
    pub alias_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedRunDocument {
    pub packet: Option<ContextPacketDigest>,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub entry: NodeId,
    pub arguments: Vec<RuntimeValue>,
    pub policy: RunPolicy,
    pub alias_count: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct EditDocument {
    version: u16,
    schema: MachineSchemaDigest,
    #[serde(default)]
    packet: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    base_revision: Revision,
    scope: EditScope,
    #[serde(default)]
    idempotency_key: Option<IdempotencyKey>,
    edits: Vec<TransactionOp>,
    #[serde(default)]
    return_symbols: Vec<DraftSymbol>,
}

pub fn render_function_document(
    snapshot: &crate::graph::Snapshot,
    function: NodeId,
) -> crate::Result<Vec<u8>> {
    if !function.is_durable() {
        return Err(document_render_error(
            "function document target must be a durable entity",
            Some(function),
        ));
    }
    if function.workspace() != snapshot.workspace() {
        return Err(crate::LkError::new(
            crate::ErrorCode::WrongWorkspace,
            "function document target belongs to another workspace",
        )
        .for_node(function));
    }
    let mut nodes = BTreeMap::new();
    let mut stack = vec![function];
    while let Some(node_id) = stack.pop() {
        if nodes.contains_key(&node_id) {
            return Err(crate::LkError::new(
                crate::ErrorCode::ArtifactCorrupt,
                "function document ownership tree contains a duplicate or cycle",
            )
            .for_node(node_id));
        }
        if nodes.len() >= crate::workbench::MAXIMUM_SEMANTIC_QUERY_WORK_ITEMS {
            return Err(crate::LkError::new(
                crate::ErrorCode::PolicyExceeded,
                "function document scope exceeds the semantic query work policy",
            )
            .for_node(function));
        }
        let node = snapshot.node(node_id)?;
        nodes.insert(node_id, node);
        for index in (0..node.owned_child_count()).rev() {
            stack.push(node.owned_child(index).ok_or_else(|| {
                crate::LkError::new(
                    crate::ErrorCode::ArtifactCorrupt,
                    "function document ownership tree omitted a counted child",
                )
                .for_node(node_id)
            })?);
        }
    }
    let body = match nodes.get(&function).copied() {
        Some(Node::Function {
            body: Some(body), ..
        }) => *body,
        Some(Node::Function { body: None, .. }) => {
            return Err(document_render_error(
                "function document target has no body to replace",
                Some(function),
            ));
        }
        Some(_) => {
            return Err(document_render_error(
                "function document target has the wrong semantic kind",
                Some(function),
            ));
        }
        None => {
            return Err(document_render_error(
                "function document target is absent from its context packet",
                Some(function),
            ));
        }
    };
    let body = render_function_body_draft(&nodes, body)?;
    let document = EditDocument {
        version: EDIT_DOCUMENT_VERSION,
        schema: active_machine_schema_digest()?,
        packet: None,
        workspace: snapshot.workspace(),
        base_revision: snapshot.revision(),
        scope: EditScope::Function(function),
        idempotency_key: None,
        edits: vec![TransactionOp::ReplaceFunctionBody { function, body }],
        return_symbols: Vec::new(),
    };
    let value = serde_json::to_value(document).map_err(|error| {
        document_render_error(
            format!("cannot project the editable function document: {error}"),
            Some(function),
        )
    })?;
    let mut output = b"document ".to_vec();
    render_document_value(&value, 0, &mut output)?;
    output.push(b'\n');
    if output.len() > MAX_WORKBENCH_INPUT_BYTES {
        return Err(crate::LkError::new(
            crate::ErrorCode::PolicyExceeded,
            "editable function document exceeds the input byte policy",
        )
        .for_node(function));
    }
    Ok(output)
}

fn render_function_body_draft(
    nodes: &BTreeMap<NodeId, &Node>,
    region: NodeId,
) -> crate::Result<FunctionBodyDraft> {
    let (operations, terminator) = render_region(nodes, region, 0)?;
    let value = match terminator {
        OperationKind::Return { value } => render_value_draft(nodes, value)?,
        _ => {
            return Err(document_render_error(
                "function body region does not end in return",
                Some(region),
            ));
        }
    };
    Ok(FunctionBodyDraft {
        operations,
        return_value: value,
    })
}

fn render_yielding_body_draft(
    nodes: &BTreeMap<NodeId, &Node>,
    region: NodeId,
    depth: usize,
) -> crate::Result<YieldingBodyDraft> {
    let (operations, terminator) = render_region(nodes, region, depth)?;
    let value = match terminator {
        OperationKind::Yield { value } => render_value_draft(nodes, value)?,
        _ => {
            return Err(document_render_error(
                "structured body region does not end in yield",
                Some(region),
            ));
        }
    };
    Ok(YieldingBodyDraft {
        operations,
        yield_value: value,
    })
}

fn render_region(
    nodes: &BTreeMap<NodeId, &Node>,
    region: NodeId,
    depth: usize,
) -> crate::Result<(Vec<ExpressionDraft>, OperationKind)> {
    if depth > crate::transaction::MAX_STRUCTURED_DRAFT_DEPTH {
        return Err(document_render_error(
            "function body exceeds the editable document depth policy",
            Some(region),
        ));
    }
    let block = match nodes.get(&region).copied() {
        Some(Node::Region { blocks, .. }) if blocks.len() == 1 => blocks[0],
        Some(Node::Region { .. }) => {
            return Err(document_render_error(
                "editable documents require one canonical block per region",
                Some(region),
            ));
        }
        _ => {
            return Err(document_render_error(
                "function document is missing a region from its packet",
                Some(region),
            ));
        }
    };
    let (operation_ids, terminator) = match nodes.get(&block).copied() {
        Some(Node::Block {
            operations,
            terminator: Some(terminator),
            ..
        }) => (operations, *terminator),
        Some(Node::Block {
            terminator: None, ..
        }) => {
            return Err(document_render_error(
                "editable function body is incomplete at a block terminator",
                Some(block),
            ));
        }
        _ => {
            return Err(document_render_error(
                "function document is missing a block from its packet",
                Some(block),
            ));
        }
    };
    let mut operations = Vec::with_capacity(operation_ids.len());
    for operation in operation_ids {
        if operation.is_durable() {
            return Err(document_render_error(
                "whole-body replacement cannot preserve a durable repair anchor; refine the anchor instead",
                Some(*operation),
            ));
        }
        let kind = match nodes.get(operation).copied() {
            Some(Node::Operation { operation, .. }) => operation,
            _ => {
                return Err(document_render_error(
                    "function document is missing an operation from its packet",
                    Some(*operation),
                ));
            }
        };
        operations.push(ExpressionDraft {
            symbol: Some(local_symbol(*operation)?),
            operation: render_expression_kind(nodes, kind, depth)?,
        });
    }
    let terminator = match nodes.get(&terminator).copied() {
        Some(Node::Operation { operation, .. }) => operation.clone(),
        _ => {
            return Err(document_render_error(
                "function document is missing a terminator from its packet",
                Some(terminator),
            ));
        }
    };
    Ok((operations, terminator))
}

fn render_expression_kind(
    nodes: &BTreeMap<NodeId, &Node>,
    operation: &OperationKind,
    depth: usize,
) -> crate::Result<ExpressionKindDraft> {
    let value = |value| render_value_draft(nodes, value);
    Ok(match operation {
        OperationKind::ConstUnit => ExpressionKindDraft::ConstUnit,
        OperationKind::ConstI64(value) => ExpressionKindDraft::ConstI64(*value),
        OperationKind::ConstBool(value) => ExpressionKindDraft::ConstBool(*value),
        OperationKind::ConstBytes(value) => ExpressionKindDraft::ConstBytes(value.clone()),
        OperationKind::ConstText(value) => ExpressionKindDraft::ConstText(value.clone()),
        OperationKind::AddI64 { lhs, rhs } => ExpressionKindDraft::AddI64 {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::LtI64 { lhs, rhs } => ExpressionKindDraft::LtI64 {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::EqualI64 { lhs, rhs } => ExpressionKindDraft::EqualI64 {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::NotBool { value: operand } => ExpressionKindDraft::NotBool {
            value: value(*operand)?,
        },
        OperationKind::AndBool { lhs, rhs } => ExpressionKindDraft::AndBool {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::OrBool { lhs, rhs } => ExpressionKindDraft::OrBool {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::BytesLen { value: operand } => ExpressionKindDraft::BytesLen {
            value: value(*operand)?,
        },
        OperationKind::BytesAt {
            value: operand,
            index,
        } => ExpressionKindDraft::BytesAt {
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::BytesSlice {
            value: operand,
            start,
            length,
        } => ExpressionKindDraft::BytesSlice {
            value: value(*operand)?,
            start: value(*start)?,
            length: value(*length)?,
        },
        OperationKind::BytesEqual { lhs, rhs } => ExpressionKindDraft::BytesEqual {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::BytesConcat { lhs, rhs } => ExpressionKindDraft::BytesConcat {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::TextLen { value: operand } => ExpressionKindDraft::TextLen {
            value: value(*operand)?,
        },
        OperationKind::TextEqual { lhs, rhs } => ExpressionKindDraft::TextEqual {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::TextConcat { lhs, rhs } => ExpressionKindDraft::TextConcat {
            lhs: value(*lhs)?,
            rhs: value(*rhs)?,
        },
        OperationKind::TextScalarLen { value: operand } => ExpressionKindDraft::TextScalarLen {
            value: value(*operand)?,
        },
        OperationKind::TextGraphemeLen { value: operand } => ExpressionKindDraft::TextGraphemeLen {
            value: value(*operand)?,
        },
        OperationKind::TextLineCount { value: operand } => ExpressionKindDraft::TextLineCount {
            value: value(*operand)?,
        },
        OperationKind::TextScalarAt {
            value: operand,
            index,
        } => ExpressionKindDraft::TextScalarAt {
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::TextPreviousGraphemeBoundary {
            value: operand,
            index,
        } => ExpressionKindDraft::TextPreviousGraphemeBoundary {
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::TextNextGraphemeBoundary {
            value: operand,
            index,
        } => ExpressionKindDraft::TextNextGraphemeBoundary {
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::TextLineStart {
            value: operand,
            line,
        } => ExpressionKindDraft::TextLineStart {
            value: value(*operand)?,
            line: value(*line)?,
        },
        OperationKind::TextLineEnd {
            value: operand,
            line,
        } => ExpressionKindDraft::TextLineEnd {
            value: value(*operand)?,
            line: value(*line)?,
        },
        OperationKind::TextByteToLine {
            value: operand,
            index,
        } => ExpressionKindDraft::TextByteToLine {
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::TextSlice {
            value: operand,
            start,
            end_exclusive,
        } => ExpressionKindDraft::TextSlice {
            value: value(*operand)?,
            start: value(*start)?,
            end_exclusive: value(*end_exclusive)?,
        },
        OperationKind::TextSplice {
            value: operand,
            start,
            end_exclusive,
            replacement,
        } => ExpressionKindDraft::TextSplice {
            value: value(*operand)?,
            start: value(*start)?,
            end_exclusive: value(*end_exclusive)?,
            replacement: value(*replacement)?,
        },
        OperationKind::TextFindForward {
            value: operand,
            query,
            start,
        } => ExpressionKindDraft::TextFindForward {
            value: value(*operand)?,
            query: value(*query)?,
            start: value(*start)?,
        },
        OperationKind::TextFindBackward {
            value: operand,
            query,
            end_exclusive,
        } => ExpressionKindDraft::TextFindBackward {
            value: value(*operand)?,
            query: value(*query)?,
            end_exclusive: value(*end_exclusive)?,
        },
        OperationKind::TextLineEndingKind { value: operand } => {
            ExpressionKindDraft::TextLineEndingKind {
                value: value(*operand)?,
            }
        }
        OperationKind::TextDisplayWidth {
            value: operand,
            start,
            end_exclusive,
            initial_column,
            tab_width,
        } => ExpressionKindDraft::TextDisplayWidth {
            value: value(*operand)?,
            start: value(*start)?,
            end_exclusive: value(*end_exclusive)?,
            initial_column: value(*initial_column)?,
            tab_width: value(*tab_width)?,
        },
        OperationKind::TextCellPrefixBoundary {
            value: operand,
            start,
            end_exclusive,
            initial_column,
            maximum_cells,
            tab_width,
        } => ExpressionKindDraft::TextCellPrefixBoundary {
            value: value(*operand)?,
            start: value(*start)?,
            end_exclusive: value(*end_exclusive)?,
            initial_column: value(*initial_column)?,
            maximum_cells: value(*maximum_cells)?,
            tab_width: value(*tab_width)?,
        },
        OperationKind::TextFromScalar { value: operand } => ExpressionKindDraft::TextFromScalar {
            value: value(*operand)?,
        },
        OperationKind::TextToScalars {
            sequence,
            value: operand,
        } => ExpressionKindDraft::TextToScalars {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
        },
        OperationKind::TextFromScalars {
            sequence,
            value: operand,
        } => ExpressionKindDraft::TextFromScalars {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
        },
        OperationKind::SequenceEmpty { sequence } => ExpressionKindDraft::SequenceEmpty {
            sequence: NodeTarget::Existing(*sequence),
        },
        OperationKind::SequenceLen {
            sequence,
            value: operand,
        } => ExpressionKindDraft::SequenceLen {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
        },
        OperationKind::SequenceGet {
            sequence,
            value: operand,
            index,
        } => ExpressionKindDraft::SequenceGet {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
            index: value(*index)?,
        },
        OperationKind::SequenceAppend {
            sequence,
            value: operand,
            element,
        } => ExpressionKindDraft::SequenceAppend {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
            element: value(*element)?,
        },
        OperationKind::SequenceReplace {
            sequence,
            value: operand,
            index,
            element,
        } => ExpressionKindDraft::SequenceReplace {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
            index: value(*index)?,
            element: value(*element)?,
        },
        OperationKind::SequenceSlice {
            sequence,
            value: operand,
            start,
            end_exclusive,
        } => ExpressionKindDraft::SequenceSlice {
            sequence: NodeTarget::Existing(*sequence),
            value: value(*operand)?,
            start: value(*start)?,
            end_exclusive: value(*end_exclusive)?,
        },
        OperationKind::SequenceConcat { sequence, lhs, rhs } => {
            ExpressionKindDraft::SequenceConcat {
                sequence: NodeTarget::Existing(*sequence),
                lhs: value(*lhs)?,
                rhs: value(*rhs)?,
            }
        }
        OperationKind::SequenceRepeat {
            sequence,
            element,
            count,
        } => ExpressionKindDraft::SequenceRepeat {
            sequence: NodeTarget::Existing(*sequence),
            element: value(*element)?,
            count: value(*count)?,
        },
        OperationKind::Call {
            function,
            arguments,
        } => ExpressionKindDraft::Call {
            function: NodeTarget::Existing(*function),
            arguments: arguments
                .iter()
                .map(|argument| value(*argument))
                .collect::<crate::Result<Vec<_>>>()?,
        },
        OperationKind::Hole { .. } => {
            return Err(document_render_error(
                "durable repair anchors use refine_hole rather than whole-body replacement",
                None,
            ));
        }
        OperationKind::If {
            condition,
            result,
            then_region,
            else_region,
        } => ExpressionKindDraft::If {
            condition: value(*condition)?,
            result: TypeDraft::from(*result),
            then_body: render_yielding_body_draft(nodes, *then_region, depth + 1)?,
            else_body: render_yielding_body_draft(nodes, *else_region, depth + 1)?,
        },
        OperationKind::ForI64 {
            start,
            end_exclusive,
            step,
            initial,
            carried,
            body_region,
        } => {
            let arguments = region_arguments(nodes, *body_region)?;
            if arguments.len() != 2 {
                return Err(document_render_error(
                    "counted-loop body must carry its canonical index and value binders",
                    Some(*body_region),
                ));
            }
            ExpressionKindDraft::ForI64 {
                start: value(*start)?,
                end_exclusive: value(*end_exclusive)?,
                step: *step,
                initial: value(*initial)?,
                carried: TypeDraft::from(*carried),
                index_symbol: local_symbol(arguments[0])?,
                carried_symbol: local_symbol(arguments[1])?,
                body: render_yielding_body_draft(nodes, *body_region, depth + 1)?,
            }
        }
        OperationKind::ConstructProduct { product, fields } => {
            ExpressionKindDraft::ConstructProduct {
                product: NodeTarget::Existing(*product),
                fields: fields
                    .iter()
                    .map(|field| render_product_field(nodes, field))
                    .collect::<crate::Result<Vec<_>>>()?,
            }
        }
        OperationKind::ProjectField {
            value: operand,
            field,
        } => ExpressionKindDraft::ProjectField {
            value: value(*operand)?,
            field: NodeTarget::Existing(*field),
        },
        OperationKind::ConstructVariant { variant, payload } => {
            ExpressionKindDraft::ConstructVariant {
                variant: NodeTarget::Existing(*variant),
                payload: payload.map(value).transpose()?,
            }
        }
        OperationKind::MatchSum {
            scrutinee,
            result,
            arms,
        } => ExpressionKindDraft::MatchSum {
            scrutinee: value(*scrutinee)?,
            result: TypeDraft::from(*result),
            arms: arms
                .iter()
                .map(|arm| render_match_arm(nodes, arm, depth + 1))
                .collect::<crate::Result<Vec<_>>>()?,
        },
        OperationKind::Return { .. } | OperationKind::Yield { .. } => {
            return Err(document_render_error(
                "terminator appears in the editable body operation list",
                None,
            ));
        }
    })
}

fn render_product_field(
    nodes: &BTreeMap<NodeId, &Node>,
    field: &ProductFieldValue,
) -> crate::Result<ProductFieldValueDraft> {
    Ok(ProductFieldValueDraft {
        field: NodeTarget::Existing(field.field),
        value: render_value_draft(nodes, field.value)?,
    })
}

fn render_match_arm(
    nodes: &BTreeMap<NodeId, &Node>,
    arm: &MatchArm,
    depth: usize,
) -> crate::Result<MatchArmDraft> {
    let arguments = region_arguments(nodes, arm.region)?;
    if arguments.len() > 1 {
        return Err(document_render_error(
            "variant arm has more than one payload binder",
            Some(arm.region),
        ));
    }
    Ok(MatchArmDraft {
        variant: NodeTarget::Existing(arm.variant),
        payload_symbol: arguments.first().copied().map(local_symbol).transpose()?,
        body: render_yielding_body_draft(nodes, arm.region, depth)?,
    })
}

fn region_arguments(nodes: &BTreeMap<NodeId, &Node>, region: NodeId) -> crate::Result<Vec<NodeId>> {
    let block = match nodes.get(&region).copied() {
        Some(Node::Region { blocks, .. }) if blocks.len() == 1 => blocks[0],
        _ => {
            return Err(document_render_error(
                "structured region is absent or noncanonical in the packet",
                Some(region),
            ));
        }
    };
    match nodes.get(&block).copied() {
        Some(Node::Block { arguments, .. }) => Ok(arguments.clone()),
        _ => Err(document_render_error(
            "structured block is absent from the packet",
            Some(block),
        )),
    }
}

fn render_value_draft(
    nodes: &BTreeMap<NodeId, &Node>,
    value: ValueRef,
) -> crate::Result<crate::schema::ValueDraft> {
    Ok(match value {
        ValueRef::FunctionParameter(parameter) => {
            if !matches!(nodes.get(&parameter).copied(), Some(Node::Parameter { .. })) {
                return Err(document_render_error(
                    "function parameter is absent from the packet",
                    Some(parameter),
                ));
            }
            crate::schema::ValueDraft::FunctionParameter(NodeTarget::Existing(parameter))
        }
        ValueRef::OperationResult { operation, output } => {
            if output != 0 {
                return Err(document_render_error(
                    "operation result uses an unsupported output index",
                    Some(operation),
                ));
            }
            crate::schema::ValueDraft::OperationResult {
                operation: NodeTarget::Draft(local_symbol(operation)?),
                output,
            }
        }
        ValueRef::BlockArgument(argument) => {
            crate::schema::ValueDraft::BlockArgument(NodeTarget::Draft(local_symbol(argument)?))
        }
    })
}

fn local_symbol(id: NodeId) -> crate::Result<DraftSymbol> {
    let ordinal = id.local_ordinal().ok_or_else(|| {
        document_render_error("editable body value is not a revision-local term", Some(id))
    })?;
    Ok(DraftSymbol::new(&format!("v{ordinal}")))
}

fn render_document_value(value: &Value, depth: usize, output: &mut Vec<u8>) -> crate::Result<()> {
    if depth > MAX_DOCUMENT_DEPTH {
        return Err(crate::LkError::new(
            crate::ErrorCode::PolicyExceeded,
            "editable document rendering exceeds the depth policy",
        ));
    }
    match value {
        Value::Null => output.extend_from_slice(b"null"),
        Value::Bool(value) => output.extend_from_slice(if *value { b"true" } else { b"false" }),
        Value::Number(value) => output.extend_from_slice(value.to_string().as_bytes()),
        Value::String(value) if safe_bare_atom(value) => output.extend_from_slice(value.as_bytes()),
        Value::String(value) => output.extend_from_slice(
            serde_json::to_string(value)
                .map_err(|error| document_render_error(error.to_string(), None))?
                .as_bytes(),
        ),
        Value::Array(values) => {
            output.push(b'[');
            for value in values {
                output.push(b'\n');
                indent(output, depth + 1);
                render_document_value(value, depth + 1, output)?;
            }
            if !values.is_empty() {
                output.push(b'\n');
                indent(output, depth);
            }
            output.push(b']');
        }
        Value::Object(fields)
            if fields.len() <= 2
                && fields.contains_key("kind")
                && fields.keys().all(|key| key == "kind" || key == "data") =>
        {
            let kind = fields
                .get("kind")
                .and_then(Value::as_str)
                .ok_or_else(|| document_render_error("variant kind is not a string", None))?;
            output.push(b'(');
            output.extend_from_slice(kind.as_bytes());
            if let Some(data) = fields.get("data") {
                output.push(b' ');
                render_document_value(data, depth + 1, output)?;
            }
            output.push(b')');
        }
        Value::Object(fields) => {
            output.push(b'{');
            for (field, value) in fields {
                output.push(b'\n');
                indent(output, depth + 1);
                output.extend_from_slice(field.as_bytes());
                output.push(b' ');
                render_document_value(value, depth + 1, output)?;
            }
            if !fields.is_empty() {
                output.push(b'\n');
                indent(output, depth);
            }
            output.push(b'}');
        }
    }
    if output.len() > MAX_WORKBENCH_INPUT_BYTES {
        return Err(crate::LkError::new(
            crate::ErrorCode::PolicyExceeded,
            "editable document rendering exceeds the byte policy",
        ));
    }
    Ok(())
}

fn indent(output: &mut Vec<u8>, depth: usize) {
    output.resize(output.len().saturating_add(depth.saturating_mul(2)), b' ');
}

fn safe_bare_atom(value: &str) -> bool {
    valid_atom(value)
        && !matches!(value, "true" | "false" | "null")
        && !numeric_spelling(value)
        && !value.starts_with('@')
}

fn document_render_error(message: impl Into<String>, target: Option<NodeId>) -> crate::LkError {
    let error = crate::LkError::new(crate::ErrorCode::InvalidQuery, message);
    target.map_or(error.clone(), |target| error.for_node(target))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum EditScope {
    Workspace,
    Function(NodeId),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RunDocument {
    #[serde(default)]
    packet: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    revision: Revision,
    entry: NodeId,
    arguments: Vec<RuntimeValue>,
    policy: RunPolicy,
}

pub fn parse_edit_document(
    bytes: &[u8],
    mode: TransactionMode,
    packet: Option<&ContextPacket>,
) -> Result<ParsedEditDocument, DocumentError> {
    let parsed = parse_document(bytes, "document", packet)?;
    let document: EditDocument = serde_json::from_value(parsed.value).map_err(|error| {
        DocumentError::shape(format!("edit document shape is invalid: {error}"))
    })?;
    validate_edit_contract(&document, packet)?;
    bind_packet(
        document.packet,
        document.workspace,
        document.base_revision,
        parsed.alias_count,
        packet,
    )?;
    if mode == TransactionMode::ValidateOnly && document.idempotency_key.is_some() {
        return Err(DocumentError::shape(
            "validate-only documents cannot carry an idempotency key",
        ));
    }
    Ok(ParsedEditDocument {
        packet: document.packet,
        request: ApplyTransactionRequest {
            transaction: Transaction {
                workspace: document.workspace,
                base_revision: document.base_revision,
                idempotency_key: document.idempotency_key,
                mode,
                operations: document.edits,
            },
            response: TransactionResponseSpec {
                return_symbols: document.return_symbols,
            },
        },
        alias_count: parsed.alias_count,
    })
}

fn validate_edit_contract(
    document: &EditDocument,
    packet: Option<&ContextPacket>,
) -> Result<(), DocumentError> {
    if document.version != EDIT_DOCUMENT_VERSION {
        return Err(DocumentError::at(
            DocumentErrorCode::UnsupportedVersion,
            Location::start(),
            "editable semantic document version is unsupported",
        ));
    }
    let active = active_machine_schema_digest().map_err(|error| {
        DocumentError::shape(format!(
            "cannot derive the active semantic contract: {error}"
        ))
    })?;
    if document.schema != active {
        return Err(DocumentError::at(
            DocumentErrorCode::StaleSchema,
            Location::start(),
            "editable semantic document schema digest is stale",
        ));
    }
    match document.scope {
        EditScope::Workspace => Ok(()),
        EditScope::Function(function) => {
            if function.workspace() != document.workspace || !function.is_durable() {
                return Err(DocumentError::at(
                    DocumentErrorCode::ScopeMismatch,
                    Location::start(),
                    "function scope must name a durable function in the document workspace",
                ));
            }
            if !matches!(
                document.edits.as_slice(),
                [TransactionOp::ReplaceFunctionBody { function: target, .. }] if *target == function
            ) {
                return Err(DocumentError::at(
                    DocumentErrorCode::ScopeMismatch,
                    Location::start(),
                    "function scope accepts exactly one matching function-body replacement",
                ));
            }
            if let Some(packet) = packet
                && !packet
                    .payload
                    .nodes
                    .iter()
                    .any(|view| view.summary.node == function)
            {
                return Err(DocumentError::at(
                    DocumentErrorCode::ScopeMismatch,
                    Location::start(),
                    "function scope is not present in the bound context packet",
                ));
            }
            Ok(())
        }
    }
}

pub fn parse_run_document(
    bytes: &[u8],
    packet: Option<&ContextPacket>,
) -> Result<ParsedRunDocument, DocumentError> {
    let parsed = parse_document(bytes, "run", packet)?;
    let document: RunDocument = serde_json::from_value(parsed.value)
        .map_err(|error| DocumentError::shape(format!("run document shape is invalid: {error}")))?;
    bind_packet(
        document.packet,
        document.workspace,
        document.revision,
        parsed.alias_count,
        packet,
    )?;
    Ok(ParsedRunDocument {
        packet: document.packet,
        workspace: document.workspace,
        revision: document.revision,
        entry: document.entry,
        arguments: document.arguments,
        policy: document.policy,
        alias_count: parsed.alias_count,
    })
}

fn bind_packet(
    declared: Option<ContextPacketDigest>,
    workspace: WorkspaceId,
    revision: Revision,
    alias_count: u64,
    packet: Option<&ContextPacket>,
) -> Result<(), DocumentError> {
    match (declared, packet) {
        (None, None) if alias_count == 0 => Ok(()),
        (None, Some(_)) => Err(DocumentError::at(
            DocumentErrorCode::PacketMismatch,
            Location::start(),
            "a supplied packet must be named by its digest in the document",
        )),
        (None, None) => Err(DocumentError::at(
            DocumentErrorCode::PacketRequired,
            Location::start(),
            "packet aliases require an exact packet",
        )),
        (Some(_), None) => Err(DocumentError::at(
            DocumentErrorCode::PacketRequired,
            Location::start(),
            "the declared packet was not supplied",
        )),
        (Some(digest), Some(packet)) => {
            if packet.digest != digest {
                return Err(DocumentError::at(
                    DocumentErrorCode::PacketMismatch,
                    Location::start(),
                    "document and supplied packet digests differ",
                ));
            }
            if packet.payload.workspace != workspace {
                return Err(DocumentError::at(
                    DocumentErrorCode::PacketMismatch,
                    Location::start(),
                    "document and packet workspaces differ",
                ));
            }
            if packet.payload.revision != revision {
                return Err(DocumentError::at(
                    DocumentErrorCode::PacketMismatch,
                    Location::start(),
                    "document and packet revisions differ",
                ));
            }
            Ok(())
        }
    }
}

struct ParsedDocument {
    value: Value,
    alias_count: u64,
}

fn parse_document(
    bytes: &[u8],
    expected_root: &str,
    packet: Option<&ContextPacket>,
) -> Result<ParsedDocument, DocumentError> {
    if bytes.len() > MAX_WORKBENCH_INPUT_BYTES {
        return Err(DocumentError::at(
            DocumentErrorCode::InputTooLarge,
            Location::start(),
            "workbench input exceeds the byte policy",
        ));
    }
    let input = std::str::from_utf8(bytes).map_err(|error| {
        DocumentError::at(
            DocumentErrorCode::InvalidUtf8,
            Location::from_offset(bytes, error.valid_up_to()),
            "workbench document is not valid UTF-8",
        )
    })?;
    let aliases = packet
        .map(|packet| {
            packet
                .payload
                .aliases
                .iter()
                .map(|alias| (alias.alias.clone(), alias.node.to_string()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    Parser::new(input, aliases).parse(expected_root)
}

#[derive(Clone, Copy)]
struct Location {
    offset: usize,
    line: u32,
    column: u32,
}

impl Location {
    const fn start() -> Self {
        Self {
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn from_offset(bytes: &[u8], offset: usize) -> Self {
        let mut line = 1_u32;
        let mut column = 1_u32;
        for byte in bytes.iter().take(offset) {
            if *byte == b'\n' {
                line = line.saturating_add(1);
                column = 1;
            } else {
                column = column.saturating_add(1);
            }
        }
        Self {
            offset,
            line,
            column,
        }
    }
}

enum TokenKind {
    OpenObject,
    CloseObject,
    OpenList,
    CloseList,
    OpenVariant,
    CloseVariant,
    Atom(String),
    String(String),
    Number(Number),
    Alias(String),
}

struct Token {
    kind: TokenKind,
    location: Location,
}

struct Lexer<'a> {
    input: &'a str,
    offset: usize,
    line: u32,
    column: u32,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            offset: 0,
            line: 1,
            column: 1,
        }
    }

    fn location(&self) -> Location {
        Location {
            offset: self.offset,
            line: self.line,
            column: self.column,
        }
    }

    fn next(&mut self) -> Result<Option<Token>, DocumentError> {
        self.skip_whitespace();
        if self.offset == self.input.len() {
            return Ok(None);
        }
        let location = self.location();
        let byte = self.input.as_bytes()[self.offset];
        let kind = match byte {
            b'{' => {
                self.advance_ascii(1);
                TokenKind::OpenObject
            }
            b'}' => {
                self.advance_ascii(1);
                TokenKind::CloseObject
            }
            b'[' => {
                self.advance_ascii(1);
                TokenKind::OpenList
            }
            b']' => {
                self.advance_ascii(1);
                TokenKind::CloseList
            }
            b'(' => {
                self.advance_ascii(1);
                TokenKind::OpenVariant
            }
            b')' => {
                self.advance_ascii(1);
                TokenKind::CloseVariant
            }
            b'"' => TokenKind::String(self.string(location)?),
            _ => self.atom(location)?,
        };
        Ok(Some(Token { kind, location }))
    }

    fn skip_whitespace(&mut self) {
        while self.offset < self.input.len() {
            match self.input.as_bytes()[self.offset] {
                b' ' | b'\t' | b'\r' => self.advance_ascii(1),
                b'\n' => {
                    self.offset += 1;
                    self.line = self.line.saturating_add(1);
                    self.column = 1;
                }
                _ => break,
            }
        }
    }

    fn string(&mut self, location: Location) -> Result<String, DocumentError> {
        let start = self.offset;
        self.advance_ascii(1);
        let mut escaped = false;
        while self.offset < self.input.len() {
            let byte = self.input.as_bytes()[self.offset];
            if byte == b'\n' || byte == b'\r' {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    self.location(),
                    "a quoted string cannot contain a raw line break",
                ));
            }
            self.advance_ascii(1);
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                let spelling = &self.input[start..self.offset];
                return serde_json::from_str::<String>(spelling).map_err(|error| {
                    DocumentError::at(
                        DocumentErrorCode::Syntax,
                        location,
                        format!("invalid JSON string literal: {error}"),
                    )
                });
            }
        }
        Err(DocumentError::at(
            DocumentErrorCode::Syntax,
            location,
            "unterminated quoted string",
        ))
    }

    fn atom(&mut self, location: Location) -> Result<TokenKind, DocumentError> {
        let start = self.offset;
        while self.offset < self.input.len() {
            let byte = self.input.as_bytes()[self.offset];
            if byte.is_ascii_whitespace() || b"{}[]()".contains(&byte) {
                break;
            }
            if matches!(byte, b',' | b';' | b'=') {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    self.location(),
                    "commas, semicolons, and equals signs are not part of the document grammar",
                ));
            }
            let character = self.input[self.offset..].chars().next().ok_or_else(|| {
                DocumentError::at(DocumentErrorCode::Syntax, location, "invalid token")
            })?;
            self.offset += character.len_utf8();
            self.column = self.column.saturating_add(1);
        }
        let atom = &self.input[start..self.offset];
        if atom.is_empty() {
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "expected a document token",
            ));
        }
        if let Some(alias) = atom.strip_prefix('@') {
            if !valid_alias(alias) {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "packet aliases must match @[a-z][a-z0-9_]*",
                ));
            }
            return Ok(TokenKind::Alias(alias.to_owned()));
        }
        if numeric_spelling(atom) {
            let number = atom.parse::<Number>().map_err(|_| {
                DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "integer literal is outside the JSON integer domain",
                )
            })?;
            return Ok(TokenKind::Number(number));
        }
        if !valid_atom(atom) {
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "bare atoms may contain ASCII letters, digits, underscore, hyphen, dot, or colon",
            ));
        }
        Ok(TokenKind::Atom(atom.to_owned()))
    }

    fn advance_ascii(&mut self, count: usize) {
        self.offset += count;
        self.column = self
            .column
            .saturating_add(u32::try_from(count).unwrap_or(u32::MAX));
    }
}

fn valid_alias(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
}

fn valid_atom(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.' | b':'))
}

fn numeric_spelling(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.len() <= 20 && digits.bytes().all(|byte| byte.is_ascii_digit())
}

enum Frame {
    Object {
        fields: Map<String, Value>,
        keys: BTreeSet<String>,
        pending: Option<(String, Location)>,
        location: Location,
    },
    List {
        values: Vec<Value>,
        location: Location,
    },
    Variant {
        kind: Option<(String, Location)>,
        data: Option<Value>,
        location: Location,
    },
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    aliases: BTreeMap<String, String>,
    frames: Vec<Frame>,
    root: Option<Value>,
    items: usize,
    alias_count: u64,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str, aliases: BTreeMap<String, String>) -> Self {
        Self {
            lexer: Lexer::new(input),
            aliases,
            frames: Vec::new(),
            root: None,
            items: 0,
            alias_count: 0,
        }
    }

    fn parse(mut self, expected_root: &str) -> Result<ParsedDocument, DocumentError> {
        let root = self.lexer.next()?.ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                Location::start(),
                "empty document input",
            )
        })?;
        match root.kind {
            TokenKind::Atom(value) if value == expected_root => {}
            _ => {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    root.location,
                    format!("expected {expected_root} document"),
                ));
            }
        }
        let opening = self.lexer.next()?.ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                self.lexer.location(),
                "expected document object",
            )
        })?;
        if !matches!(opening.kind, TokenKind::OpenObject) {
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                opening.location,
                "expected an object after the document name",
            ));
        }
        self.frames.push(Frame::Object {
            fields: Map::new(),
            keys: BTreeSet::new(),
            pending: None,
            location: opening.location,
        });
        while let Some(token) = self.lexer.next()? {
            self.consume(token)?;
            if self.root.is_some() {
                if let Some(trailing) = self.lexer.next()? {
                    return Err(DocumentError::at(
                        DocumentErrorCode::Syntax,
                        trailing.location,
                        "trailing input after the document",
                    ));
                }
                break;
            }
        }
        if !self.frames.is_empty() {
            let location = self
                .frames
                .last()
                .map(frame_location)
                .unwrap_or_else(|| self.lexer.location());
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "unterminated document container",
            ));
        }
        let value = self.root.ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                self.lexer.location(),
                "unterminated document document",
            )
        })?;
        Ok(ParsedDocument {
            value,
            alias_count: self.alias_count,
        })
    }

    fn consume(&mut self, token: Token) -> Result<(), DocumentError> {
        match token.kind {
            TokenKind::CloseObject => self.close_object(token.location),
            TokenKind::CloseList => self.close_list(token.location),
            TokenKind::CloseVariant => self.close_variant(token.location),
            TokenKind::OpenObject => self.open(Frame::Object {
                fields: Map::new(),
                keys: BTreeSet::new(),
                pending: None,
                location: token.location,
            }),
            TokenKind::OpenList => self.open(Frame::List {
                values: Vec::new(),
                location: token.location,
            }),
            TokenKind::OpenVariant => self.open(Frame::Variant {
                kind: None,
                data: None,
                location: token.location,
            }),
            TokenKind::Atom(value) => self.atom_value(value, token.location),
            TokenKind::String(value) => self.add_value(Value::String(value), token.location),
            TokenKind::Number(value) => self.add_value(Value::Number(value), token.location),
            TokenKind::Alias(alias) => {
                let value = self.aliases.get(alias.as_str()).ok_or_else(|| {
                    DocumentError::at(
                        if self.aliases.is_empty() {
                            DocumentErrorCode::PacketRequired
                        } else {
                            DocumentErrorCode::UnknownAlias
                        },
                        token.location,
                        format!("unknown packet alias @{alias}"),
                    )
                })?;
                self.alias_count = self.alias_count.checked_add(1).ok_or_else(|| {
                    DocumentError::at(
                        DocumentErrorCode::ItemLimitExceeded,
                        token.location,
                        "alias accounting overflow",
                    )
                })?;
                self.add_value(Value::String(value.clone()), token.location)
            }
        }
    }

    fn open(&mut self, frame: Frame) -> Result<(), DocumentError> {
        if self.frames.len() >= MAX_DOCUMENT_DEPTH {
            return Err(DocumentError::at(
                DocumentErrorCode::DepthExceeded,
                frame_location(&frame),
                "document nesting exceeds the depth policy",
            ));
        }
        if let Some(Frame::Object { pending: None, .. }) = self.frames.last() {
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                frame_location(&frame),
                "an object field name must precede its value",
            ));
        }
        if let Some(Frame::Variant { kind: None, .. }) = self.frames.last() {
            return Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                frame_location(&frame),
                "a tagged variant must begin with a kind atom",
            ));
        }
        self.bump_item(frame_location(&frame))?;
        self.frames.push(frame);
        Ok(())
    }

    fn atom_value(&mut self, value: String, location: Location) -> Result<(), DocumentError> {
        if let Some(Frame::Object {
            pending,
            keys,
            fields: _,
            location: _,
        }) = self.frames.last_mut()
            && pending.is_none()
        {
            if !keys.insert(value.clone()) {
                return Err(DocumentError::at(
                    DocumentErrorCode::DuplicateField,
                    location,
                    format!("duplicate object field {value}"),
                ));
            }
            *pending = Some((value, location));
            return Ok(());
        }
        if let Some(Frame::Variant { kind, data, .. }) = self.frames.last_mut()
            && kind.is_none()
        {
            if data.is_some() {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "variant kind must precede its payload",
                ));
            }
            *kind = Some((value, location));
            return Ok(());
        }
        let scalar = match value.as_str() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            "null" => Value::Null,
            _ => Value::String(value),
        };
        self.add_value(scalar, location)
    }

    fn close_object(&mut self, location: Location) -> Result<(), DocumentError> {
        let frame = self.frames.pop().ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "unexpected closing object delimiter",
            )
        })?;
        match frame {
            Frame::Object {
                fields,
                pending: None,
                ..
            } => self.finish_container(Value::Object(fields), location),
            Frame::Object {
                pending: Some((field, field_location)),
                ..
            } => Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                field_location,
                format!("object field {field} has no value"),
            )),
            other => {
                self.frames.push(other);
                Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "object delimiter closes a different container",
                ))
            }
        }
    }

    fn close_list(&mut self, location: Location) -> Result<(), DocumentError> {
        let frame = self.frames.pop().ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "unexpected closing list delimiter",
            )
        })?;
        match frame {
            Frame::List { values, .. } => self.finish_container(Value::Array(values), location),
            other => {
                self.frames.push(other);
                Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "list delimiter closes a different container",
                ))
            }
        }
    }

    fn close_variant(&mut self, location: Location) -> Result<(), DocumentError> {
        let frame = self.frames.pop().ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "unexpected closing variant delimiter",
            )
        })?;
        match frame {
            Frame::Variant {
                kind: Some((kind, _)),
                data,
                ..
            } => {
                let mut object = Map::new();
                object.insert("kind".to_owned(), Value::String(kind));
                if let Some(data) = data {
                    object.insert("data".to_owned(), data);
                }
                self.finish_container(Value::Object(object), location)
            }
            Frame::Variant { kind: None, .. } => Err(DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "tagged variant has no kind",
            )),
            other => {
                self.frames.push(other);
                Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "variant delimiter closes a different container",
                ))
            }
        }
    }

    fn finish_container(&mut self, value: Value, location: Location) -> Result<(), DocumentError> {
        if self.frames.is_empty() {
            if self.root.replace(value).is_some() {
                return Err(DocumentError::at(
                    DocumentErrorCode::Syntax,
                    location,
                    "multiple document documents are not allowed",
                ));
            }
            Ok(())
        } else {
            self.add_value(value, location)
        }
    }

    fn add_value(&mut self, value: Value, location: Location) -> Result<(), DocumentError> {
        self.bump_item(location)?;
        let frame = self.frames.last_mut().ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::Syntax,
                location,
                "value appears outside the document object",
            )
        })?;
        match frame {
            Frame::Object {
                fields, pending, ..
            } => {
                let (field, _) = pending.take().ok_or_else(|| {
                    DocumentError::at(
                        DocumentErrorCode::Syntax,
                        location,
                        "object value has no field name",
                    )
                })?;
                fields.insert(field, value);
                Ok(())
            }
            Frame::List { values, .. } => {
                values.push(value);
                Ok(())
            }
            Frame::Variant { kind, data, .. } => {
                if kind.is_none() {
                    return Err(DocumentError::at(
                        DocumentErrorCode::Syntax,
                        location,
                        "tagged variant must begin with a kind atom",
                    ));
                }
                if data.replace(value).is_some() {
                    return Err(DocumentError::at(
                        DocumentErrorCode::Syntax,
                        location,
                        "tagged variant accepts at most one payload value",
                    ));
                }
                Ok(())
            }
        }
    }

    fn bump_item(&mut self, location: Location) -> Result<(), DocumentError> {
        self.items = self.items.checked_add(1).ok_or_else(|| {
            DocumentError::at(
                DocumentErrorCode::ItemLimitExceeded,
                location,
                "document item accounting overflow",
            )
        })?;
        if self.items > MAX_DOCUMENT_ITEMS {
            return Err(DocumentError::at(
                DocumentErrorCode::ItemLimitExceeded,
                location,
                "document exceeds the item policy",
            ));
        }
        Ok(())
    }
}

fn frame_location(frame: &Frame) -> Location {
    match frame {
        Frame::Object { location, .. }
        | Frame::List { location, .. }
        | Frame::Variant { location, .. } => *location,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::WorkspaceId;
    use crate::transaction::TransactionOp;

    fn workspace() -> WorkspaceId {
        WorkspaceId::from_bytes([0x11; 16])
    }

    #[test]
    fn compact_document_normalizes_directly_to_the_typed_transaction() {
        let source = format!(
            "document {{ version 2 schema {} workspace {} base_revision 0 scope (workspace) edits [\n\
             (create_package {{ symbol app name \"deployment\" }})\n\
             ] return_symbols [app] }}",
            active_machine_schema_digest().expect("schema digest"),
            workspace()
        );
        let parsed = parse_edit_document(source.as_bytes(), TransactionMode::Commit, None)
            .expect("compact document");
        assert_eq!(parsed.request.transaction.workspace, workspace());
        assert_eq!(parsed.request.transaction.base_revision, Revision::INITIAL);
        assert!(matches!(
            parsed.request.transaction.operations.as_slice(),
            [TransactionOp::CreatePackage { symbol, name }]
                if *symbol == DraftSymbol::new("app") && name == "deployment"
        ));
        assert_eq!(
            parsed.request.response.return_symbols,
            vec![DraftSymbol::new("app")]
        );
    }

    #[test]
    fn parser_rejects_duplicates_trailing_input_and_mismatched_delimiters() {
        for (source, code) in [
            ("plan {}", DocumentErrorCode::Syntax),
            (
                "document { workspace x workspace y }",
                DocumentErrorCode::DuplicateField,
            ),
            ("document {} trailing", DocumentErrorCode::Syntax),
            ("document { edits [ }", DocumentErrorCode::Syntax),
            ("document { edits [(x a b)] }", DocumentErrorCode::Syntax),
        ] {
            assert_eq!(
                parse_edit_document(source.as_bytes(), TransactionMode::Commit, None)
                    .expect_err("malformed document")
                    .code,
                code
            );
        }
    }

    #[test]
    fn quoted_alias_spelling_is_not_alias_resolution() {
        let source = format!(
            "document {{ version 2 schema {} workspace {} base_revision 0 scope (workspace) edits [\n\
             (create_package {{ symbol app name \"@n1\" }})] }}",
            active_machine_schema_digest().expect("schema digest"),
            workspace()
        );
        let parsed = parse_edit_document(source.as_bytes(), TransactionMode::Commit, None)
            .expect("quoted alias name");
        assert_eq!(parsed.alias_count, 0);
        assert!(matches!(
            parsed.request.transaction.operations.as_slice(),
            [TransactionOp::CreatePackage { name, .. }] if name == "@n1"
        ));
    }

    #[test]
    fn unquoted_alias_without_packet_rejects_at_its_location() {
        let source = format!(
            "document {{\n version 2\n schema {}\n workspace {}\n base_revision 0\n scope (workspace)\n edits [\n\
             (rename_node {{ node @n1 name x }})] }}",
            active_machine_schema_digest().expect("schema digest"),
            workspace()
        );
        let error = parse_edit_document(source.as_bytes(), TransactionMode::Commit, None)
            .expect_err("packet required");
        assert_eq!(error.code, DocumentErrorCode::PacketRequired);
        assert_eq!(error.line, 8);
    }
}
