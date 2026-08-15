use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, operation_result_type};
use crate::ids::NodeId;
use crate::schema::{
    Node, NodeKind, OperationKind, SemanticType, TypeRule, ValueRef, owner_kind_is_valid,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_snapshot(snapshot: &Snapshot) -> Result<()> {
    validate_identity(snapshot)?;
    validate_containment(snapshot)?;
    validate_names(snapshot)?;
    validate_semantics(snapshot)?;
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
    if snapshot
        .nodes
        .values()
        .filter(|node| node.kind() == NodeKind::WorkspaceRoot)
        .count()
        != 1
    {
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
        for index in 0..owner.owned_child_count() {
            let child_id = owner.owned_child(index).ok_or_else(|| {
                corrupt(snapshot, "owned-child accessor disagrees with its count")
                    .for_node(*owner_id)
            })?;
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
        for index in 0..owner.direct_reference_count() {
            let reference = owner.direct_reference(index).ok_or_else(|| {
                corrupt(
                    snapshot,
                    "direct-reference accessor disagrees with its count",
                )
                .for_node(*owner_id)
            })?;
            let target = reference.target();
            if target.workspace() != snapshot.workspace {
                return Err(LkError::new(
                    ErrorCode::WrongWorkspace,
                    "direct reference belongs to another workspace",
                )
                .for_workspace(snapshot.workspace)
                .for_node(target)
                .with_related([*owner_id]));
            }
            if !snapshot.nodes.contains_key(&target) {
                return Err(LkError::new(
                    ErrorCode::NodeNotFound,
                    "direct reference target does not exist",
                )
                .for_node(target)
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
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                stack.push(child);
            }
        }
    }
    if visited.len() != snapshot.nodes.len() {
        let mut error = corrupt(snapshot, "snapshot contains an unreachable semantic node");
        if let Some(id) = snapshot.nodes.keys().find(|id| !visited.contains(id)) {
            error = error.for_node(*id);
        }
        return Err(error);
    }
    Ok(())
}

fn validate_slot_targets(snapshot: &Snapshot, id: NodeId, node: &Node) -> Result<()> {
    match node {
        Node::WorkspaceRoot { packages } => {
            require_children(snapshot, id, packages, NodeKind::Package)?
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
            require_children(snapshot, id, functions, NodeKind::Function)?
        }
        Node::Function {
            parameters, body, ..
        } => {
            require_children(snapshot, id, parameters, NodeKind::Parameter)?;
            if let Some(body) = body {
                require_kind(snapshot, *body, NodeKind::Region, id)?;
            }
        }
        Node::Parameter { .. } | Node::BlockArgument { .. } => {}
        Node::Region { blocks, .. } => {
            if blocks.len() != 1 {
                return Err(
                    corrupt(snapshot, "semantic regions must contain exactly one block")
                        .for_node(id),
                );
            }
            require_children(snapshot, id, blocks, NodeKind::Block)?;
        }
        Node::Block {
            arguments,
            operations,
            terminator,
            ..
        } => {
            require_children(snapshot, id, arguments, NodeKind::BlockArgument)?;
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
        Node::Operation { operation, .. } => {
            let descriptor = operation.descriptor();
            if operation.owned_region_count() != descriptor.regions.len() {
                return Err(corrupt(
                    snapshot,
                    "operation owned-region accessor disagrees with its descriptor",
                )
                .for_node(id));
            }
            for index in 0..operation.owned_region_count() {
                let region = operation.owned_region(index).ok_or_else(|| {
                    corrupt(snapshot, "operation region slot is missing").for_node(id)
                })?;
                require_kind(snapshot, region, NodeKind::Region, id)?;
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
        let named_children: &[NodeId] = match owner {
            Node::WorkspaceRoot { packages } => packages,
            Node::Package { modules, .. } => modules,
            Node::Module { functions, .. } => functions,
            Node::Function { parameters, .. } => parameters,
            _ => &[],
        };
        let mut names = BTreeMap::<&str, NodeId>::new();
        for child_id in named_children {
            let child = snapshot
                .nodes
                .get(child_id)
                .ok_or_else(|| corrupt(snapshot, "named child is missing").for_node(*child_id))?;
            let name = child.name().ok_or_else(|| {
                corrupt(snapshot, "named slot contains an unnamed node").for_node(*child_id)
            })?;
            if name.is_empty() {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "display names must not be empty",
                )
                .for_node(*child_id));
            }
            if let Some(previous) = names.insert(name, *child_id) {
                return Err(LkError::new(
                    ErrorCode::DuplicateName,
                    "sibling lookup names must be unique",
                )
                .for_node(*child_id)
                .with_related([*owner_id, previous]));
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct RegionContract {
    function: NodeId,
    expected_arguments: [Option<SemanticType>; 2],
    argument_count: usize,
    terminator: crate::schema::OperationCode,
    yielded: SemanticType,
}

fn validate_semantics(snapshot: &Snapshot) -> Result<()> {
    for (function_id, node) in &snapshot.nodes {
        let Node::Function {
            parameters,
            result: _,
            body: _,
            ..
        } = node
        else {
            continue;
        };
        for (expected, parameter_id) in parameters.iter().enumerate() {
            let Node::Parameter { owner, ordinal, .. } = snapshot.node(*parameter_id)? else {
                return Err(corrupt(snapshot, "function parameter slot has wrong kind")
                    .for_node(*parameter_id));
            };
            let expected = u32::try_from(expected).map_err(|_| {
                corrupt(snapshot, "parameter ordinal overflows representation")
                    .for_node(*parameter_id)
            })?;
            if *owner != *function_id || *ordinal != expected {
                return Err(corrupt(
                    snapshot,
                    "parameter owner and ordinals must be dense and ordered",
                )
                .for_node(*parameter_id));
            }
        }
    }

    for (region_id, node) in &snapshot.nodes {
        if !matches!(node, Node::Region { .. }) {
            continue;
        }
        let contract = region_contract(snapshot, *region_id)?;
        let Node::Region { blocks, .. } = node else {
            unreachable!()
        };
        let block_id = blocks[0];
        validate_block(snapshot, *region_id, block_id, contract)?;
    }
    Ok(())
}

fn region_contract(snapshot: &Snapshot, region_id: NodeId) -> Result<RegionContract> {
    let Node::Region { owner, .. } = snapshot.node(region_id)? else {
        return Err(corrupt(snapshot, "region contract target is not a region").for_node(region_id));
    };
    match snapshot.node(*owner)? {
        Node::Function { result, body, .. } => {
            if *body != Some(region_id) {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "function-owned region is not its body slot",
                )
                .for_node(region_id)
                .with_related([*owner]));
            }
            Ok(RegionContract {
                function: *owner,
                expected_arguments: [None, None],
                argument_count: 0,
                terminator: crate::schema::OperationCode::Return,
                yielded: *result,
            })
        }
        Node::Operation {
            owner: parent_block,
            operation,
        } => {
            let role_index = (0..operation.owned_region_count())
                .find(|index| operation.owned_region(*index) == Some(region_id))
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::OwnerMismatch,
                        "operation-owned region is absent from its closed region slots",
                    )
                    .for_node(region_id)
                    .with_related([*owner])
                })?;
            let descriptor = &operation.descriptor().regions[role_index];
            let function = owner_function_for_block(snapshot, *parent_block)?;
            let mut expected_arguments = [None, None];
            for (index, argument) in descriptor.block_arguments.iter().enumerate() {
                expected_arguments[index] =
                    resolve_type_rule(snapshot, operation, argument.ty, function, Some(region_id))?;
            }
            let yielded = resolve_type_rule(
                snapshot,
                operation,
                descriptor.yield_type,
                function,
                Some(region_id),
            )?
            .ok_or_else(|| {
                corrupt(snapshot, "region yield type rule cannot be resolved").for_node(region_id)
            })?;
            Ok(RegionContract {
                function,
                expected_arguments,
                argument_count: descriptor.block_arguments.len(),
                terminator: descriptor.terminator,
                yielded,
            })
        }
        other => Err(LkError::new(
            ErrorCode::OwnerMismatch,
            "region owner must be a function or structured operation",
        )
        .for_node(region_id)
        .with_kinds(NodeKind::Operation, other.kind())
        .with_related([*owner])),
    }
}

fn validate_block(
    snapshot: &Snapshot,
    region_id: NodeId,
    block_id: NodeId,
    contract: RegionContract,
) -> Result<()> {
    let Node::Block {
        arguments,
        operations,
        terminator,
        ..
    } = snapshot.node(block_id)?
    else {
        return Err(corrupt(snapshot, "region child is not a block").for_node(block_id));
    };
    if arguments.len() != contract.argument_count {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "block argument count does not match its region role",
        )
        .for_node(block_id)
        .with_related([region_id]));
    }
    for (index, argument_id) in arguments.iter().enumerate() {
        let Node::BlockArgument { owner, ordinal, ty } = snapshot.node(*argument_id)? else {
            return Err(
                corrupt(snapshot, "block argument slot has wrong kind").for_node(*argument_id)
            );
        };
        let expected_ordinal = u32::try_from(index).map_err(|_| {
            corrupt(snapshot, "block argument ordinal overflows representation")
                .for_node(*argument_id)
        })?;
        let expected = contract.expected_arguments[index].ok_or_else(|| {
            corrupt(snapshot, "block argument type contract is absent").for_node(*argument_id)
        })?;
        if *owner != block_id || *ordinal != expected_ordinal {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "block argument owner and ordinals must be dense and ordered",
            )
            .for_node(*argument_id)
            .with_related([block_id]));
        }
        if *ty != expected {
            return Err(type_error(
                *argument_id,
                expected,
                *ty,
                ValueRef::BlockArgument(*argument_id),
            ));
        }
    }
    let terminator_id = terminator
        .ok_or_else(|| corrupt(snapshot, "block terminator is absent").for_node(block_id))?;
    let mut positions = BTreeMap::new();
    for (position, operation) in operations.iter().enumerate() {
        positions.insert(*operation, position);
    }
    for (position, operation_id) in operations.iter().enumerate() {
        validate_operation(
            snapshot,
            contract.function,
            block_id,
            *operation_id,
            position,
            &positions,
            None,
        )?;
    }
    validate_operation(
        snapshot,
        contract.function,
        block_id,
        terminator_id,
        operations.len(),
        &positions,
        Some((contract.terminator, contract.yielded)),
    )?;
    Ok(())
}

