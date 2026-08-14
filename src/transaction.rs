use crate::diff;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace, require_kind};
use crate::ids::{
    ChangeDigest, IdempotencyKey, LocalHandle, NodeId, Revision, SnapshotHash, WorkspaceId,
};
use crate::query;
use crate::schema::{
    Node, NodeKind, OperationDraft, OperationKind, SemanticType, ValueDraft, ValueRef,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NodeTarget {
    Existing(NodeId),
    Local(LocalHandle),
}

pub const MAX_RETURNED_BINDINGS: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionMode {
    Commit,
    ValidateOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Transaction {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub mode: TransactionMode,
    pub operations: Vec<TransactionOp>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionResponseSpec {
    pub return_handles: Vec<LocalHandle>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyTransactionRequest {
    pub transaction: Transaction,
    pub response: TransactionResponseSpec,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionOpCode {
    CreatePackage,
    CreateModule,
    CreateFunction,
    CreateParameter,
    CreateRegion,
    CreateBlock,
    CreateOperation,
    SetFunctionBody,
    SetEntryFunction,
    RenameNode,
    ReplaceOperation,
    ReplaceOperand,
    DeleteOwnedSubtree,
    RefineHole,
}
impl TransactionOpCode {
    pub const ALL: [Self; 14] = [
        Self::CreatePackage,
        Self::CreateModule,
        Self::CreateFunction,
        Self::CreateParameter,
        Self::CreateRegion,
        Self::CreateBlock,
        Self::CreateOperation,
        Self::SetFunctionBody,
        Self::SetEntryFunction,
        Self::RenameNode,
        Self::ReplaceOperation,
        Self::ReplaceOperand,
        Self::DeleteOwnedSubtree,
        Self::RefineHole,
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::CreatePackage => 1,
            Self::CreateModule => 2,
            Self::CreateFunction => 3,
            Self::CreateParameter => 4,
            Self::CreateRegion => 5,
            Self::CreateBlock => 6,
            Self::CreateOperation => 7,
            Self::SetFunctionBody => 8,
            Self::SetEntryFunction => 9,
            Self::RenameNode => 10,
            Self::ReplaceOperation => 11,
            Self::ReplaceOperand => 12,
            Self::DeleteOwnedSubtree => 13,
            Self::RefineHole => 14,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::CreatePackage),
            2 => Some(Self::CreateModule),
            3 => Some(Self::CreateFunction),
            4 => Some(Self::CreateParameter),
            5 => Some(Self::CreateRegion),
            6 => Some(Self::CreateBlock),
            7 => Some(Self::CreateOperation),
            8 => Some(Self::SetFunctionBody),
            9 => Some(Self::SetEntryFunction),
            10 => Some(Self::RenameNode),
            11 => Some(Self::ReplaceOperation),
            12 => Some(Self::ReplaceOperand),
            13 => Some(Self::DeleteOwnedSubtree),
            14 => Some(Self::RefineHole),
            _ => None,
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::CreatePackage => "create_package",
            Self::CreateModule => "create_module",
            Self::CreateFunction => "create_function",
            Self::CreateParameter => "create_parameter",
            Self::CreateRegion => "create_region",
            Self::CreateBlock => "create_block",
            Self::CreateOperation => "create_operation",
            Self::SetFunctionBody => "set_function_body",
            Self::SetEntryFunction => "set_entry_function",
            Self::RenameNode => "rename_node",
            Self::ReplaceOperation => "replace_operation",
            Self::ReplaceOperand => "replace_operand",
            Self::RefineHole => "refine_hole",
            Self::DeleteOwnedSubtree => "delete_owned_subtree",
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
pub enum TransactionOp {
    CreatePackage {
        handle: LocalHandle,
        name: String,
    },
    CreateModule {
        handle: LocalHandle,
        package: NodeTarget,
        name: String,
    },
    CreateFunction {
        handle: LocalHandle,
        module: NodeTarget,
        name: String,
        result: SemanticType,
    },
    CreateParameter {
        handle: LocalHandle,
        function: NodeTarget,
        name: String,
        ty: SemanticType,
    },
    CreateRegion {
        handle: LocalHandle,
        function: NodeTarget,
    },
    CreateBlock {
        handle: LocalHandle,
        region: NodeTarget,
    },
    CreateOperation {
        handle: LocalHandle,
        block: NodeTarget,
        before: Option<NodeTarget>,
        operation: OperationDraft,
    },
    SetFunctionBody {
        function: NodeTarget,
        region: NodeTarget,
    },
    SetEntryFunction {
        package: NodeTarget,
        function: NodeTarget,
    },
    RenameNode {
        node: NodeTarget,
        name: String,
    },
    ReplaceOperation {
        operation: NodeTarget,
        replacement: OperationDraft,
    },
    ReplaceOperand {
        operation: NodeTarget,
        index: u8,
        value: ValueDraft,
    },
    RefineHole {
        hole: NodeTarget,
        replacement: OperationDraft,
    },
    DeleteOwnedSubtree {
        root: NodeTarget,
    },
}

impl TransactionOp {
    pub const fn code(&self) -> TransactionOpCode {
        match self {
            Self::CreatePackage { .. } => TransactionOpCode::CreatePackage,
            Self::CreateModule { .. } => TransactionOpCode::CreateModule,
            Self::CreateFunction { .. } => TransactionOpCode::CreateFunction,
            Self::CreateParameter { .. } => TransactionOpCode::CreateParameter,
            Self::CreateRegion { .. } => TransactionOpCode::CreateRegion,
            Self::CreateBlock { .. } => TransactionOpCode::CreateBlock,
            Self::CreateOperation { .. } => TransactionOpCode::CreateOperation,
            Self::SetFunctionBody { .. } => TransactionOpCode::SetFunctionBody,
            Self::SetEntryFunction { .. } => TransactionOpCode::SetEntryFunction,
            Self::RenameNode { .. } => TransactionOpCode::RenameNode,
            Self::ReplaceOperation { .. } => TransactionOpCode::ReplaceOperation,
            Self::ReplaceOperand { .. } => TransactionOpCode::ReplaceOperand,
            Self::RefineHole { .. } => TransactionOpCode::RefineHole,
            Self::DeleteOwnedSubtree { .. } => TransactionOpCode::DeleteOwnedSubtree,
        }
    }
    pub const fn created_handle(&self) -> Option<LocalHandle> {
        match self {
            Self::CreatePackage { handle, .. }
            | Self::CreateModule { handle, .. }
            | Self::CreateFunction { handle, .. }
            | Self::CreateParameter { handle, .. }
            | Self::CreateRegion { handle, .. }
            | Self::CreateBlock { handle, .. }
            | Self::CreateOperation { handle, .. } => Some(*handle),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionReceipt {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    pub revision: Revision,
    pub hash: SnapshotHash,
    pub published: bool,
    pub created_count: u64,
    pub returned_bindings: Vec<(LocalHandle, NodeId)>,
    pub change_count: u64,
    pub change_digest: ChangeDigest,
    pub complete_before: bool,
    pub complete_after: bool,
    pub blocker_count_before: u64,
    pub blocker_count_after: u64,
}

#[derive(Debug)]
pub(crate) struct PreparedTransaction {
    pub snapshot: Arc<Snapshot>,
    pub receipt: TransactionReceipt,
}

impl Workspace {
    pub(crate) fn prepare_transaction(
        &self,
        request: &ApplyTransactionRequest,
    ) -> Result<PreparedTransaction> {
        let transaction = &request.transaction;
        if transaction.workspace != self.id() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "transaction names a different workspace",
            )
            .for_workspace(self.id()));
        }
        if transaction.mode == TransactionMode::ValidateOnly
            && transaction.idempotency_key.is_some()
        {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "validate-only transactions cannot carry an idempotency key",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        if transaction.base_revision != self.head_revision() {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "transaction base revision is not the current head",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        if transaction.operations.is_empty() {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "empty transactions do not publish revisions",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        let base = self.snapshot(transaction.base_revision)?;
        let (allocations, next_serial) = allocate_handles(base, &transaction.operations)?;
        validate_response_spec(&request.response, &allocations)?;
        let mut nodes = base.nodes.clone();
        let mut tombstones = base.tombstones.clone();

        for (index, operation) in transaction.operations.iter().enumerate() {
            if let Err(mut error) =
                apply_operation(base, &mut nodes, &mut tombstones, &allocations, operation)
            {
                if error.operation_index.is_none() {
                    error = error.at_operation(index);
                }
                return Err(error);
            }
        }

        if nodes == base.nodes && tombstones == base.tombstones && next_serial == base.next_serial {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "transaction produced no canonical state change",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision));
        }
        let revision = transaction.base_revision.next().ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionConflict,
                "workspace revision is exhausted",
            )
            .for_workspace(self.id())
            .at_revision(transaction.base_revision)
        })?;
        let candidate = match Snapshot::from_parts(
            self.id(),
            revision,
            base.root,
            next_serial,
            tombstones,
            nodes,
        ) {
            Ok(candidate) => Arc::new(candidate),
            Err(mut error) => {
                if error.operation_index.is_none() {
                    error = error.at_operation(transaction.operations.len().saturating_sub(1));
                }
                return Err(error);
            }
        };
        let semantic_diff = diff::between(base, &candidate);
        let blockers_before = query::workspace_blockers(base);
        let blockers_after = query::workspace_blockers(&candidate);
        let returned_bindings = request
            .response
            .return_handles
            .iter()
            .map(|handle| allocated(&allocations, *handle).map(|node| (*handle, node)))
            .collect::<Result<Vec<_>>>()?;
        let receipt = TransactionReceipt {
            workspace: self.id(),
            base_revision: transaction.base_revision,
            revision,
            hash: candidate.hash(),
            published: transaction.mode == TransactionMode::Commit,
            created_count: u64::try_from(allocations.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "created node count does not fit receipt representation",
                )
            })?,
            returned_bindings,
            change_count: semantic_diff.change_count(),
            change_digest: semantic_diff.digest,
            complete_before: blockers_before.is_empty(),
            complete_after: blockers_after.is_empty(),
            blocker_count_before: u64::try_from(blockers_before.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "prior blocker count does not fit receipt representation",
                )
            })?,
            blocker_count_after: u64::try_from(blockers_after.len()).map_err(|_| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "result blocker count does not fit receipt representation",
                )
            })?,
        };
        Ok(PreparedTransaction {
            snapshot: candidate,
            receipt,
        })
    }
}

