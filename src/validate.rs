use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, operation_result_type};
use crate::ids::NodeId;
use crate::schema::{
    DirectReference, Node, NodeKind, OperationKind, RegionArity, SemanticType, TypeRule, ValueRef,
    owner_kind_is_valid,
};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn validate_snapshot(snapshot: &Snapshot) -> Result<()> {
    validate_identity(snapshot)?;
    validate_containment(snapshot)?;
    validate_identity_domains(snapshot)?;
    validate_names(snapshot)?;
    validate_semantics(snapshot)?;
    crate::type_layout::validate_acyclic(snapshot)?;
    crate::target::validate_snapshot_targets(snapshot)?;
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
    if NodeId::new(snapshot.workspace, snapshot.next_serial).is_err() {
        return Err(corrupt(
            snapshot,
            "durable allocator frontier is outside the durable identity domain",
        ));
    }
    let live_count = u64::try_from(snapshot.nodes.keys().filter(|id| id.is_durable()).count())
        .map_err(|_| {
            corrupt(
                snapshot,
                "live durable identity count overflows allocator representation",
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
            "represented durable identity count overflows allocator state",
        )
    })?;
    if represented != snapshot.next_serial - 1 {
        return Err(corrupt(
            snapshot,
            "every allocated durable identity must be live or tombstoned",
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
        if id.is_durable() && id.serial() >= snapshot.next_serial {
            return Err(
                corrupt(snapshot, "live durable identity is beyond allocator state").for_node(*id),
            );
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

fn validate_identity_domains(snapshot: &Snapshot) -> Result<()> {
    for (id, node) in &snapshot.nodes {
        let local_kind = matches!(
            node,
            Node::Region { .. }
                | Node::Block { .. }
                | Node::BlockArgument { .. }
                | Node::Operation { .. }
        );
        if id.is_function_local() && !local_kind {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "durable semantic entities cannot use a function-local reference",
            )
            .for_node(*id));
        }
        if id.is_durable()
            && matches!(
                node,
                Node::Region { .. } | Node::Block { .. } | Node::BlockArgument { .. }
            )
        {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "body scaffolding requires a function-local reference",
            )
            .for_node(*id));
        }
        let Some(function_serial) = id.local_function_serial() else {
            continue;
        };
        let function = NodeId::new(snapshot.workspace, function_serial).map_err(|error| {
            corrupt(
                snapshot,
                &format!("function-local reference has an invalid owner domain: {error}"),
            )
            .for_node(*id)
        })?;
        if !matches!(snapshot.nodes.get(&function), Some(Node::Function { .. })) {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "function-local reference names a missing or non-function durable owner",
            )
            .for_node(*id)
            .with_related([function]));
        }
        let mut current = *id;
        let mut remaining = snapshot.nodes.len().saturating_add(1);
        loop {
            if remaining == 0 {
                return Err(corrupt(
                    snapshot,
                    "function-local owner chain does not terminate at its durable function",
                )
                .for_node(*id));
            }
            remaining -= 1;
            if current == function {
                break;
            }
            let current_node = snapshot.nodes.get(&current).ok_or_else(|| {
                corrupt(
                    snapshot,
                    "function-local owner chain contains a missing node",
                )
                .for_node(current)
                .with_related([*id])
            })?;
            current = current_node.owner().ok_or_else(|| {
                corrupt(
                    snapshot,
                    "function-local owner chain reached a root before its function",
                )
                .for_node(*id)
            })?;
        }
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
            let target_node = snapshot.nodes.get(&target).ok_or_else(|| {
                LkError::new(
                    ErrorCode::NodeNotFound,
                    "direct reference target does not exist",
                )
                .for_node(target)
                .with_related([*owner_id])
            })?;
            if matches!(reference, DirectReference::Type { .. })
                && !matches!(
                    target_node,
                    Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
                )
            {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "nominal semantic type must target a product, sum, or sequence declaration",
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
        Node::WorkspaceRoot { packages, targets } => {
            require_children(snapshot, id, packages, NodeKind::Package)?;
            require_children(snapshot, id, targets, NodeKind::BuildTarget)?;
        }
        Node::BuildTarget { .. } => {}
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
        Node::Module {
            types, functions, ..
        } => {
            for ty in types {
                let node = snapshot.node(*ty)?;
                if !matches!(
                    node,
                    Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
                ) {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "module type slot must contain a nominal declaration",
                    )
                    .for_node(*ty)
                    .with_related([id]));
                }
            }
            require_children(snapshot, id, functions, NodeKind::Function)?;
        }
        Node::ProductType { fields, .. } => {
            require_children(snapshot, id, fields, NodeKind::ProductField)?;
        }
        Node::ProductField { .. } | Node::SumVariant { .. } | Node::SequenceType { .. } => {}
        Node::SumType { variants, .. } => {
            if variants.is_empty() {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "sum declarations require at least one variant",
                )
                .for_node(id));
            }
            require_children(snapshot, id, variants, NodeKind::SumVariant)?;
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
            let expected_regions = match descriptor.region_arity {
                RegionArity::Fixed(count) => usize::from(count),
                RegionArity::MatchVariants { .. } => match operation {
                    OperationKind::MatchSum { arms, .. } => arms.len(),
                    _ => {
                        return Err(corrupt(
                            snapshot,
                            "dynamic match-region rule belongs to the wrong operation",
                        )
                        .for_node(id));
                    }
                },
            };
            if operation.owned_region_count() != expected_regions {
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
    fn validate_group(snapshot: &Snapshot, owner: NodeId, children: &[NodeId]) -> Result<()> {
        let mut names = BTreeMap::<&str, NodeId>::new();
        for child_id in children {
            let child = snapshot
                .nodes
                .get(child_id)
                .ok_or_else(|| corrupt(snapshot, "named child is missing").for_node(*child_id))?;
            let name = child.name().ok_or_else(|| {
                corrupt(snapshot, "named slot contains an unnamed node").for_node(*child_id)
            })?;
            if name.len() < crate::schema::MINIMUM_NAME_UTF8_BYTES {
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
                .with_related([owner, previous]));
            }
        }
        Ok(())
    }

    for (owner_id, owner) in &snapshot.nodes {
        for group in crate::schema::NameUniquenessGroup::ALL {
            if let Some(children) = group.children(owner) {
                validate_group(snapshot, *owner_id, children)?;
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
    for (declaration_id, node) in &snapshot.nodes {
        match node {
            Node::ProductType { fields, .. } => {
                for (expected, field_id) in fields.iter().enumerate() {
                    let Node::ProductField { owner, ordinal, .. } = snapshot.node(*field_id)?
                    else {
                        return Err(corrupt(snapshot, "product field slot has wrong kind")
                            .for_node(*field_id));
                    };
                    let expected = u32::try_from(expected).map_err(|_| {
                        corrupt(snapshot, "product field ordinal overflows representation")
                            .for_node(*field_id)
                    })?;
                    if *owner != *declaration_id || *ordinal != expected {
                        return Err(corrupt(
                            snapshot,
                            "product field owner and ordinals must be dense and ordered",
                        )
                        .for_node(*field_id));
                    }
                }
            }
            Node::SumType { variants, .. } => {
                for (expected, variant_id) in variants.iter().enumerate() {
                    let Node::SumVariant { owner, ordinal, .. } = snapshot.node(*variant_id)?
                    else {
                        return Err(corrupt(snapshot, "sum variant slot has wrong kind")
                            .for_node(*variant_id));
                    };
                    let expected = u32::try_from(expected).map_err(|_| {
                        corrupt(snapshot, "sum variant ordinal overflows representation")
                            .for_node(*variant_id)
                    })?;
                    if *owner != *declaration_id || *ordinal != expected {
                        return Err(corrupt(
                            snapshot,
                            "sum variant owner and ordinals must be dense and ordered",
                        )
                        .for_node(*variant_id));
                    }
                }
            }
            Node::SequenceType { element, .. } => {
                if let SemanticType::Nominal(target) = element
                    && !matches!(
                        snapshot.node(*target)?,
                        Node::ProductType { .. } | Node::SumType { .. } | Node::SequenceType { .. }
                    )
                {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "sequence element type must name a nominal declaration",
                    )
                    .for_node(*target)
                    .with_related([*declaration_id]));
                }
            }
            _ => {}
        }
    }

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
            let function = owner_function_for_block(snapshot, *parent_block)?;
            if let RegionArity::MatchVariants {
                payload_type,
                terminator,
                yield_type,
            } = operation.descriptor().region_arity
            {
                let OperationKind::MatchSum { arms, .. } = operation else {
                    return Err(
                        corrupt(snapshot, "dynamic match-region rule has wrong operation")
                            .for_node(*owner),
                    );
                };
                let arm = arms.get(role_index).ok_or_else(|| {
                    corrupt(snapshot, "match arm region index is absent").for_node(region_id)
                })?;
                let payload = match (payload_type, snapshot.node(arm.variant)?) {
                    (TypeRule::VariantPayload, Node::SumVariant { payload, .. }) => *payload,
                    (TypeRule::VariantPayload, node) => {
                        return Err(LkError::new(
                            ErrorCode::WrongKind,
                            "match arm must name a sum variant",
                        )
                        .for_node(arm.variant)
                        .with_kinds(NodeKind::SumVariant, node.kind()));
                    }
                    _ => {
                        return Err(corrupt(snapshot, "unsupported dynamic match payload rule")
                            .for_node(*owner));
                    }
                };
                let yielded =
                    resolve_type_rule(snapshot, operation, yield_type, function, Some(region_id))?
                        .ok_or_else(|| {
                            corrupt(snapshot, "dynamic match yield type cannot be resolved")
                                .for_node(region_id)
                        })?;
                return Ok(RegionContract {
                    function,
                    expected_arguments: [payload, None],
                    argument_count: usize::from(payload.is_some()),
                    terminator,
                    yielded,
                });
            }
            let descriptor = operation
                .descriptor()
                .regions
                .get(role_index)
                .ok_or_else(|| {
                    corrupt(snapshot, "fixed operation region descriptor is absent")
                        .for_node(region_id)
                })?;
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
    if let OperationKind::ConstBytes(value) = operation
        && value.len() > crate::schema::MAXIMUM_BYTE_LITERAL_BYTES
    {
        return Err(LkError::new(
            ErrorCode::ByteLiteralTooLarge,
            "const_bytes literal exceeds the semantic literal policy",
        )
        .for_node(operation_id));
    }
    if let OperationKind::ConstText(value) = operation
        && value.len_bytes() > crate::schema::MAXIMUM_TEXT_LITERAL_BYTES
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "const_text literal exceeds the semantic literal policy",
        )
        .for_node(operation_id));
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
    validate_nominal_operation_contract(snapshot, operation_id, operation)?;
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