fn validate_operation(
    snapshot: &Snapshot,
    function: NodeId,
    block: NodeId,
    operation_id: NodeId,
    position: usize,
    positions: &BTreeMap<NodeId, usize>,
    terminator_contract: Option<(crate::schema::OperationCode, SemanticType)>,
) -> Result<()> {
    let Node::Operation { owner, operation } = snapshot.node(operation_id)? else {
        return Err(corrupt(snapshot, "block contains a non-operation node").for_node(operation_id));
    };
    if *owner != block {
        return Err(LkError::new(
            ErrorCode::OwnerMismatch,
            "operation owner disagrees with block slot",
        )
        .for_node(operation_id)
        .with_related([block]));
    }
    match terminator_contract {
        Some((code, _)) if !operation.is_terminator() || operation.code() != code => {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "terminator kind does not match the owning region contract",
            )
            .for_node(operation_id));
        }
        None if operation.is_terminator() => {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "terminator appears in a regular operation slot",
            )
            .for_node(operation_id));
        }
        _ => {}
    }
    if let OperationKind::ForI64 { step, .. } = operation
        && *step <= 0
    {
        return Err(LkError::new(
            ErrorCode::InvalidOperand,
            "for_i64 step must be positive and nonzero",
        )
        .for_node(operation_id));
    }
    let expected_types = expected_operand_types(
        snapshot,
        function,
        operation_id,
        operation,
        terminator_contract.map(|(_, ty)| ty),
    )?;
    if expected_types.len() != operation.operand_count() {
        return Err(LkError::new(
            ErrorCode::InvalidOperand,
            "operation operand count does not match its context-dependent contract",
        )
        .for_node(operation_id));
    }
    for (index, expected) in expected_types.into_iter().enumerate() {
        let operand = operation.operand(index).ok_or_else(|| {
            corrupt(
                snapshot,
                "operation operand accessor disagrees with its count",
            )
            .for_node(operation_id)
        })?;
        let actual = value_type_at_use(snapshot, function, block, position, positions, operand)?;
        if actual != expected {
            return Err(type_error(operation_id, expected, actual, operand));
        }
    }
    for index in 0..operation.result_count() {
        if operation_result_type(snapshot, operation_id, operation, index).is_none() {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "operation result type cannot be resolved from its exact contract",
            )
            .for_node(operation_id));
        }
    }
    Ok(())
}