fn validate_response_spec(
    response: &TransactionResponseSpec,
    allocations: &BTreeMap<LocalHandle, NodeId>,
) -> Result<()> {
    if response.return_handles.len() > MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "selected return handles exceed transaction response policy",
        ));
    }
    let mut previous = None;
    for handle in &response.return_handles {
        if previous.is_some_and(|prior| *handle <= prior) {
            return Err(LkError::new(
                ErrorCode::InvalidHandle,
                "selected return handles must be unique and strictly increasing",
            ));
        }
        if !allocations.contains_key(handle) {
            return Err(LkError::new(
                ErrorCode::InvalidHandle,
                "selected return handle is not declared by this transaction",
            ));
        }
        previous = Some(*handle);
    }
    Ok(())
}

fn allocate_handles(
    base: &Snapshot,
    operations: &[TransactionOp],
) -> Result<(BTreeMap<LocalHandle, NodeId>, u64)> {
    let mut allocations = BTreeMap::new();
    let mut next = base.next_serial;
    for (index, operation) in operations.iter().enumerate() {
        let Some(handle) = operation.created_handle() else {
            continue;
        };
        if allocations.contains_key(&handle) {
            return Err(LkError::new(
                ErrorCode::DuplicateHandle,
                "transaction-local handle is declared more than once",
            )
            .at_operation(index));
        }
        let id = NodeId::new(base.workspace(), next).map_err(|error| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                format!("node identity allocation failed: {error}"),
            )
            .at_operation(index)
        })?;
        next = next.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "node identity serial is exhausted",
            )
            .at_operation(index)
        })?;
        allocations.insert(handle, id);
    }
    Ok((allocations, next))
}

