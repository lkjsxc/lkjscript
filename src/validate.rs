use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::schema::{Node, NodeKind, OperationKind, SemanticType, ValueRef, owner_kind_is_valid};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_snapshot(snapshot: &Snapshot) -> Result<()> {
    validate_identity(snapshot)?;
    validate_containment(snapshot)?;
    validate_names(snapshot)?;
    validate_bodies(snapshot)?;
    Ok(())
}

fn validate_identity(snapshot: &Snapshot) -> Result<()> {
    if snapshot.next_serial < 2 {
        return Err(corrupt(
            snapshot,
            "allocator state must retain the canonical root allocation",
        ));
    }
    if snapshot.root.serial() != 1 {
        return Err(
            corrupt(snapshot, "workspace root must use canonical serial one")
                .for_node(snapshot.root),
        );
    }
    let live_count = u64::try_from(snapshot.nodes.len()).map_err(|_| {
        corrupt(
            snapshot,
            "live node count overflows allocator representation",
        )
    })?;
    let tombstone_count = u64::try_from(snapshot.tombstones.len()).map_err(|_| {
        corrupt(
            snapshot,
            "tombstone count overflows allocator representation",
        )
    })?;
    let represented = live_count.checked_add(tombstone_count).ok_or_else(|| {
        corrupt(
            snapshot,
            "represented identity count overflows allocator state",
        )
    })?;
    if represented != snapshot.next_serial - 1 {
        return Err(corrupt(
            snapshot,
            "every allocated node serial must be live or tombstoned",
        ));
    }
    for serial in &snapshot.tombstones {
        if *serial == 0 || *serial >= snapshot.next_serial {
            return Err(corrupt(
                snapshot,
                "tombstone is outside allocated identity history",
            ));
        }
        let id = node_id(snapshot, *serial)?;
        if snapshot.nodes.contains_key(&id) {
            return Err(corrupt(snapshot, "live node identity is also tombstoned").for_node(id));
        }
    }
    for id in snapshot.nodes.keys() {
        if id.workspace() != snapshot.workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "snapshot contains a node from another workspace",
            )
            .for_workspace(snapshot.workspace)
            .for_node(*id));
        }
        if id.serial() >= snapshot.next_serial {
            return Err(corrupt(snapshot, "live node is beyond allocator state").for_node(*id));
        }
    }
    let root = snapshot
        .nodes
        .get(&snapshot.root)
        .ok_or_else(|| corrupt(snapshot, "snapshot root does not exist").for_node(snapshot.root))?;
    if root.kind() != NodeKind::WorkspaceRoot {
        return Err(corrupt(snapshot, "snapshot root has the wrong kind")
            .for_node(snapshot.root)
            .with_kinds(NodeKind::WorkspaceRoot, root.kind()));
    }
    let root_count = snapshot
        .nodes
        .values()
        .filter(|node| node.kind() == NodeKind::WorkspaceRoot)
        .count();
    if root_count != 1 {
        return Err(corrupt(
            snapshot,
            "snapshot must contain exactly one workspace root",
        ));
    }
    Ok(())
}