fn expected_operand_types(
    snapshot: &Snapshot,
    function: NodeId,
    operation_id: NodeId,
    operation: &OperationKind,
    region_yield: Option<SemanticType>,
) -> Result<Vec<SemanticType>> {
    let function_result = match snapshot.node(function)? {
        Node::Function { result, .. } => *result,
        _ => {
            return Err(
                corrupt(snapshot, "operation function context is not a function")
                    .for_node(function),
            );
        }
    };
    match operation {
        OperationKind::Call {
            function: target,
            arguments: _,
        } => match snapshot.node(*target)? {
            Node::Function { parameters, .. } => parameters
                .iter()
                .map(|parameter| match snapshot.node(*parameter) {
                    Ok(Node::Parameter { ty, .. }) => Ok(*ty),
                    Ok(node) => Err(LkError::new(
                        ErrorCode::WrongKind,
                        "call target parameter slot has wrong kind",
                    )
                    .for_node(*parameter)
                    .with_kinds(NodeKind::Parameter, node.kind())),
                    Err(error) => Err(error),
                })
                .collect(),
            node => Err(LkError::new(
                ErrorCode::WrongKind,
                "call target must be a function identity",
            )
            .for_node(*target)
            .with_kinds(NodeKind::Function, node.kind())
            .with_related([operation_id])),
        },
        _ => (0..operation.operand_count())
            .map(|index| {
                let rule = operation
                    .descriptor()
                    .operands
                    .get(index)
                    .map(|operand| operand.ty)
                    .ok_or_else(|| {
                        corrupt(snapshot, "fixed operand descriptor is incomplete")
                            .for_node(operation_id)
                    })?;
                resolve_type_rule(snapshot, operation, rule, function, None)?
                    .or(match rule {
                        TypeRule::OwnerFunctionResult => Some(function_result),
                        TypeRule::OwningRegionYield => region_yield,
                        _ => None,
                    })
                    .ok_or_else(|| {
                        corrupt(snapshot, "operand type rule cannot be resolved")
                            .for_node(operation_id)
                    })
            })
            .collect(),
    }
}