fn validate_nominal_operation_contract(
    snapshot: &Snapshot,
    operation_id: NodeId,
    operation: &OperationKind,
) -> Result<()> {
    match operation {
        OperationKind::ConstructProduct { product, fields } => {
            let declared = match snapshot.node(*product)? {
                Node::ProductType { fields, .. } => fields,
                node => {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "product construction must name a product declaration",
                    )
                    .for_node(*product)
                    .with_kinds(NodeKind::ProductType, node.kind())
                    .with_related([operation_id]));
                }
            };
            if fields.len() != declared.len() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "product field count does not match its declaration",
                )
                .for_node(operation_id)
                .with_related([*product]));
            }
            for (binding, expected) in fields.iter().zip(declared) {
                if binding.field != *expected {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "product fields must be exact and in declaration order",
                    )
                    .for_node(binding.field)
                    .with_related([operation_id, *product, *expected]));
                }
                match snapshot.node(binding.field)? {
                    Node::ProductField { owner, .. } if *owner == *product => {}
                    Node::ProductField { .. } => {
                        return Err(LkError::new(
                            ErrorCode::OwnerMismatch,
                            "product field belongs to another declaration",
                        )
                        .for_node(binding.field)
                        .with_related([*product, operation_id]));
                    }
                    node => {
                        return Err(LkError::new(
                            ErrorCode::WrongKind,
                            "product binding must name a product field",
                        )
                        .for_node(binding.field)
                        .with_kinds(NodeKind::ProductField, node.kind()));
                    }
                }
            }
        }
        OperationKind::ProjectField { field, .. } => {
            if !matches!(snapshot.node(*field)?, Node::ProductField { .. }) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "projection must name a product field",
                )
                .for_node(*field)
                .with_kinds(NodeKind::ProductField, snapshot.node(*field)?.kind()));
            }
        }
        OperationKind::ConstructVariant { variant, payload } => match snapshot.node(*variant)? {
            Node::SumVariant {
                payload: expected, ..
            } if expected.is_some() == payload.is_some() => {}
            Node::SumVariant { .. } => {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "variant payload presence does not match its declaration",
                )
                .for_node(*variant)
                .with_related([operation_id]));
            }
            node => {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "variant construction must name a sum variant",
                )
                .for_node(*variant)
                .with_kinds(NodeKind::SumVariant, node.kind()));
            }
        },
        OperationKind::MatchSum { arms, .. } => {
            let first = arms.first().ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "match_sum requires exhaustive arms",
                )
                .for_node(operation_id)
            })?;
            let sum = match snapshot.node(first.variant)? {
                Node::SumVariant { owner, .. } => *owner,
                node => {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "match arm must name a sum variant",
                    )
                    .for_node(first.variant)
                    .with_kinds(NodeKind::SumVariant, node.kind()));
                }
            };
            let variants = match snapshot.node(sum)? {
                Node::SumType { variants, .. } => variants,
                _ => unreachable!(),
            };
            if arms.len() != variants.len() {
                return Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "match arm count is not exhaustive",
                )
                .for_node(operation_id)
                .with_related([sum]));
            }
            for (arm, expected) in arms.iter().zip(variants) {
                if arm.variant != *expected {
                    return Err(LkError::new(
                        ErrorCode::InvalidOperand,
                        "match arms must be exact and in declaration order",
                    )
                    .for_node(arm.variant)
                    .with_related([operation_id, sum, *expected]));
                }
            }
        }
        OperationKind::SequenceEmpty { sequence }
        | OperationKind::SequenceLen { sequence, .. }
        | OperationKind::SequenceGet { sequence, .. }
        | OperationKind::SequenceAppend { sequence, .. }
        | OperationKind::SequenceReplace { sequence, .. }
        | OperationKind::SequenceSlice { sequence, .. }
        | OperationKind::SequenceConcat { sequence, .. } => {
            let node = snapshot.node(*sequence)?;
            if !matches!(node, Node::SequenceType { .. }) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "sequence operation must name a sequence declaration",
                )
                .for_node(*sequence)
                .with_kinds(NodeKind::SequenceType, node.kind())
                .with_related([operation_id]));
            }
        }
        _ => {}
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
        OperationKind::ConstructProduct { fields, .. } => fields
            .iter()
            .map(|binding| match snapshot.node(binding.field)? {
                Node::ProductField { ty, .. } => Ok(*ty),
                node => Err(LkError::new(
                    ErrorCode::WrongKind,
                    "product binding target must be a field",
                )
                .for_node(binding.field)
                .with_kinds(NodeKind::ProductField, node.kind())),
            })
            .collect(),
        OperationKind::ProjectField { field, .. } => match snapshot.node(*field)? {
            Node::ProductField { owner, .. } => Ok(vec![SemanticType::Nominal(*owner)]),
            node => Err(LkError::new(
                ErrorCode::WrongKind,
                "projection target must be a product field",
            )
            .for_node(*field)
            .with_kinds(NodeKind::ProductField, node.kind())),
        },
        OperationKind::ConstructVariant {
            variant,
            payload: _,
        } => match snapshot.node(*variant)? {
            Node::SumVariant {
                payload: expected, ..
            } => Ok(expected.iter().copied().collect()),
            node => Err(
                LkError::new(ErrorCode::WrongKind, "variant target must be a sum variant")
                    .for_node(*variant)
                    .with_kinds(NodeKind::SumVariant, node.kind()),
            ),
        },
        OperationKind::MatchSum { arms, .. } => {
            let first = arms.first().ok_or_else(|| {
                LkError::new(ErrorCode::InvalidOperand, "match_sum requires arms")
                    .for_node(operation_id)
            })?;
            let sum = match snapshot.node(first.variant)? {
                Node::SumVariant { owner, .. } => *owner,
                node => {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "match arm must name a sum variant",
                    )
                    .for_node(first.variant)
                    .with_kinds(NodeKind::SumVariant, node.kind()));
                }
            };
            Ok(vec![SemanticType::Nominal(sum)])
        }
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
    snapshot: &Snapshot,
    operation: &OperationKind,
    rule: TypeRule,
    _function: NodeId,
    _region: Option<NodeId>,
) -> Result<Option<SemanticType>> {
    Ok(match rule {
        TypeRule::Fixed(ty) => Some(ty),
        TypeRule::PayloadExpected => match operation {
            OperationKind::Hole { expected }
            | OperationKind::MatchSum {
                result: expected, ..
            } => Some(*expected),
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
        TypeRule::ProductDeclarationResult => match operation {
            OperationKind::ConstructProduct { product, .. } => {
                Some(SemanticType::Nominal(*product))
            }
            _ => None,
        },
        TypeRule::MatchResult => match operation {
            OperationKind::MatchSum { result, .. } => Some(*result),
            _ => None,
        },
        TypeRule::SequenceDeclarationResult | TypeRule::SequenceOwner => match operation {
            OperationKind::SequenceEmpty { sequence }
            | OperationKind::SequenceLen { sequence, .. }
            | OperationKind::SequenceGet { sequence, .. }
            | OperationKind::SequenceAppend { sequence, .. }
            | OperationKind::SequenceReplace { sequence, .. }
            | OperationKind::SequenceSlice { sequence, .. }
            | OperationKind::SequenceConcat { sequence, .. } => {
                Some(SemanticType::Nominal(*sequence))
            }
            _ => None,
        },
        TypeRule::SequenceElement => match operation {
            OperationKind::SequenceEmpty { sequence }
            | OperationKind::SequenceLen { sequence, .. }
            | OperationKind::SequenceGet { sequence, .. }
            | OperationKind::SequenceAppend { sequence, .. }
            | OperationKind::SequenceReplace { sequence, .. }
            | OperationKind::SequenceSlice { sequence, .. }
            | OperationKind::SequenceConcat { sequence, .. } => match snapshot.node(*sequence)? {
                Node::SequenceType { element, .. } => Some(*element),
                _ => None,
            },
            _ => None,
        },
        TypeRule::OwnerFunctionResult
        | TypeRule::CallTargetParameter
        | TypeRule::CallTargetResult
        | TypeRule::OwningRegionYield
        | TypeRule::ProductFieldType
        | TypeRule::ProjectionOwner
        | TypeRule::ProjectedFieldResult
        | TypeRule::VariantPayload
        | TypeRule::VariantOwnerResult
        | TypeRule::MatchScrutinee => None,
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
    fn deeply_nested_match_validation_uses_explicit_graph_work() {
        const DEPTH: usize = 1_000;
        let workspace = WorkspaceId::from_bytes([0x96; 16]);
        let function = NodeId::new(workspace, 6).expect("function");
        let id = |serial: u64| {
            if serial <= 6 {
                NodeId::new(workspace, serial).expect("durable entity")
            } else {
                NodeId::new_function_local(
                    workspace,
                    function,
                    u32::try_from(serial - 6).expect("local ordinal"),
                )
                .expect("function-local node")
            }
        };
        let mut nodes = BTreeMap::new();
        nodes.insert(
            id(1),
            Node::WorkspaceRoot {
                packages: vec![id(2)],
                targets: Vec::new(),
            },
        );
        nodes.insert(
            id(2),
            Node::Package {
                owner: id(1),
                name: "p".into(),
                modules: vec![id(3)],
                entry: Some(id(6)),
            },
        );
        nodes.insert(
            id(3),
            Node::Module {
                owner: id(2),
                name: "m".into(),
                types: vec![id(4)],
                functions: vec![id(6)],
            },
        );
        nodes.insert(
            id(4),
            Node::SumType {
                owner: id(3),
                name: "Only".into(),
                variants: vec![id(5)],
            },
        );
        nodes.insert(
            id(5),
            Node::SumVariant {
                owner: id(4),
                ordinal: 0,
                name: "only".into(),
                payload: None,
            },
        );
        nodes.insert(
            id(6),
            Node::Function {
                owner: id(3),
                name: "main".into(),
                parameters: Vec::new(),
                result: SemanticType::I64,
                body: Some(id(7)),
            },
        );
        nodes.insert(
            id(7),
            Node::Region {
                owner: id(6),
                blocks: vec![id(8)],
            },
        );
        let variant_operation = id(9);
        nodes.insert(
            variant_operation,
            Node::Operation {
                owner: id(8),
                operation: OperationKind::ConstructVariant {
                    variant: id(5),
                    payload: None,
                },
            },
        );
        let mut next_local = 10_u64;
        let mut matches = Vec::with_capacity(DEPTH);
        for _ in 0..DEPTH {
            let operation = id(next_local);
            next_local += 1;
            let region = id(next_local);
            next_local += 1;
            let block = id(next_local);
            next_local += 1;
            let yield_operation = id(next_local);
            next_local += 1;
            matches.push((operation, region, block, yield_operation));
        }
        let constant = id(next_local);
        next_local += 1;
        let return_operation = id(next_local);
        nodes.insert(
            id(8),
            Node::Block {
                owner: id(7),
                arguments: Vec::new(),
                operations: vec![variant_operation, matches[0].0],
                terminator: Some(return_operation),
            },
        );
        for (index, (operation, region, block, yield_operation)) in
            matches.iter().copied().enumerate()
        {
            let owner_block = if index == 0 {
                id(8)
            } else {
                matches[index - 1].2
            };
            nodes.insert(
                operation,
                Node::Operation {
                    owner: owner_block,
                    operation: OperationKind::MatchSum {
                        scrutinee: ValueRef::OperationResult {
                            operation: variant_operation,
                            output: 0,
                        },
                        result: SemanticType::I64,
                        arms: vec![crate::schema::MatchArm {
                            variant: id(5),
                            region,
                        }],
                    },
                },
            );
            nodes.insert(
                region,
                Node::Region {
                    owner: operation,
                    blocks: vec![block],
                },
            );
            let yielded = if index + 1 == DEPTH {
                constant
            } else {
                matches[index + 1].0
            };
            nodes.insert(
                block,
                Node::Block {
                    owner: region,
                    arguments: Vec::new(),
                    operations: vec![yielded],
                    terminator: Some(yield_operation),
                },
            );
            nodes.insert(
                yield_operation,
                Node::Operation {
                    owner: block,
                    operation: OperationKind::Yield {
                        value: ValueRef::OperationResult {
                            operation: yielded,
                            output: 0,
                        },
                    },
                },
            );
        }
        nodes.insert(
            constant,
            Node::Operation {
                owner: matches[DEPTH - 1].2,
                operation: OperationKind::ConstI64(1),
            },
        );
        nodes.insert(
            return_operation,
            Node::Operation {
                owner: id(8),
                operation: OperationKind::Return {
                    value: ValueRef::OperationResult {
                        operation: matches[0].0,
                        output: 0,
                    },
                },
            },
        );
        Snapshot::from_parts(
            workspace,
            Revision::INITIAL,
            id(1),
            7,
            BTreeSet::new(),
            nodes,
        )
        .expect("deep match snapshot validates without native recursion");
    }

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
                targets: Vec::new(),
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

    fn structured_for_id(workspace: WorkspaceId, serial: u64) -> NodeId {
        if serial <= 5 {
            NodeId::new(workspace, serial).expect("durable entity")
        } else if serial == 15 {
            NodeId::new(workspace, 6).expect("durable hole anchor")
        } else {
            NodeId::new_function_local(
                workspace,
                NodeId::new(workspace, 4).expect("function"),
                u32::try_from(serial).expect("local ordinal"),
            )
            .expect("function-local node")
        }
    }

    fn structured_for_nodes(step: i64) -> (WorkspaceId, BTreeMap<NodeId, Node>) {
        let workspace = WorkspaceId::from_bytes([0x4a; 16]);
        let id = |serial| structured_for_id(workspace, serial);
        let result = |serial| ValueRef::OperationResult {
            operation: id(serial),
            output: 0,
        };
        let nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                    targets: Vec::new(),
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
                    types: Vec::new(),
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
        let id = |serial| structured_for_id(workspace, serial);
        let previous = Snapshot::from_parts(
            workspace,
            Revision::new(1),
            id(1),
            7,
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
            7,
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
                7,
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
        let id = |serial| structured_for_id(workspace, serial);
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                7,
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
                7,
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
                7,
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
        let function = NodeId::new(workspace, 4).expect("function");
        let id = |serial: u64| {
            if serial <= 5 {
                NodeId::new(workspace, serial).expect("durable entity")
            } else {
                NodeId::new_function_local(
                    workspace,
                    function,
                    u32::try_from(serial - 5).expect("local ordinal"),
                )
                .expect("function-local node")
            }
        };
        let result = |serial| ValueRef::OperationResult {
            operation: id(serial),
            output: 0,
        };
        let mut nodes = BTreeMap::from([
            (
                id(1),
                Node::WorkspaceRoot {
                    packages: vec![id(2)],
                    targets: Vec::new(),
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
                    types: Vec::new(),
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
            6,
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
                6,
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
        let id = |serial| structured_for_id(workspace, serial);
        let callee = NodeId::new(workspace, 7).expect("callee entity");
        let callee_parameter = NodeId::new(workspace, 8).expect("callee parameter entity");
        let Node::Module { functions, .. } = nodes.get_mut(&id(3)).expect("module") else {
            unreachable!()
        };
        functions.push(callee);
        nodes.insert(
            callee,
            Node::Function {
                owner: id(3),
                name: "callee".into(),
                parameters: vec![callee_parameter],
                result: SemanticType::I64,
                body: None,
            },
        );
        nodes.insert(
            callee_parameter,
            Node::Parameter {
                owner: callee,
                ordinal: 0,
                name: "x".into(),
                ty: SemanticType::I64,
            },
        );
        let Node::Operation { operation, .. } = nodes.get_mut(&id(8)).expect("start") else {
            unreachable!()
        };
        *operation = OperationKind::Call {
            function: callee,
            arguments: vec![ValueRef::FunctionParameter(id(5))],
        };
        Snapshot::from_parts(
            workspace,
            Revision::new(1),
            id(1),
            9,
            BTreeSet::new(),
            nodes.clone(),
        )
        .expect("identity-targeted exact call");
        let Node::Operation { operation, .. } = nodes.get_mut(&id(8)).expect("call") else {
            unreachable!()
        };
        *operation = OperationKind::Call {
            function: callee,
            arguments: vec![],
        };
        assert_eq!(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                id(1),
                9,
                BTreeSet::new(),
                nodes
            )
            .expect_err("call arity mismatch")
            .code,
            ErrorCode::InvalidOperand
        );
    }
}