fn apply_operation(
    base: &Snapshot,
    nodes: &mut BTreeMap<NodeId, Node>,
    tombstones: &mut BTreeSet<u64>,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    operation: &TransactionOp,
) -> Result<()> {
    match operation {
        TransactionOp::CreatePackage { handle, name } => {
            let id = allocated(allocations, *handle)?;
            insert_new(
                nodes,
                id,
                Node::Package {
                    owner: base.root,
                    name: name.clone(),
                    modules: Vec::new(),
                    entry: None,
                },
            )?;
            let root = require_kind_mut(nodes, base.root, NodeKind::WorkspaceRoot)?;
            let Node::WorkspaceRoot { packages } = root else {
                return Err(invariant("workspace root kind changed during staging"));
            };
            packages.push(id);
        }
        TransactionOp::CreateModule {
            handle,
            package,
            name,
        } => {
            let id = allocated(allocations, *handle)?;
            let package = resolve(*package, allocations, base.workspace())?;
            require_kind(nodes, package, NodeKind::Package)?;
            insert_new(
                nodes,
                id,
                Node::Module {
                    owner: package,
                    name: name.clone(),
                    functions: Vec::new(),
                },
            )?;
            let Node::Package { modules, .. } =
                require_kind_mut(nodes, package, NodeKind::Package)?
            else {
                return Err(invariant("package kind changed during staging"));
            };
            modules.push(id);
        }
        TransactionOp::CreateFunction {
            handle,
            module,
            name,
            result,
        } => {
            let id = allocated(allocations, *handle)?;
            let module = resolve(*module, allocations, base.workspace())?;
            require_kind(nodes, module, NodeKind::Module)?;
            insert_new(
                nodes,
                id,
                Node::Function {
                    owner: module,
                    name: name.clone(),
                    parameters: Vec::new(),
                    result: *result,
                    body: None,
                },
            )?;
            let Node::Module { functions, .. } = require_kind_mut(nodes, module, NodeKind::Module)?
            else {
                return Err(invariant("module kind changed during staging"));
            };
            functions.push(id);
        }
        TransactionOp::CreateParameter {
            handle,
            function,
            name,
            ty,
        } => {
            let id = allocated(allocations, *handle)?;
            let function = resolve(*function, allocations, base.workspace())?;
            let ordinal = match require_kind(nodes, function, NodeKind::Function)? {
                Node::Function { parameters, .. } => {
                    u32::try_from(parameters.len()).map_err(|_| {
                        LkError::new(
                            ErrorCode::PolicyExceeded,
                            "parameter ordinal exceeds protocol representation",
                        )
                        .for_node(function)
                    })?
                }
                _ => return Err(invariant("function kind changed during staging")),
            };
            insert_new(
                nodes,
                id,
                Node::Parameter {
                    owner: function,
                    ordinal,
                    name: name.clone(),
                    ty: *ty,
                },
            )?;
            let Node::Function { parameters, .. } =
                require_kind_mut(nodes, function, NodeKind::Function)?
            else {
                return Err(invariant("function kind changed during staging"));
            };
            parameters.push(id);
        }
        TransactionOp::CreateRegion { handle, function } => {
            let id = allocated(allocations, *handle)?;
            let function = resolve(*function, allocations, base.workspace())?;
            require_kind(nodes, function, NodeKind::Function)?;
            insert_new(
                nodes,
                id,
                Node::Region {
                    owner: function,
                    blocks: Vec::new(),
                },
            )?;
        }
        TransactionOp::CreateBlock { handle, region } => {
            let id = allocated(allocations, *handle)?;
            let region = resolve(*region, allocations, base.workspace())?;
            require_kind(nodes, region, NodeKind::Region)?;
            insert_new(
                nodes,
                id,
                Node::Block {
                    owner: region,
                    operations: Vec::new(),
                    terminator: None,
                },
            )?;
            let Node::Region { blocks, .. } = require_kind_mut(nodes, region, NodeKind::Region)?
            else {
                return Err(invariant("region kind changed during staging"));
            };
            blocks.push(id);
        }
        TransactionOp::CreateOperation {
            handle,
            block,
            before,
            operation,
        } => {
            let id = allocated(allocations, *handle)?;
            let block = resolve(*block, allocations, base.workspace())?;
            require_kind(nodes, block, NodeKind::Block)?;
            let operation = resolve_operation(operation, allocations, base.workspace())?;
            let terminator = operation.is_terminator();
            insert_new(
                nodes,
                id,
                Node::Operation {
                    owner: block,
                    operation,
                },
            )?;
            let before = before
                .map(|target| resolve(target, allocations, base.workspace()))
                .transpose()?;
            let Node::Block {
                operations,
                terminator: block_terminator,
                ..
            } = require_kind_mut(nodes, block, NodeKind::Block)?
            else {
                return Err(invariant("block kind changed during staging"));
            };
            if terminator {
                if before.is_some() || block_terminator.is_some() {
                    return Err(LkError::new(
                        ErrorCode::InvalidContainment,
                        "block already has a terminator or terminator requested an order anchor",
                    )
                    .for_node(block));
                }
                *block_terminator = Some(id);
            } else if let Some(before) = before {
                let position = operations
                    .iter()
                    .position(|candidate| *candidate == before)
                    .ok_or_else(|| {
                        LkError::new(
                            ErrorCode::InvalidContainment,
                            "operation order anchor is not a regular operation in this block",
                        )
                        .for_node(before)
                        .with_related([block])
                    })?;
                operations.insert(position, id);
            } else {
                operations.push(id);
            }
        }
        TransactionOp::SetFunctionBody { function, region } => {
            let function = resolve(*function, allocations, base.workspace())?;
            let region = resolve(*region, allocations, base.workspace())?;
            require_kind(nodes, region, NodeKind::Region)?;
            let Node::Function { body, .. } =
                require_kind_mut(nodes, function, NodeKind::Function)?
            else {
                return Err(invariant("function kind changed during staging"));
            };
            *body = Some(region);
        }
        TransactionOp::SetEntryFunction { package, function } => {
            let package = resolve(*package, allocations, base.workspace())?;
            let function = resolve(*function, allocations, base.workspace())?;
            require_kind(nodes, function, NodeKind::Function)?;
            let Node::Package { entry, .. } = require_kind_mut(nodes, package, NodeKind::Package)?
            else {
                return Err(invariant("package kind changed during staging"));
            };
            *entry = Some(function);
        }
        TransactionOp::RenameNode { node, name } => {
            let node = resolve(*node, allocations, base.workspace())?;
            let target = nodes.get_mut(&node).ok_or_else(|| missing(node))?;
            if !target.set_name(name.clone()) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "this node kind has no display name",
                )
                .for_node(node));
            }
        }
        TransactionOp::ReplaceOperation {
            operation,
            replacement,
        } => {
            let operation = resolve(*operation, allocations, base.workspace())?;
            let replacement = resolve_operation(replacement, allocations, base.workspace())?;
            let Node::Operation {
                operation: current, ..
            } = require_kind_mut(nodes, operation, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            if current.code() != replacement.code()
                || !current.same_result_contract(&replacement)
                || current.is_terminator() != replacement.is_terminator()
            {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "identity-preserving operation replacement requires the same operation contract",
                )
                .for_node(operation));
            }
            *current = replacement;
        }
        TransactionOp::ReplaceOperand {
            operation,
            index,
            value,
        } => {
            let operation = resolve(*operation, allocations, base.workspace())?;
            let value = resolve_value(*value, allocations, base.workspace())?;
            let Node::Operation {
                operation: current, ..
            } = require_kind_mut(nodes, operation, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            if !current.replace_operand(*index, value) {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "operand index is outside the operation contract",
                )
                .for_node(operation));
            }
        }
        TransactionOp::RefineHole { hole, replacement } => {
            let hole = resolve(*hole, allocations, base.workspace())?;
            let replacement = resolve_operation(replacement, allocations, base.workspace())?;
            let (owner, expected, current_result_count) =
                match require_kind(nodes, hole, NodeKind::Operation)? {
                    Node::Operation {
                        owner,
                        operation: current @ OperationKind::Hole { expected },
                    } => (*owner, *expected, current.result_count()),
                    Node::Operation { .. } => {
                        return Err(LkError::new(
                            ErrorCode::InvalidOperand,
                            "hole refinement target is already a complete operation",
                        )
                        .for_node(hole));
                    }
                    _ => return Err(invariant("operation kind changed during staging")),
                };
            let Node::Block {
                operations,
                terminator,
                ..
            } = require_kind(nodes, owner, NodeKind::Block)?
            else {
                return Err(invariant("operation owner kind changed during staging"));
            };
            if !operations.contains(&hole) || *terminator == Some(hole) {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "hole refinement target must occupy a regular operation slot",
                )
                .for_node(hole)
                .with_related([owner]));
            }
            if !replacement.is_complete()
                || replacement.is_terminator()
                || matches!(replacement, OperationKind::Hole { .. })
            {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "hole replacement must be a complete non-terminator operation",
                )
                .for_node(hole));
            }
            let actual = replacement.result_type(0, None);
            if current_result_count != replacement.result_count() || actual != Some(expected) {
                let mut error = LkError::new(
                    ErrorCode::TypeMismatch,
                    "hole replacement result contract does not match the expected type",
                )
                .for_node(hole);
                if let Some(actual) = actual {
                    error = error.with_types(expected, actual);
                } else {
                    error.expected_type = Some(expected);
                }
                return Err(error);
            }
            let Node::Operation {
                operation: current, ..
            } = require_kind_mut(nodes, hole, NodeKind::Operation)?
            else {
                return Err(invariant("operation kind changed during staging"));
            };
            *current = replacement;
        }
        TransactionOp::DeleteOwnedSubtree { root } => {
            let root = resolve(*root, allocations, base.workspace())?;
            delete_subtree(base.root, nodes, tombstones, root)?;
        }
    }
    Ok(())
}