fn resolve_type_rule(
    _snapshot: &Snapshot,
    operation: &OperationKind,
    rule: TypeRule,
    _function: NodeId,
    _region: Option<NodeId>,
) -> Result<Option<SemanticType>> {
    Ok(match rule {
        TypeRule::Fixed(ty) => Some(ty),
        TypeRule::PayloadExpected => match operation {
            OperationKind::Hole { expected } => Some(*expected),
            _ => None,
        },
        TypeRule::PayloadResult => match operation {
            OperationKind::If { result, .. } => Some(*result),
            _ => None,
        },
        TypeRule::PayloadCarried => match operation {
            OperationKind::ForI64 { carried, .. } => Some(*carried),
            _ => None,
        },
        TypeRule::OwnerFunctionResult
        | TypeRule::CallTargetParameter
        | TypeRule::CallTargetResult
        | TypeRule::OwningRegionYield => None,
    })
}

fn value_type_at_use(
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
    let limits = lexical_block_limits(snapshot, function, block, use_position)?;
    match value {
        ValueRef::FunctionParameter(parameter) => match snapshot.node(parameter)? {
            Node::Parameter { owner, ty, .. } if *owner == function => Ok(*ty),
            Node::Parameter { .. } => Err(LkError::new(
                ErrorCode::InvalidOperand,
                "function parameter is outside the owning function",
            )
            .for_node(parameter)
            .with_related([function])),
            node => Err(LkError::new(
                ErrorCode::WrongKind,
                "function parameter value must reference a parameter node",
            )
            .for_node(parameter)
            .with_kinds(NodeKind::Parameter, node.kind())),
        },
        ValueRef::BlockArgument(argument) => match snapshot.node(argument)? {
            Node::BlockArgument { owner, ty, .. } if limits.contains_key(owner) => Ok(*ty),
            Node::BlockArgument { owner, .. } => Err(LkError::new(
                ErrorCode::InvalidOperand,
                "block argument is not lexically visible at this use",
            )
            .for_node(argument)
            .with_related([*owner, block])),
            node => Err(LkError::new(
                ErrorCode::WrongKind,
                "block argument value must reference a block_argument node",
            )
            .for_node(argument)
            .with_kinds(NodeKind::BlockArgument, node.kind())),
        },
        ValueRef::OperationResult { operation, output } => {
            let Node::Operation {
                owner: producer_block,
                operation: producer,
            } = snapshot.node(operation)?
            else {
                let node = snapshot.node(operation)?;
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "operation result must reference an operation node",
                )
                .for_node(operation)
                .with_kinds(NodeKind::Operation, node.kind()));
            };
            let limit = limits.get(producer_block).ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result is not lexically visible at this use",
                )
                .for_node(operation)
                .with_related([block])
            })?;
            let producer_position = if *producer_block == block {
                positions.get(&operation).copied()
            } else {
                operation_position(snapshot, *producer_block, operation)
            }
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidContainment,
                    "operation result producer is not in its owner block regular-operation slot",
                )
                .for_node(operation)
            })?;
            if producer_position >= *limit {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result must be produced before its lexical use",
                )
                .for_node(operation)
                .with_related([*producer_block, block]));
            }
            operation_result_type(snapshot, operation, producer, usize::from(output)).ok_or_else(
                || {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "operation result index or dynamic result contract is invalid",
                    )
                    .for_node(operation)
                },
            )
        }
    }
}

