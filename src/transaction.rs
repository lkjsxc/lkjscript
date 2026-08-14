use crate::diff::{self, SemanticDiff};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace, require_kind};
use crate::ids::{IdempotencyKey, LocalHandle, NodeId, Revision, SnapshotHash, WorkspaceId};
use crate::schema::{
    Node, NodeKind, OperationDraft, OperationKind, SemanticType, ValueDraft, ValueRef,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NodeTarget {
    Existing(NodeId),
    Local(LocalHandle),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Transaction {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    pub idempotency_key: Option<IdempotencyKey>,
    pub dry_run: bool,
    pub operations: Vec<TransactionOp>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
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
    DeleteOwnedSubtree {
        root: NodeTarget,
    },
}

impl TransactionOp {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransactionResult {
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    pub revision: Revision,
    pub hash: SnapshotHash,
    pub allocations: Vec<(LocalHandle, NodeId)>,
    pub diff: SemanticDiff,
    pub published: bool,
}

#[derive(Debug)]
pub(crate) struct PreparedTransaction {
    pub snapshot: Arc<Snapshot>,
    pub result: TransactionResult,
}

impl Workspace {
    pub(crate) fn prepare_transaction(
        &self,
        transaction: &Transaction,
    ) -> Result<PreparedTransaction> {
        if transaction.workspace != self.id() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "transaction names a different workspace",
            )
            .for_workspace(self.id()));
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
        let result = TransactionResult {
            workspace: self.id(),
            base_revision: transaction.base_revision,
            revision,
            hash: candidate.hash(),
            allocations: allocations.into_iter().collect(),
            diff: semantic_diff,
            published: !transaction.dry_run,
        };
        Ok(PreparedTransaction {
            snapshot: candidate,
            result,
        })
    }
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
            let terminator = operation.contract().terminator;
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
            let current_contract = current.contract();
            let replacement_contract = replacement.contract();
            if current.stable_tag() != replacement.stable_tag()
                || current_contract.result_types != replacement_contract.result_types
                || current_contract.terminator != replacement_contract.terminator
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
        let mut children = node.owned_children();
        children.reverse();
        stack.extend(children);
    }
    for (source, node) in nodes.iter() {
        if deleted.contains(source) {
            continue;
        }
        let blockers: Vec<NodeId> = node
            .direct_references()
            .into_iter()
            .filter(|target| deleted.contains(target))
            .collect();
        if !blockers.is_empty() {
            return Err(LkError::new(
                ErrorCode::DeleteBlocked,
                "surviving node directly references the requested deletion subtree",
            )
            .for_node(root)
            .with_related(std::iter::once(*source).chain(blockers)));
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

    fn commit(workspace: &mut Workspace, transaction: &Transaction) -> Result<TransactionResult> {
        let prepared = workspace.prepare_transaction(transaction)?;
        let result = prepared.result.clone();
        if !transaction.dry_run {
            workspace.publish(prepared.snapshot)?;
        }
        Ok(result)
    }

    fn create_package_and_module(id: WorkspaceId) -> Transaction {
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            dry_run: false,
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
    fn failed_batches_and_dry_runs_do_not_consume_node_ids() {
        let id = WorkspaceId::from_bytes([11; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.allocations[1].1;
        assert_eq!(module.serial(), 3);

        let failed = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: false,
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
            .prepare_transaction(&failed)
            .expect_err("duplicate names must reject");
        assert_eq!(error.code, ErrorCode::DuplicateName);
        assert_eq!(workspace.head_revision(), Revision::new(1));
        assert_eq!(workspace.head().expect("head").next_serial(), 4);

        let dry_run = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: true,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(5),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let predicted = commit(&mut workspace, &dry_run).expect("dry run");
        assert_eq!(predicted.allocations[0].1.serial(), 4);
        assert_eq!(workspace.head_revision(), Revision::new(1));

        let mut real = dry_run;
        real.dry_run = false;
        let committed = commit(&mut workspace, &real).expect("real commit");
        assert_eq!(committed.allocations[0].1, predicted.allocations[0].1);
        assert_eq!(workspace.head_revision(), Revision::new(2));
    }

    #[test]
    fn deletion_tombstones_identity_and_old_snapshots_retain_nodes() {
        let id = WorkspaceId::from_bytes([12; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let module = first.allocations[1].1;
        let create = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: false,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: NodeTarget::Existing(module),
                name: "function".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let created = commit(&mut workspace, &create).expect("create function");
        let function = created.allocations[0].1;
        assert_eq!(function.serial(), 4);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(2),
            idempotency_key: None,
            dry_run: false,
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
            dry_run: false,
            operations: vec![TransactionOp::CreateFunction {
                handle: LocalHandle::new(4),
                module: NodeTarget::Existing(module),
                name: "replacement".to_owned(),
                result: SemanticType::I64,
            }],
        };
        let replacement = commit(&mut workspace, &replacement).expect("replacement function");
        assert_eq!(replacement.allocations[0].1.serial(), 5);
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
            dry_run: false,
            operations,
        };
        let created = commit(&mut workspace, &create).expect("large graph commit");
        let package_id = created.allocations[0].1;
        assert_eq!(workspace.head().expect("head").node_count(), 10_007);

        let delete = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: false,
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

    #[test]
    fn stale_revisions_wrong_workspaces_and_no_changes_reject_deterministically() {
        let id = WorkspaceId::from_bytes([13; 16]);
        let other = WorkspaceId::from_bytes([14; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let first = commit(&mut workspace, &create_package_and_module(id)).expect("first commit");
        let package = first.allocations[0].1;

        let stale = create_package_and_module(id);
        assert_eq!(
            workspace
                .prepare_transaction(&stale)
                .expect_err("stale")
                .code,
            ErrorCode::RevisionConflict
        );
        let wrong = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: false,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(NodeId::new(other, package.serial()).expect("node")),
                name: "renamed".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&wrong)
                .expect_err("wrong workspace")
                .code,
            ErrorCode::WrongWorkspace
        );
        let no_change = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            dry_run: false,
            operations: vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(package),
                name: "package".to_owned(),
            }],
        };
        assert_eq!(
            workspace
                .prepare_transaction(&no_change)
                .expect_err("no change")
                .code,
            ErrorCode::NoChange
        );
    }
}