fn resolve_operation(
    operation: &OperationDraft,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    workspace: WorkspaceId,
) -> Result<OperationKind> {
    Ok(match operation {
        OperationDraft::ConstI64(value) => OperationKind::ConstI64(*value),
        OperationDraft::ConstBool(value) => OperationKind::ConstBool(*value),
        OperationDraft::AddI64 { lhs, rhs } => OperationKind::AddI64 {
            lhs: resolve_value(*lhs, allocations, workspace)?,
            rhs: resolve_value(*rhs, allocations, workspace)?,
        },
        OperationDraft::Hole { expected } => OperationKind::Hole {
            expected: *expected,
        },
        OperationDraft::Return { value } => OperationKind::Return {
            value: resolve_value(*value, allocations, workspace)?,
        },
    })
}

fn resolve_value(
    value: ValueDraft,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    workspace: WorkspaceId,
) -> Result<ValueRef> {
    Ok(match value {
        ValueDraft::FunctionParameter(parameter) => {
            ValueRef::FunctionParameter(resolve(parameter, allocations, workspace)?)
        }
        ValueDraft::OperationResult { operation, output } => ValueRef::OperationResult {
            operation: resolve(operation, allocations, workspace)?,
            output,
        },
    })
}

fn resolve(
    target: NodeTarget,
    allocations: &BTreeMap<LocalHandle, NodeId>,
    workspace: WorkspaceId,
) -> Result<NodeId> {
    match target {
        NodeTarget::Existing(id) => {
            if id.workspace() != workspace {
                return Err(LkError::new(
                    ErrorCode::WrongWorkspace,
                    "transaction target belongs to another workspace",
                )
                .for_workspace(workspace)
                .for_node(id));
            }
            Ok(id)
        }
        NodeTarget::Local(handle) => allocations.get(&handle).copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidHandle,
                "transaction references an undeclared local handle",
            )
        }),
    }
}

fn allocated(allocations: &BTreeMap<LocalHandle, NodeId>, handle: LocalHandle) -> Result<NodeId> {
    allocations.get(&handle).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::InvalidHandle,
            "create operation has no staged node allocation",
        )
    })
}

fn insert_new(nodes: &mut BTreeMap<NodeId, Node>, id: NodeId, node: Node) -> Result<()> {
    if nodes.insert(id, node).is_some() {
        return Err(LkError::new(
            ErrorCode::InvalidHandle,
            "staged node identity already exists",
        )
        .for_node(id));
    }
    Ok(())
}