fn lexical_block_limits(
    snapshot: &Snapshot,
    function: NodeId,
    block: NodeId,
    use_position: usize,
) -> Result<BTreeMap<NodeId, usize>> {
    let mut limits = BTreeMap::from([(block, use_position)]);
    let mut current = block;
    loop {
        let Node::Block { owner: region, .. } = snapshot.node(current)? else {
            return Err(corrupt(snapshot, "lexical context contains a non-block").for_node(current));
        };
        let Node::Region { owner, .. } = snapshot.node(*region)? else {
            return Err(corrupt(snapshot, "block owner is not a region").for_node(*region));
        };
        match snapshot.node(*owner)? {
            Node::Function { .. } => {
                if *owner != function {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "lexical context escaped its owning function",
                    )
                    .for_node(block)
                    .with_related([function, *owner]));
                }
                break;
            }
            Node::Operation {
                owner: parent_block,
                ..
            } => {
                let position = operation_position(snapshot, *parent_block, *owner).ok_or_else(|| {
                    LkError::new(ErrorCode::InvalidContainment, "structured region owner is not a regular operation in its parent block").for_node(*owner).with_related([*parent_block])
                })?;
                limits.insert(*parent_block, position);
                current = *parent_block;
            }
            node => {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "region owner has the wrong kind",
                )
                .for_node(*region)
                .with_kinds(NodeKind::Operation, node.kind()));
            }
        }
    }
    Ok(limits)
}