fn validate_containment(snapshot: &Snapshot) -> Result<()> {
    let mut owner_counts = BTreeMap::<NodeId, usize>::new();
    for (owner_id, owner) in &snapshot.nodes {
        let mut local = BTreeSet::new();
        for child_id in owner.owned_children() {
            if !local.insert(child_id) {
                return Err(corrupt(snapshot, "owned child appears more than once")
                    .for_node(child_id)
                    .with_related([*owner_id]));
            }
            let child = snapshot.nodes.get(&child_id).ok_or_else(|| {
                corrupt(snapshot, "owned child does not exist")
                    .for_node(child_id)
                    .with_related([*owner_id])
            })?;
            if child_id.workspace() != snapshot.workspace {
                return Err(LkError::new(
                    ErrorCode::WrongWorkspace,
                    "containment target belongs to another workspace",
                )
                .for_workspace(snapshot.workspace)
                .for_node(child_id));
            }
            if !owner_kind_is_valid(child.kind(), owner.kind()) {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "child kind is not permitted in this owner slot",
                )
                .for_node(child_id)
                .with_kinds(owner.kind(), child.kind())
                .with_related([*owner_id]));
            }
            if child.owner() != Some(*owner_id) {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "child owner field disagrees with containment",
                )
                .for_node(child_id)
                .with_related([*owner_id]));
            }
            *owner_counts.entry(child_id).or_default() += 1;
        }
        validate_slot_targets(snapshot, *owner_id, owner)?;
        for reference in owner.direct_references() {
            if reference.workspace() != snapshot.workspace {
                return Err(LkError::new(
                    ErrorCode::WrongWorkspace,
                    "direct reference belongs to another workspace",
                )
                .for_workspace(snapshot.workspace)
                .for_node(reference)
                .with_related([*owner_id]));
            }
            if !snapshot.nodes.contains_key(&reference) {
                return Err(LkError::new(
                    ErrorCode::NodeNotFound,
                    "direct reference target does not exist",
                )
                .for_node(reference)
                .with_related([*owner_id]));
            }
        }
    }

    for (id, node) in &snapshot.nodes {
        let count = owner_counts.get(id).copied().unwrap_or(0);
        if *id == snapshot.root {
            if count != 0 || node.owner().is_some() {
                return Err(corrupt(snapshot, "workspace root cannot be owned").for_node(*id));
            }
        } else if count != 1 {
            return Err(
                corrupt(snapshot, "every non-root node must have exactly one owner").for_node(*id),
            );
        }
    }

    let mut visited = BTreeSet::new();
    let mut stack = vec![snapshot.root];
    while let Some(id) = stack.pop() {
        if !visited.insert(id) {
            continue;
        }
        let node = snapshot.nodes.get(&id).ok_or_else(|| {
            corrupt(snapshot, "containment traversal reached a missing node").for_node(id)
        })?;
        let mut children = node.owned_children();
        children.reverse();
        stack.extend(children);
    }
    if visited.len() != snapshot.nodes.len() {
        let unowned = snapshot
            .nodes
            .keys()
            .find(|id| !visited.contains(id))
            .copied();
        let mut error = corrupt(snapshot, "snapshot contains an unreachable semantic node");
        if let Some(unowned) = unowned {
            error = error.for_node(unowned);
        }
        return Err(error);
    }
    Ok(())
}

fn validate_slot_targets(snapshot: &Snapshot, id: NodeId, node: &Node) -> Result<()> {
    match node {
        Node::WorkspaceRoot { packages } => {
            require_children(snapshot, id, packages, NodeKind::Package)?;
        }
        Node::Package { modules, entry, .. } => {
            require_children(snapshot, id, modules, NodeKind::Module)?;
            if let Some(entry) = entry {
                let function = require_kind(snapshot, *entry, NodeKind::Function, id)?;
                let module = function.owner().ok_or_else(|| {
                    corrupt(snapshot, "entry function has no module owner").for_node(*entry)
                })?;
                let module_node = require_kind(snapshot, module, NodeKind::Module, *entry)?;
                if module_node.owner() != Some(id) {
                    return Err(LkError::new(
                        ErrorCode::OwnerMismatch,
                        "entry function is not contained by the package",
                    )
                    .for_node(*entry)
                    .with_related([id]));
                }
            }
        }
        Node::Module { functions, .. } => {
            require_children(snapshot, id, functions, NodeKind::Function)?;
        }
        Node::Function {
            parameters, body, ..
        } => {
            require_children(snapshot, id, parameters, NodeKind::Parameter)?;
            if let Some(body) = body {
                require_kind(snapshot, *body, NodeKind::Region, id)?;
            }
        }
        Node::Parameter { .. } | Node::Operation { .. } => {}
        Node::Region { blocks, .. } => {
            if blocks.len() != 1 {
                return Err(corrupt(
                    snapshot,
                    "bootstrap function regions must contain exactly one block",
                )
                .for_node(id));
            }
            require_children(snapshot, id, blocks, NodeKind::Block)?;
        }
        Node::Block {
            operations,
            terminator,
            ..
        } => {
            require_children(snapshot, id, operations, NodeKind::Operation)?;
            let terminator = terminator.ok_or_else(|| {
                corrupt(snapshot, "block must own exactly one terminator").for_node(id)
            })?;
            require_kind(snapshot, terminator, NodeKind::Operation, id)?;
            if operations.contains(&terminator) {
                return Err(
                    corrupt(snapshot, "terminator also appears as a regular operation")
                        .for_node(terminator),
                );
            }
        }
    }
    Ok(())
}

fn require_children(
    snapshot: &Snapshot,
    owner: NodeId,
    children: &[NodeId],
    expected: NodeKind,
) -> Result<()> {
    for child in children {
        require_kind(snapshot, *child, expected, owner)?;
    }
    Ok(())
}

fn require_kind(
    snapshot: &Snapshot,
    target: NodeId,
    expected: NodeKind,
    related: NodeId,
) -> Result<&Node> {
    let node = snapshot.nodes.get(&target).ok_or_else(|| {
        LkError::new(ErrorCode::NodeNotFound, "slot target does not exist")
            .for_node(target)
            .with_related([related])
    })?;
    if node.kind() != expected {
        return Err(
            LkError::new(ErrorCode::WrongKind, "slot target has the wrong kind")
                .for_node(target)
                .with_kinds(expected, node.kind())
                .with_related([related]),
        );
    }
    Ok(node)
}