fn require_kind_mut(
    nodes: &mut BTreeMap<NodeId, Node>,
    id: NodeId,
    expected: NodeKind,
) -> Result<&mut Node> {
    let node = nodes.get_mut(&id).ok_or_else(|| missing(id))?;
    let actual = node.kind();
    if actual != expected {
        return Err(
            LkError::new(ErrorCode::WrongKind, "target has the wrong node kind")
                .for_node(id)
                .with_kinds(expected, actual),
        );
    }
    Ok(node)
}

fn delete_subtree(
    workspace_root: NodeId,
    nodes: &mut BTreeMap<NodeId, Node>,
    tombstones: &mut BTreeSet<u64>,
    root: NodeId,
) -> Result<()> {
    if root == workspace_root {
        return Err(
            LkError::new(ErrorCode::DeleteBlocked, "workspace root cannot be deleted")
                .for_node(root),
        );
    }
    let root_node = nodes.get(&root).ok_or_else(|| missing(root))?;
    let owner = root_node.owner().ok_or_else(|| {
        LkError::new(
            ErrorCode::OwnerMismatch,
            "deleted subtree root has no owner",
        )
        .for_node(root)
    })?;
    let mut deleted = BTreeSet::new();
    let mut stack = vec![root];
    while let Some(id) = stack.pop() {
        if !deleted.insert(id) {
            continue;
        }
        let node = nodes.get(&id).ok_or_else(|| missing(id))?;
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                stack.push(child);
            }
        }
    }
    for (source, node) in nodes.iter() {
        if deleted.contains(source) {
            continue;
        }
        let mut blockers = (0..node.direct_reference_count())
            .filter_map(|index| {
                node.direct_reference(index)
                    .map(|reference| reference.target())
            })
            .filter(|target| deleted.contains(target));
        if let Some(first) = blockers.next() {
            return Err(LkError::new(
                ErrorCode::DeleteBlocked,
                "surviving node directly references the requested deletion subtree",
            )
            .for_node(root)
            .with_related(
                std::iter::once(*source)
                    .chain(std::iter::once(first))
                    .chain(blockers),
            ));
        }
    }
    detach_child(nodes, owner, root)?;
    for id in deleted {
        if nodes.remove(&id).is_none() {
            return Err(invariant("deletion traversal lost a staged node"));
        }
        tombstones.insert(id.serial());
    }
    Ok(())
}

fn detach_child(nodes: &mut BTreeMap<NodeId, Node>, owner: NodeId, child: NodeId) -> Result<()> {
    let owner_node = nodes.get_mut(&owner).ok_or_else(|| missing(owner))?;
    let removed = match owner_node {
        Node::WorkspaceRoot { packages } => remove_one(packages, child),
        Node::Package { modules, .. } => remove_one(modules, child),
        Node::Module { functions, .. } => remove_one(functions, child),
        Node::Function {
            parameters, body, ..
        } => {
            if *body == Some(child) {
                *body = None;
                true
            } else {
                remove_one(parameters, child)
            }
        }
        Node::Region { blocks, .. } => remove_one(blocks, child),
        Node::Block {
            operations,
            terminator,
            ..
        } => {
            if *terminator == Some(child) {
                *terminator = None;
                true
            } else {
                remove_one(operations, child)
            }
        }
        Node::Parameter { .. } | Node::Operation { .. } => false,
    };
    if !removed {
        return Err(LkError::new(
            ErrorCode::OwnerMismatch,
            "owner does not contain requested deletion root",
        )
        .for_node(child)
        .with_related([owner]));
    }
    Ok(())
}

fn remove_one(values: &mut Vec<NodeId>, target: NodeId) -> bool {
    let Some(position) = values.iter().position(|value| *value == target) else {
        return false;
    };
    values.remove(position);
    true
}

fn missing(id: NodeId) -> LkError {
    LkError::new(ErrorCode::NodeNotFound, "transaction target does not exist").for_node(id)
}