fn operation_position(snapshot: &Snapshot, block: NodeId, operation: NodeId) -> Option<usize> {
    match snapshot.nodes.get(&block) {
        Some(Node::Block { operations, .. }) => operations
            .iter()
            .position(|candidate| *candidate == operation),
        _ => None,
    }
}

pub(crate) fn owner_function_for_block(snapshot: &Snapshot, block: NodeId) -> Result<NodeId> {
    let mut current = block;
    loop {
        let Node::Block { owner: region, .. } = snapshot.node(current)? else {
            return Err(
                corrupt(snapshot, "owner-function walk encountered a non-block").for_node(current),
            );
        };
        let Node::Region { owner, .. } = snapshot.node(*region)? else {
            return Err(
                corrupt(snapshot, "owner-function walk encountered a non-region").for_node(*region),
            );
        };
        match snapshot.node(*owner)? {
            Node::Function { .. } => return Ok(*owner),
            Node::Operation {
                owner: parent_block,
                ..
            } => current = *parent_block,
            node => {
                return Err(LkError::new(
                    ErrorCode::OwnerMismatch,
                    "region owner cannot lead to a function",
                )
                .for_node(*region)
                .with_kinds(NodeKind::Operation, node.kind()));
            }
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
                nodes
            )
            .expect_err("allocator gap")
            .code,
            ErrorCode::InvalidContainment
        );
    }

    fn structured_for_nodes(step: i64) -> (WorkspaceId, BTreeMap<NodeId, Node>) {
        let workspace = WorkspaceId::from_bytes([0x4a; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let result = |serial| ValueRef::OperationResult {
            operation: id(serial),
            output: 0,
        };
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
                    name: "app".into(),
                    modules: vec![id(3)],
                    entry: Some(id(4)),
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "main".into(),
                    functions: vec![id(4)],
                },
            ),
            (
                id(4),
                Node::Function {
                    owner: id(3),
                    name: "sum".into(),
                    parameters: vec![id(5)],
                    result: SemanticType::I64,
                    body: Some(id(6)),
                },
            ),
            (
                id(5),
                Node::Parameter {
                    owner: id(4),
                    ordinal: 0,
                    name: "n".into(),
                    ty: SemanticType::I64,
                },
            ),
            (
                id(6),
                Node::Region {
                    owner: id(4),
                    blocks: vec![id(7)],
                },
            ),
            (
                id(7),
                Node::Block {
                    owner: id(6),
                    arguments: vec![],
                    operations: vec![id(8), id(9), id(10)],
                    terminator: Some(id(17)),
                },
            ),
            (
                id(8),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::ConstI64(0),
                },
            ),
            (
                id(9),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::ConstI64(0),
                },
            ),
            (
                id(10),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::ForI64 {
                        start: result(8),
                        end_exclusive: ValueRef::FunctionParameter(id(5)),
                        step,
                        initial: result(9),
                        carried: SemanticType::I64,
                        body_region: id(11),
                    },
                },
            ),
            (
                id(11),
                Node::Region {
                    owner: id(10),
                    blocks: vec![id(12)],
                },
            ),
            (
                id(12),
                Node::Block {
                    owner: id(11),
                    arguments: vec![id(13), id(14)],
                    operations: vec![id(15)],
                    terminator: Some(id(16)),
                },
            ),
            (
                id(13),
                Node::BlockArgument {
                    owner: id(12),
                    ordinal: 0,
                    ty: SemanticType::I64,
                },
            ),
            (
                id(14),
                Node::BlockArgument {
                    owner: id(12),
                    ordinal: 1,
                    ty: SemanticType::I64,
                },
            ),
            (
                id(15),
                Node::Operation {
                    owner: id(12),
                    operation: OperationKind::Hole {
                        expected: SemanticType::I64,
                    },
                },
            ),
            (
                id(16),
                Node::Operation {
                    owner: id(12),
                    operation: OperationKind::Yield { value: result(15) },
                },
            ),
            (
                id(17),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::Return { value: result(10) },
                },
            ),
        ]);
        (workspace, nodes)
    }

    #[test]
    fn structured_for_contract_scope_and_nested_refinement_are_exact() {
        let (workspace, nodes) = structured_for_nodes(1);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let previous = Snapshot::from_parts(
            workspace,
            Revision::new(1),
            id(1),
            18,
            BTreeSet::new(),
            nodes.clone(),
        )
        .expect("valid loop with nested hole");

        let mut refined_nodes = nodes.clone();
        let Node::Operation { operation, .. } = refined_nodes.get_mut(&id(15)).expect("hole")
        else {
            unreachable!()
        };
        *operation = OperationKind::AddI64 {
            lhs: ValueRef::BlockArgument(id(14)),
            rhs: ValueRef::BlockArgument(id(13)),
        };
        let refined = Snapshot::from_parts(
            workspace,
            Revision::new(2),
            id(1),
            18,
            BTreeSet::new(),
            refined_nodes,
        )
        .expect("loop arguments visible to nested add");
        crate::graph::validate_history_transition(&previous, &refined)
            .expect("hole refinement history");

        let mut owner_result_capture = nodes.clone();
        let Node::Operation { operation, .. } =
            owner_result_capture.get_mut(&id(15)).expect("hole")
        else {
            unreachable!()
        };
        *operation = OperationKind::AddI64 {
            lhs: ValueRef::OperationResult {
                operation: id(10),
                output: 0,
            },
            rhs: ValueRef::BlockArgument(id(13)),
        };
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                18,
                BTreeSet::new(),
                owner_result_capture
            )
            .expect_err("owning loop result is not visible inside its body")
            .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn structured_region_shape_step_and_terminator_rejections_are_typed() {
        let (workspace, nodes) = structured_for_nodes(0);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                18,
                BTreeSet::new(),
                nodes
            )
            .expect_err("zero loop step")
            .code,
            ErrorCode::InvalidOperand
        );

        let (_, mut bad_argument) = structured_for_nodes(1);
        let Node::BlockArgument { ty, .. } = bad_argument.get_mut(&id(13)).expect("index") else {
            unreachable!()
        };
        *ty = SemanticType::Bool;
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                18,
                BTreeSet::new(),
                bad_argument
            )
            .expect_err("loop index type mismatch")
            .code,
            ErrorCode::TypeMismatch
        );

        let (_, mut bad_yield) = structured_for_nodes(1);
        let Node::Operation { operation, .. } = bad_yield.get_mut(&id(16)).expect("yield") else {
            unreachable!()
        };
        *operation = OperationKind::Return {
            value: ValueRef::OperationResult {
                operation: id(15),
                output: 0,
            },
        };
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                18,
                BTreeSet::new(),
                bad_yield
            )
            .expect_err("return cannot terminate operation region")
            .code,
            ErrorCode::InvalidContainment
        );
    }

    #[test]
    fn structured_if_arms_capture_only_prior_outer_values() {
        let workspace = WorkspaceId::from_bytes([0x4b; 16]);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let result = |serial| ValueRef::OperationResult {
            operation: id(serial),
            output: 0,
        };
        let mut nodes = BTreeMap::from([
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
                    name: "app".into(),
                    modules: vec![id(3)],
                    entry: Some(id(4)),
                },
            ),
            (
                id(3),
                Node::Module {
                    owner: id(2),
                    name: "m".into(),
                    functions: vec![id(4)],
                },
            ),
            (
                id(4),
                Node::Function {
                    owner: id(3),
                    name: "choose".into(),
                    parameters: vec![id(5)],
                    result: SemanticType::I64,
                    body: Some(id(6)),
                },
            ),
            (
                id(5),
                Node::Parameter {
                    owner: id(4),
                    ordinal: 0,
                    name: "condition".into(),
                    ty: SemanticType::Bool,
                },
            ),
            (
                id(6),
                Node::Region {
                    owner: id(4),
                    blocks: vec![id(7)],
                },
            ),
            (
                id(7),
                Node::Block {
                    owner: id(6),
                    arguments: vec![],
                    operations: vec![id(8), id(9), id(18)],
                    terminator: Some(id(17)),
                },
            ),
            (
                id(8),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::ConstI64(1),
                },
            ),
            (
                id(9),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::If {
                        condition: ValueRef::FunctionParameter(id(5)),
                        result: SemanticType::I64,
                        then_region: id(10),
                        else_region: id(13),
                    },
                },
            ),
            (
                id(10),
                Node::Region {
                    owner: id(9),
                    blocks: vec![id(11)],
                },
            ),
            (
                id(11),
                Node::Block {
                    owner: id(10),
                    arguments: vec![],
                    operations: vec![],
                    terminator: Some(id(12)),
                },
            ),
            (
                id(12),
                Node::Operation {
                    owner: id(11),
                    operation: OperationKind::Yield { value: result(8) },
                },
            ),
            (
                id(13),
                Node::Region {
                    owner: id(9),
                    blocks: vec![id(14)],
                },
            ),
            (
                id(14),
                Node::Block {
                    owner: id(13),
                    arguments: vec![],
                    operations: vec![id(15)],
                    terminator: Some(id(16)),
                },
            ),
            (
                id(15),
                Node::Operation {
                    owner: id(14),
                    operation: OperationKind::ConstI64(2),
                },
            ),
            (
                id(16),
                Node::Operation {
                    owner: id(14),
                    operation: OperationKind::Yield { value: result(15) },
                },
            ),
            (
                id(17),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::Return { value: result(9) },
                },
            ),
            (
                id(18),
                Node::Operation {
                    owner: id(7),
                    operation: OperationKind::ConstI64(3),
                },
            ),
        ]);
        Snapshot::from_parts(
            workspace,
            Revision::new(1),
            id(1),
            19,
            BTreeSet::new(),
            nodes.clone(),
        )
        .expect("if arms may capture prior values and have no block arguments");
        let Node::Operation { operation, .. } = nodes.get_mut(&id(12)).expect("then yield") else {
            unreachable!()
        };
        *operation = OperationKind::Yield { value: result(18) };
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                19,
                BTreeSet::new(),
                nodes
            )
            .expect_err("if arm cannot capture a future outer value")
            .code,
            ErrorCode::InvalidOperand
        );
    }

    #[test]
    fn direct_call_uses_target_identity_and_exact_signature() {
        let (workspace, mut nodes) = structured_for_nodes(1);
        let id = |serial| NodeId::new(workspace, serial).expect("node");
        let Node::Module { functions, .. } = nodes.get_mut(&id(3)).expect("module") else {
            unreachable!()
        };
        functions.push(id(18));
        nodes.insert(
            id(18),
            Node::Function {
                owner: id(3),
                name: "callee".into(),
                parameters: vec![id(19)],
                result: SemanticType::I64,
                body: None,
            },
        );
        nodes.insert(
            id(19),
            Node::Parameter {
                owner: id(18),
                ordinal: 0,
                name: "x".into(),
                ty: SemanticType::I64,
            },
        );
        let Node::Operation { operation, .. } = nodes.get_mut(&id(8)).expect("start") else {
            unreachable!()
        };
        *operation = OperationKind::Call {
            function: id(18),
            arguments: vec![ValueRef::FunctionParameter(id(5))],
        };
        Snapshot::from_parts(
            workspace,
            Revision::new(1),
            id(1),
            20,
            BTreeSet::new(),
            nodes.clone(),
        )
        .expect("identity-targeted exact call");
        let Node::Operation { operation, .. } = nodes.get_mut(&id(8)).expect("call") else {
            unreachable!()
        };
        *operation = OperationKind::Call {
            function: id(18),
            arguments: vec![],
        };
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                20,
                BTreeSet::new(),
                nodes
            )
            .expect_err("call arity mismatch")
            .code,
            ErrorCode::InvalidOperand
        );
    }
}