fn validate_names(snapshot: &Snapshot) -> Result<()> {
    for (owner_id, owner) in &snapshot.nodes {
        let named_children: Vec<NodeId> = match owner {
            Node::WorkspaceRoot { packages } => packages.clone(),
            Node::Package { modules, .. } => modules.clone(),
            Node::Module { functions, .. } => functions.clone(),
            Node::Function { parameters, .. } => parameters.clone(),
            _ => Vec::new(),
        };
        let mut names = BTreeMap::<&str, NodeId>::new();
        for child_id in named_children {
            let child = snapshot
                .nodes
                .get(&child_id)
                .ok_or_else(|| corrupt(snapshot, "named child is missing").for_node(child_id))?;
            let name = child.name().ok_or_else(|| {
                corrupt(snapshot, "named containment slot contains an unnamed node")
                    .for_node(child_id)
            })?;
            if name.is_empty() {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "display names must not be empty",
                )
                .for_node(child_id));
            }
            if let Some(previous) = names.insert(name, child_id) {
                return Err(LkError::new(
                    ErrorCode::DuplicateName,
                    "sibling lookup names must be unique",
                )
                .for_node(child_id)
                .with_related([*owner_id, previous]));
            }
        }
    }
    Ok(())
}

fn validate_bodies(snapshot: &Snapshot) -> Result<()> {
    for (function_id, node) in &snapshot.nodes {
        let Node::Function {
            parameters,
            result,
            body,
            ..
        } = node
        else {
            continue;
        };
        for (expected, parameter_id) in parameters.iter().enumerate() {
            let parameter = snapshot.nodes.get(parameter_id).ok_or_else(|| {
                corrupt(snapshot, "function parameter is missing").for_node(*parameter_id)
            })?;
            let Node::Parameter { ordinal, .. } = parameter else {
                return Err(corrupt(snapshot, "function parameter slot has wrong kind")
                    .for_node(*parameter_id));
            };
            let expected = u32::try_from(expected).map_err(|_| {
                corrupt(
                    snapshot,
                    "parameter ordinal does not fit protocol representation",
                )
                .for_node(*parameter_id)
            })?;
            if *ordinal != expected {
                return Err(
                    corrupt(snapshot, "parameter ordinals must be dense and ordered")
                        .for_node(*parameter_id),
                );
            }
        }
        let Some(body) = body else {
            continue;
        };
        let region = snapshot
            .nodes
            .get(body)
            .ok_or_else(|| corrupt(snapshot, "function body region is missing").for_node(*body))?;
        let Node::Region { blocks, .. } = region else {
            return Err(corrupt(snapshot, "function body has wrong kind").for_node(*body));
        };
        for block in blocks {
            validate_block(snapshot, *function_id, *result, *block)?;
        }
    }
    Ok(())
}

fn validate_block(
    snapshot: &Snapshot,
    function: NodeId,
    function_result: SemanticType,
    block_id: NodeId,
) -> Result<()> {
    let block = snapshot
        .nodes
        .get(&block_id)
        .ok_or_else(|| corrupt(snapshot, "function block is missing").for_node(block_id))?;
    let Node::Block {
        operations,
        terminator,
        ..
    } = block
    else {
        return Err(corrupt(snapshot, "region contains a non-block node").for_node(block_id));
    };
    let terminator = terminator
        .ok_or_else(|| corrupt(snapshot, "block must contain a terminator").for_node(block_id))?;
    let mut positions = BTreeMap::new();
    for (position, operation) in operations.iter().enumerate() {
        positions.insert(*operation, position);
    }
    for (position, operation_id) in operations.iter().enumerate() {
        validate_operation(
            snapshot,
            function,
            function_result,
            block_id,
            *operation_id,
            position,
            &positions,
            false,
        )?;
    }
    validate_operation(
        snapshot,
        function,
        function_result,
        block_id,
        terminator,
        operations.len(),
        &positions,
        true,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn validate_operation(
    snapshot: &Snapshot,
    function: NodeId,
    function_result: SemanticType,
    block: NodeId,
    operation_id: NodeId,
    position: usize,
    positions: &BTreeMap<NodeId, usize>,
    is_terminator: bool,
) -> Result<()> {
    let node = snapshot
        .nodes
        .get(&operation_id)
        .ok_or_else(|| corrupt(snapshot, "block operation is missing").for_node(operation_id))?;
    let Node::Operation { operation, .. } = node else {
        return Err(corrupt(snapshot, "block contains a non-operation node").for_node(operation_id));
    };
    let contract = operation.contract();
    if contract.terminator != is_terminator {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "operation termination contract disagrees with its block slot",
        )
        .for_node(operation_id));
    }
    match operation {
        OperationKind::Return { value } => {
            let actual = value_type(snapshot, function, block, position, positions, *value)?;
            if actual != function_result {
                return Err(type_error(operation_id, function_result, actual, *value));
            }
        }
        _ => {
            let operands = operation.operands();
            if operands.len() != contract.operand_types.len() {
                return Err(
                    corrupt(snapshot, "operation contract operand arity disagrees")
                        .for_node(operation_id),
                );
            }
            for (operand, expected) in operands.into_iter().zip(contract.operand_types) {
                let actual = value_type(snapshot, function, block, position, positions, operand)?;
                if actual != expected {
                    return Err(type_error(operation_id, expected, actual, operand));
                }
            }
        }
    }
    Ok(())
}