fn invariant(message: &str) -> LkError {
    LkError::new(ErrorCode::InvalidContainment, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(transaction: &Transaction) -> ApplyTransactionRequest {
        let mut return_handles: Vec<LocalHandle> = transaction
            .operations
            .iter()
            .filter_map(TransactionOp::created_handle)
            .collect();
        return_handles.sort();
        return_handles.truncate(MAX_RETURNED_BINDINGS);
        ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec { return_handles },
        }
    }

    fn commit(workspace: &mut Workspace, transaction: &Transaction) -> Result<TransactionReceipt> {
        let prepared = workspace.prepare_transaction(&request(transaction))?;
        let receipt = prepared.receipt.clone();
        if transaction.mode == TransactionMode::Commit {
            workspace.publish(prepared.snapshot)?;
        }
        Ok(receipt)
    }

    fn create_package_and_module(id: WorkspaceId) -> Transaction {
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "package".to_owned(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: NodeTarget::Local(LocalHandle::new(1)),
                    name: "module".to_owned(),
                },
            ],
        }
    }

    #[test]
    fn failed_batches_and_validate_only_do_not_consume_node_ids() {
        let id = WorkspaceId::from_bytes([11; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.returned_bindings[1].1;
        assert_eq!(module.serial(), 3);

        let failed = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: NodeTarget::Existing(module),
                    name: "duplicate".to_owned(),
                    result: SemanticType::I64,
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(4),
                    module: NodeTarget::Existing(module),
                    name: "duplicate".to_owned(),
                    result: SemanticType::I64,
                },
            ],
        };
        let error = workspace
            .prepare_transaction(&request(&failed))
            .expect_err("duplicate names must reject");
        assert_eq!(error.code, ErrorCode::DuplicateName);
        assert_eq!(workspace.head_revision(), Revision::new(1));
        assert_eq!(workspace.head().expect("head").next_serial(), 4);

        let validate_only = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(5),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let predicted = commit(&mut workspace, &validate_only).expect("validate only");
        assert_eq!(predicted.returned_bindings[0].1.serial(), 4);
        assert_eq!(workspace.head_revision(), Revision::new(1));

        let mut real = validate_only;
        real.mode = TransactionMode::Commit;
        let committed = commit(&mut workspace, &real).expect("real commit");
        assert_eq!(
            committed.returned_bindings[0].1,
            predicted.returned_bindings[0].1
        );
        assert_eq!(workspace.head_revision(), Revision::new(2));
    }

    #[test]
    fn deletion_tombstones_identity_and_old_snapshots_retain_nodes() {
        let id = WorkspaceId::from_bytes([12; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.returned_bindings[1].1;
        let create = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let created = commit(&mut workspace, &create).expect("create function");
        let function = created.returned_bindings[0].1;
        assert_eq!(function.serial(), 4);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(2),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(function),
            }],
        };
        commit(&mut workspace, &delete).expect("delete function");
        assert!(
            workspace
                .snapshot(Revision::new(2))
                .expect("old snapshot")
                .node(function)
                .is_ok()
        );
        let current = workspace.head().expect("current snapshot");
        assert!(current.node(function).is_err());
        assert!(current.contains_tombstone(function.serial()));

        let replacement = Transaction {
            workspace: id,
            base_revision: Revision::new(3),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(4),
                module: NodeTarget::Existing(module),
                name: "replacement".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let replacement = commit(&mut workspace, &replacement).expect("replacement function");
        assert_eq!(replacement.returned_bindings[0].1.serial(), 5);
    }

    #[test]
    fn large_user_controlled_subtree_deletion_uses_an_explicit_work_stack() {
        let id = WorkspaceId::from_bytes([15; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let package = LocalHandle::new(1);
        let module = LocalHandle::new(2);
        let function = LocalHandle::new(3);
        let region = LocalHandle::new(4);
        let block = LocalHandle::new(5);
        let mut operations = vec![
            TransactionOp::CreatePackage {
                handle: package,
                name: "package".to_owned(),
            },
            TransactionOp::CreateModule {
                handle: module,
                package: NodeTarget::Local(package),
                name: "module".to_owned(),
            },
            TransactionOp::CreateFunction {
                handle: function,
                module: NodeTarget::Local(module),
                name: "main".to_owned(),
                result: SemanticType::I64,
            },
            TransactionOp::CreateRegion {
                handle: region,
                function: NodeTarget::Local(function),
            },
            TransactionOp::CreateBlock {
                handle: block,
                region: NodeTarget::Local(region),
            },
        ];
        let first_value = LocalHandle::new(6);
        for offset in 0..10_000_u32 {
            operations.push(TransactionOp::CreateOperation {
                handle: LocalHandle::new(6 + offset),
                block: NodeTarget::Local(block),
                before: None,
                operation: OperationDraft::ConstI64(i64::from(offset)),
            });
        }
        operations.extend([
            TransactionOp::CreateOperation {
                handle: LocalHandle::new(10_006),
                block: NodeTarget::Local(block),
                before: None,
                operation: OperationDraft::Return {
                    value: ValueDraft::OperationResult {
                        operation: NodeTarget::Local(first_value),
                        output: 0,
                    },
                },
            },
            TransactionOp::SetFunctionBody {
                function: NodeTarget::Local(function),
                region: NodeTarget::Local(region),
            },
            TransactionOp::SetEntryFunction {
                package: NodeTarget::Local(package),
                function: NodeTarget::Local(function),
            },
        ]);
        let create = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations,
        };
        let created = commit(&mut workspace, &create).expect("large graph commit");
        let package_id = created.returned_bindings[0].1;
        assert_eq!(workspace.head().expect("head").node_count(), 10_007);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: NodeTarget::Existing(package_id),
            }],
        };
        commit(&mut workspace, &delete).expect("iterative subtree deletion");
        assert_eq!(workspace.head().expect("head").node_count(), 1);
        assert!(
            workspace
                .head()
                .expect("head")
                .contains_tombstone(package_id.serial())
        );
    }

    fn incomplete_program(id: WorkspaceId) -> Transaction {
        let local = NodeTarget::Local;
        let value = |handle| ValueDraft::OperationResult {
            operation: local(handle),
            output: 0,
        };
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".to_owned(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(LocalHandle::new(1)),
                    name: "root".to_owned(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(LocalHandle::new(2)),
                    name: "main".to_owned(),
                    result: SemanticType::I64,
                },
                TransactionOp::CreateRegion {
                    handle: LocalHandle::new(4),
                    function: local(LocalHandle::new(3)),
                },
                TransactionOp::CreateBlock {
                    handle: LocalHandle::new(5),
                    region: local(LocalHandle::new(4)),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(6),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstI64(40),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(7),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstI64(2),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(8),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstBool(true),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(9),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::Hole {
                        expected: SemanticType::I64,
                    },
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(10),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::ConstI64(99),
                },
                TransactionOp::CreateOperation {
                    handle: LocalHandle::new(11),
                    block: local(LocalHandle::new(5)),
                    before: None,
                    operation: OperationDraft::Return {
                        value: value(LocalHandle::new(9)),
                    },
                },
                TransactionOp::SetFunctionBody {
                    function: local(LocalHandle::new(3)),
                    region: local(LocalHandle::new(4)),
                },
                TransactionOp::SetEntryFunction {
                    package: local(LocalHandle::new(1)),
                    function: local(LocalHandle::new(3)),
                },
            ],
        }
    }

    fn binding(receipt: &TransactionReceipt, handle: u32) -> NodeId {
        receipt
            .returned_bindings
            .iter()
            .find_map(|(candidate, node)| (candidate.get() == handle).then_some(*node))
            .expect("selected binding")
    }

    #[test]
    fn response_projection_is_selected_bounded_and_validate_only_is_predictive() {
        let id = WorkspaceId::from_bytes([0x71; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let transaction = create_package_and_module(id);
        let selected = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec {
                return_handles: vec![LocalHandle::new(2)],
            },
        };
        let prepared = workspace
            .prepare_transaction(&selected)
            .expect("selected receipt");
        assert_eq!(prepared.receipt.created_count, 2);
        assert_eq!(prepared.receipt.returned_bindings.len(), 1);
        assert_eq!(prepared.receipt.returned_bindings[0].0, LocalHandle::new(2));

        for return_handles in [
            vec![LocalHandle::new(1), LocalHandle::new(1)],
            vec![LocalHandle::new(2), LocalHandle::new(1)],
            vec![LocalHandle::new(3)],
        ] {
            let invalid = ApplyTransactionRequest {
                transaction: transaction.clone(),
                response: TransactionResponseSpec { return_handles },
            };
            assert_eq!(
                workspace
                    .prepare_transaction(&invalid)
                    .expect_err("invalid response projection")
                    .code,
                ErrorCode::InvalidHandle
            );
        }

        let mut too_many = Vec::new();
        for value in 0..=MAX_RETURNED_BINDINGS {
            too_many.push(LocalHandle::new(u32::try_from(value).expect("handle")));
        }
        let invalid = ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec {
                return_handles: too_many,
            },
        };
        assert_eq!(
            workspace
                .prepare_transaction(&invalid)
                .expect_err("oversized response projection")
                .code,
            ErrorCode::PolicyExceeded
        );

        let mut validate = selected.clone();
        validate.transaction.mode = TransactionMode::ValidateOnly;
        let predicted = workspace
            .prepare_transaction(&validate)
            .expect("validate-only receipt")
            .receipt;
        assert!(!predicted.published);
        let mut commit_request = validate.clone();
        commit_request.transaction.mode = TransactionMode::Commit;
        let committed = workspace
            .prepare_transaction(&commit_request)
            .expect("commit receipt")
            .receipt;
        let mut expected = predicted;
        expected.published = true;
        assert_eq!(committed, expected);

        validate.transaction.idempotency_key = Some(IdempotencyKey::from_bytes([1; 16]));
        assert_eq!(
            workspace
                .prepare_transaction(&validate)
                .expect_err("validate-only idempotency")
                .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn change_digest_includes_exact_scalar_details() {
        let id = WorkspaceId::from_bytes([0x76; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let two = binding(&created, 7);
        let edit = |value| Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceOperation {
                operation: NodeTarget::Existing(two),
                replacement: OperationDraft::ConstI64(value),
            }],
        };
        let three = workspace
            .prepare_transaction(&request(&edit(3)))
            .expect("replace with three")
            .receipt;
        let four = workspace
            .prepare_transaction(&request(&edit(4)))
            .expect("replace with four")
            .receipt;
        assert_eq!(three.change_count, four.change_count);
        assert_ne!(three.change_digest, four.change_digest);
        assert_ne!(three.hash, four.hash);
    }

    #[test]
    fn change_digest_distinguishes_refinement_payloads_and_same_typed_operands() {
        let id = WorkspaceId::from_bytes([0x77; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let forty = binding(&created, 6);
        let two = binding(&created, 7);
        let hole = binding(&created, 9);
        let refinement = |value| Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::ConstI64(value),
            }],
        };
        let two_refinement = workspace
            .prepare_transaction(&request(&refinement(2)))
            .expect("refine to two");
        let three_refinement = workspace
            .prepare_transaction(&request(&refinement(3)))
            .expect("refine to three");
        assert_ne!(two_refinement.receipt.hash, three_refinement.receipt.hash);
        assert_ne!(
            two_refinement.receipt.change_digest,
            three_refinement.receipt.change_digest
        );
        let two_change = diff::between(
            workspace.snapshot(Revision::new(1)).expect("base"),
            &two_refinement.snapshot,
        );
        assert!(two_change.changes.iter().any(|change| {
            matches!(
                &change.kind,
                crate::diff::ChangeKind::OperationRefined {
                    replacement: OperationKind::ConstI64(2),
                    ..
                }
            )
        }));

        let add_refinement = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(two),
                        output: 0,
                    },
                },
            }],
        };
        commit(&mut workspace, &add_refinement).expect("publish add refinement");
        let replacement = |index, operation| Transaction {
            workspace: id,
            base_revision: Revision::new(2),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::ReplaceOperand {
                operation: NodeTarget::Existing(hole),
                index,
                value: ValueDraft::OperationResult {
                    operation: NodeTarget::Existing(operation),
                    output: 0,
                },
            }],
        };
        let replace_left = workspace
            .prepare_transaction(&request(&replacement(0, two)))
            .expect("replace left operand");
        let replace_right = workspace
            .prepare_transaction(&request(&replacement(1, forty)))
            .expect("replace right operand");
        assert_ne!(replace_left.receipt.hash, replace_right.receipt.hash);
        assert_ne!(
            replace_left.receipt.change_digest,
            replace_right.receipt.change_digest
        );
        let left_diff = diff::between(
            workspace.snapshot(Revision::new(2)).expect("refined base"),
            &replace_left.snapshot,
        );
        assert!(left_diff.changes.iter().any(|change| {
            matches!(
                change.kind,
                crate::diff::ChangeKind::OperandChanged {
                    index: 0,
                    before: Some(ValueRef::OperationResult { operation, .. }),
                    after: Some(ValueRef::OperationResult {
                        operation: replacement,
                        ..
                    }),
                } if operation == forty && replacement == two
            )
        }));
    }

    #[test]
    fn create_then_delete_returns_selected_tombstoned_identity_and_explicit_change() {
        let id = WorkspaceId::from_bytes([0x74; 16]);
        let workspace = Workspace::new(id).expect("workspace");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "temporary".to_owned(),
                },
                TransactionOp::DeleteOwnedSubtree {
                    root: NodeTarget::Local(LocalHandle::new(1)),
                },
            ],
        };
        let prepared = workspace
            .prepare_transaction(&request(&transaction))
            .expect("create then delete");
        let allocated = binding(&prepared.receipt, 1);
        assert!(prepared.snapshot.contains_tombstone(allocated.serial()));
        assert!(prepared.receipt.change_count > 0);
        let before = workspace.head().expect("before");
        let semantic_diff = diff::between(before, &prepared.snapshot);
        assert!(semantic_diff.changes.iter().any(|change| {
            change.node == allocated
                && matches!(change.kind, crate::diff::ChangeKind::AllocatedAndTombstoned)
        }));
    }

    #[test]
    fn hole_refinement_preserves_identity_position_use_history_and_diff() {
        let id = WorkspaceId::from_bytes([0x72; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let hole = binding(&created, 9);
        let forty = binding(&created, 6);
        let two = binding(&created, 7);
        let block = binding(&created, 5);
        let return_operation = binding(&created, 11);
        let old = workspace
            .snapshot(Revision::new(1))
            .expect("old snapshot")
            .clone();
        let refine = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(two),
                        output: 0,
                    },
                },
            }],
        };
        let refined = commit(&mut workspace, &refine).expect("refine hole");
        assert_eq!(refined.created_count, 0);
        assert!(!refined.complete_before);
        assert!(refined.complete_after);
        let current = workspace.head().expect("refined snapshot");
        assert!(matches!(
            old.node(hole).expect("old hole"),
            Node::Operation {
                operation: OperationKind::Hole { .. },
                ..
            }
        ));
        assert!(matches!(
            current.node(hole).expect("refined operation"),
            Node::Operation {
                operation: OperationKind::AddI64 { .. },
                ..
            }
        ));
        let Node::Block { operations, .. } = current.node(block).expect("block") else {
            panic!("block kind");
        };
        assert_eq!(operations.iter().position(|id| *id == hole), Some(3));
        let Node::Operation {
            operation: OperationKind::Return { value },
            ..
        } = current.node(return_operation).expect("return")
        else {
            panic!("return kind");
        };
        assert_eq!(
            *value,
            ValueRef::OperationResult {
                operation: hole,
                output: 0,
            }
        );
        let semantic_diff = diff::between(&old, current);
        assert_eq!(semantic_diff.change_count(), refined.change_count);
        assert_eq!(semantic_diff.digest, refined.change_digest);
        assert!(semantic_diff.changes.iter().any(|change| {
            change.node == hole
                && matches!(
                    change.kind,
                    crate::diff::ChangeKind::OperationRefined {
                        before: crate::schema::OperationCode::Hole,
                        after: crate::schema::OperationCode::AddI64,
                        result_type: SemanticType::I64,
                        ..
                    }
                )
        }));
    }

    #[test]
    fn hole_refinement_can_use_supporting_values_created_before_it_atomically() {
        let id = WorkspaceId::from_bytes([0x75; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let block = binding(&created, 5);
        let forty = binding(&created, 6);
        let hole = binding(&created, 9);
        let support = LocalHandle::new(100);
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreateOperation {
                    handle: support,
                    block: NodeTarget::Existing(block),
                    before: Some(NodeTarget::Existing(hole)),
                    operation: OperationDraft::ConstI64(2),
                },
                TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(hole),
                    replacement: OperationDraft::AddI64 {
                        lhs: ValueDraft::OperationResult {
                            operation: NodeTarget::Existing(forty),
                            output: 0,
                        },
                        rhs: ValueDraft::OperationResult {
                            operation: NodeTarget::Local(support),
                            output: 0,
                        },
                    },
                },
            ],
        };
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec {
                    return_handles: vec![support],
                },
            })
            .expect("atomic support and refinement");
        assert_eq!(prepared.receipt.created_count, 1);
        assert!(prepared.receipt.complete_after);
        let support_id = binding(&prepared.receipt, 100);
        let Node::Block { operations, .. } = prepared.snapshot.node(block).expect("block") else {
            panic!("block kind");
        };
        let support_position = operations
            .iter()
            .position(|id| *id == support_id)
            .expect("support position");
        let hole_position = operations
            .iter()
            .position(|id| *id == hole)
            .expect("hole position");
        assert!(support_position < hole_position);
    }

    #[test]
    fn hole_refinement_rejects_wrong_targets_contracts_types_and_order() {
        let id = WorkspaceId::from_bytes([0x73; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let created = commit(&mut workspace, &incomplete_program(id)).expect("incomplete program");
        let package = binding(&created, 1);
        let forty = binding(&created, 6);
        let boolean = binding(&created, 8);
        let hole = binding(&created, 9);
        let later = binding(&created, 10);
        let value = |operation| ValueDraft::OperationResult {
            operation: NodeTarget::Existing(operation),
            output: 0,
        };
        let cases = [
            (package, OperationDraft::ConstI64(1), ErrorCode::WrongKind),
            (
                forty,
                OperationDraft::ConstI64(1),
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::Hole {
                    expected: SemanticType::I64,
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::Return {
                    value: value(forty),
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::ConstBool(false),
                ErrorCode::TypeMismatch,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(boolean),
                },
                ErrorCode::TypeMismatch,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(later),
                },
                ErrorCode::InvalidOperand,
            ),
            (
                hole,
                OperationDraft::AddI64 {
                    lhs: value(forty),
                    rhs: value(hole),
                },
                ErrorCode::InvalidOperand,
            ),
        ];
        for (target, replacement, expected) in cases {
            let refine = Transaction {
                workspace: id,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![TransactionOp::RefineHole {
                    hole: NodeTarget::Existing(target),
                    replacement,
                }],
            };
            assert_eq!(
                workspace
                    .prepare_transaction(&request(&refine))
                    .expect_err("invalid refinement")
                    .code,
                expected
            );
            assert_eq!(workspace.head_revision(), Revision::new(1));
            assert!(matches!(
                workspace.head().expect("head").node(hole).expect("hole"),
                Node::Operation {
                    operation: OperationKind::Hole { .. },
                    ..
                }
            ));
        }
    }

    #[test]
    fn stale_revisions_wrong_workspaces_and_no_changes_reject_deterministically() {
        let id = WorkspaceId::from_bytes([13; 16]);
        let other = WorkspaceId::from_bytes([14; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let package = first.returned_bindings[0].1;

        let stale = create_package_and_module(id);
        assert_eq!(
            workspace
                .prepare_transaction(&request(&stale))
                .expect_err("stale")
                .code,
            ErrorCode::RevisionConflict
        );
        let wrong = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(NodeId::new(other, package.serial()).expect("node")),
                name: "renamed".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&wrong))
                .expect_err("wrong workspace")
                .code,
            ErrorCode::WrongWorkspace
        );
        let no_change = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(package),
                name: "package".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&request(&no_change))
                .expect_err("no change")
                .code,
            ErrorCode::NoChange
        );
    }
}