fn value_type(
    snapshot: &Snapshot,
    function: NodeId,
    block: NodeId,
    use_position: usize,
    positions: &BTreeMap<NodeId, usize>,
    value: ValueRef,
) -> Result<SemanticType> {
    let referenced = value.referenced_node();
    if referenced.workspace() != snapshot.workspace {
        return Err(LkError::new(
            ErrorCode::WrongWorkspace,
            "operand value belongs to another workspace",
        )
        .for_workspace(snapshot.workspace)
        .for_node(referenced));
    }
    match value {
        ValueRef::FunctionParameter(parameter) => {
            let node = snapshot.nodes.get(&parameter).ok_or_else(|| {
                LkError::new(ErrorCode::NodeNotFound, "parameter value does not exist")
                    .for_node(parameter)
            })?;
            let Node::Parameter { owner, ty, .. } = node else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "parameter value must reference a parameter node",
                )
                .for_node(parameter)
                .with_kinds(NodeKind::Parameter, node.kind()));
            };
            if *owner != function {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "parameter value is outside the owning function",
                )
                .for_node(parameter)
                .with_related([function]));
            }
            Ok(*ty)
        }
        ValueRef::OperationResult { operation, output } => {
            let producer_position = positions.get(&operation).ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result is outside the current block",
                )
                .for_node(operation)
                .with_related([block])
            })?;
            if *producer_position >= use_position {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result must be produced before its use",
                )
                .for_node(operation)
                .with_related([block]));
            }
            let producer = snapshot.nodes.get(&operation).ok_or_else(|| {
                LkError::new(ErrorCode::NodeNotFound, "operand producer does not exist")
                    .for_node(operation)
            })?;
            let Node::Operation {
                owner,
                operation: producer_kind,
            } = producer
            else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "operation result must reference an operation node",
                )
                .for_node(operation)
                .with_kinds(NodeKind::Operation, producer.kind()));
            };
            if *owner != block {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result belongs to another block",
                )
                .for_node(operation)
                .with_related([block]));
            }
            producer_kind
                .contract()
                .result_types
                .get(usize::from(output))
                .copied()
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "operation result index is outside the operation contract",
                    )
                    .for_node(operation)
                })
        }
    }
}

fn type_error(
    operation: NodeId,
    expected: SemanticType,
    actual: SemanticType,
    value: ValueRef,
) -> LkError {
    LkError::new(
        ErrorCode::TypeMismatch,
        "operand type does not match operation contract",
    )
    .for_node(operation)
    .with_types(expected, actual)
    .with_related([value.referenced_node()])
}

fn node_id(snapshot: &Snapshot, serial: u64) -> Result<NodeId> {
    NodeId::new(snapshot.workspace, serial).map_err(|error| corrupt(snapshot, &error.to_string()))
}

fn corrupt(snapshot: &Snapshot, message: &str) -> LkError {
    LkError::new(ErrorCode::InvalidContainment, message)
        .for_workspace(snapshot.workspace)
        .at_revision(snapshot.revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Snapshot;
    use crate::ids::{Revision, WorkspaceId};

    #[test]
    fn initial_snapshot_has_one_valid_root() {
        let snapshot =
            Snapshot::initial(WorkspaceId::from_bytes([7; 16])).expect("initial graph is valid");
        assert_eq!(snapshot.node_count(), 1);
        assert!(validate_snapshot(&snapshot).is_ok());
    }

    #[test]
    fn allocator_gaps_and_noncanonical_roots_reject() {
        let workspace = WorkspaceId::from_bytes([8; 16]);
        let root = NodeId::new(workspace, 1).expect("root");
        let nodes = BTreeMap::from([(
            root,
            Node::WorkspaceRoot {
                packages: Vec::new(),
            },
        )]);
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::INITIAL,
                root,
                3,
                BTreeSet::new(),
                nodes,
            )
            .expect_err("allocator gap")
            .code,
            ErrorCode::InvalidContainment
        );
    }
}
