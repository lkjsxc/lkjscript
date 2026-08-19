use super::{
    CanonicalReleaseTest, DecodedRelease, MAXIMUM_RELEASE_DEPENDENCIES, MAXIMUM_RELEASE_EXPORTS,
    MAXIMUM_RELEASE_IMPORTS, MAXIMUM_RELEASE_ITEMS, MAXIMUM_RELEASE_SUITE_FUEL,
    MAXIMUM_RELEASE_TESTS, ReleaseBuildRequest, ReleaseContentDigest, ReleaseDependency,
    ReleaseExport, ReleaseExportKind, ReleaseId, ReleaseImport, ReleaseItemId,
    ReleaseTestExpectation,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::interpret;
use crate::schema::{
    DirectReference, MatchArm, Node, OperationKind, ProductFieldValue, SemanticType, ValueRef,
};
use std::collections::{BTreeMap, BTreeSet};

/// Private identity used only while validating one decoded release. Its bytes are never encoded.
pub(super) const RELEASE_LOCAL_WORKSPACE: WorkspaceId = WorkspaceId::from_bytes([
    0x6c, 0x6b, 0x6a, 0x72, 0x65, 0x6c, 0x65, 0x61, 0x73, 0x65, 0x30, 0x30, 0x30, 0x30, 0x30, 0x31,
]);

pub(super) fn project(
    source: &Snapshot,
    request: &ReleaseBuildRequest,
    supplied: &[DecodedRelease],
) -> Result<(DecodedRelease, BTreeMap<NodeId, ReleaseItemId>)> {
    super::validate_coordinate(&request.coordinate)?;
    super::validate_user_version(&request.user_version)?;
    validate_counts(request)?;

    let Node::Package { .. } = source.node(request.root)? else {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "reusable release root must be a workspace package",
        )
        .for_node(request.root));
    };

    let supplied_by_id = supplied
        .iter()
        .map(|release| (release.id, release))
        .collect::<BTreeMap<_, _>>();
    let dependencies = canonical_dependencies(request, &supplied_by_id)?;
    let source_imports = canonical_source_imports(source, request, &dependencies, &supplied_by_id)?;
    let source_exports = canonical_source_exports(source, request)?;
    let roots = source_exports
        .iter()
        .map(|(_, target)| *target)
        .chain(request.tests.iter().map(|test| test.target))
        .collect::<Vec<_>>();
    if roots.is_empty() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "a reusable release must contain at least one export or test",
        ));
    }
    let selected = semantic_closure_ids(source, &roots)?;
    ensure_one_package(source, request.root, &selected)?;
    for local in source_imports.keys() {
        if !selected.contains(local) {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "release import is outside the exact export and test closure",
            )
            .for_node(*local));
        }
    }

    let map = build_id_map(source, request.root, &selected)?;
    let snapshot = remap_snapshot(source, request.root, &selected, &map)?;
    let mut imports = source_imports
        .into_iter()
        .map(|(local, (dependency_slot, target))| {
            Ok(ReleaseImport {
                local: release_item(map_id(&map, local)?)?,
                dependency_slot,
                target,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    imports.sort_by_key(|import| import.local);
    let exports = source_exports
        .into_iter()
        .map(|(name, target)| {
            let mapped = map_id(&map, target)?;
            let kind = ReleaseExportKind::for_node(snapshot.node(mapped)?).ok_or_else(|| {
                LkError::new(ErrorCode::WrongKind, "release export kind is unsupported")
                    .for_node(target)
            })?;
            Ok(ReleaseExport {
                name,
                kind,
                target: release_item(mapped)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let tests = canonical_tests(request, &map)?;
    let unit_root = release_item(map_id(&map, request.root)?)?;
    let release = DecodedRelease {
        bytes: Vec::new(),
        id: ReleaseId::from_bytes([0; 32]),
        content_digest: ReleaseContentDigest::from_bytes([0; 32]),
        coordinate: request.coordinate.clone(),
        user_version: request.user_version.clone(),
        unit_root,
        dependencies,
        imports,
        exports,
        tests,
        snapshot,
    };
    validate_release_model(&release, false)?;
    let source_items = map
        .into_iter()
        .filter(|(source, _)| source.is_durable())
        .map(|(source, local)| Ok((source, release_item(local)?)))
        .collect::<Result<BTreeMap<_, _>>>()?;
    Ok((release, source_items))
}

fn validate_counts(request: &ReleaseBuildRequest) -> Result<()> {
    for (actual, maximum, label) in [
        (
            request.exports.len(),
            MAXIMUM_RELEASE_EXPORTS,
            "release export count",
        ),
        (
            request.dependencies.len(),
            MAXIMUM_RELEASE_DEPENDENCIES,
            "release dependency count",
        ),
        (
            request.imports.len(),
            MAXIMUM_RELEASE_IMPORTS,
            "release import count",
        ),
        (
            request.tests.len(),
            MAXIMUM_RELEASE_TESTS,
            "release test count",
        ),
    ] {
        if actual > maximum {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                format!("{label} exceeds policy"),
            ));
        }
    }
    Ok(())
}

fn canonical_dependencies(
    request: &ReleaseBuildRequest,
    supplied: &BTreeMap<ReleaseId, &DecodedRelease>,
) -> Result<Vec<ReleaseDependency>> {
    let mut dependencies = request
        .dependencies
        .iter()
        .map(|dependency| {
            super::validate_symbol(
                &dependency.slot,
                super::MAXIMUM_RELEASE_SLOT_BYTES,
                "dependency slot",
            )?;
            if !supplied.contains_key(&dependency.release) {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    format!(
                        "exact dependency {} for slot '{}' was not supplied",
                        dependency.release, dependency.slot
                    ),
                ));
            }
            Ok(ReleaseDependency {
                slot: dependency.slot.clone(),
                release: dependency.release,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    dependencies.sort_by(|left, right| left.slot.cmp(&right.slot));
    if dependencies
        .windows(2)
        .any(|pair| pair[0].slot == pair[1].slot)
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "release contains a duplicate dependency slot",
        ));
    }
    Ok(dependencies)
}

fn canonical_source_imports(
    source: &Snapshot,
    request: &ReleaseBuildRequest,
    dependencies: &[ReleaseDependency],
    supplied: &BTreeMap<ReleaseId, &DecodedRelease>,
) -> Result<BTreeMap<NodeId, (String, ReleaseItemId)>> {
    let slots = dependencies
        .iter()
        .map(|dependency| (dependency.slot.as_str(), dependency.release))
        .collect::<BTreeMap<_, _>>();
    let mut imports = BTreeMap::new();
    let mut used_slots = BTreeSet::new();
    for import in &request.imports {
        let release_id = slots.get(import.dependency_slot.as_str()).ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                format!(
                    "release import names undeclared dependency slot '{}'",
                    import.dependency_slot
                ),
            )
        })?;
        let dependency = supplied.get(release_id).ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "release import dependency bytes are missing",
            )
        })?;
        let export = dependency
            .exports
            .iter()
            .find(|export| export.name == import.export)
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::NodeNotFound,
                    format!(
                        "dependency slot '{}' has no export named '{}'",
                        import.dependency_slot, import.export
                    ),
                )
            })?;
        let local_kind =
            ReleaseExportKind::for_node(source.node(import.local)?).ok_or_else(|| {
                LkError::new(
                    ErrorCode::WrongKind,
                    "release import proxy must be a function or nominal declaration",
                )
                .for_node(import.local)
            })?;
        if local_kind != export.kind {
            return Err(LkError::new(
                ErrorCode::TypeMismatch,
                "release import proxy and dependency export have different kinds",
            )
            .for_node(import.local));
        }
        if imports
            .insert(
                import.local,
                (import.dependency_slot.clone(), export.target),
            )
            .is_some()
        {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "release contains a duplicate local import proxy",
            )
            .for_node(import.local));
        }
        used_slots.insert(import.dependency_slot.as_str());
    }
    if dependencies
        .iter()
        .any(|dependency| !used_slots.contains(dependency.slot.as_str()))
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "every exact dependency must have at least one direct import",
        ));
    }
    Ok(imports)
}

fn canonical_source_exports(
    source: &Snapshot,
    request: &ReleaseBuildRequest,
) -> Result<Vec<(String, NodeId)>> {
    if request.exports.is_empty() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "a reusable release must expose at least one item",
        ));
    }
    let imported = request
        .imports
        .iter()
        .map(|import| import.local)
        .collect::<BTreeSet<_>>();
    let mut exports = request
        .exports
        .iter()
        .map(|export| {
            super::validate_symbol(
                &export.name,
                super::MAXIMUM_RELEASE_NAME_BYTES,
                "release export name",
            )?;
            if imported.contains(&export.target) {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "a dependency proxy cannot be re-exported in release format 2",
                )
                .for_node(export.target));
            }
            if ReleaseExportKind::for_node(source.node(export.target)?).is_none() {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "release export must target a function or nominal declaration",
                )
                .for_node(export.target));
            }
            Ok((export.name.clone(), export.target))
        })
        .collect::<Result<Vec<_>>>()?;
    exports.sort_by(|left, right| left.0.cmp(&right.0));
    if exports.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "release contains a duplicate export name",
        ));
    }
    Ok(exports)
}

fn canonical_tests(
    request: &ReleaseBuildRequest,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<Vec<CanonicalReleaseTest>> {
    let mut total_fuel = 0_u64;
    let mut tests = request
        .tests
        .iter()
        .map(|test| {
            super::validate_release_test_name(&test.name)?;
            interpret::validate_policy(test.policy)?;
            total_fuel = total_fuel.checked_add(test.policy.fuel).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "release test suite fuel overflows",
                )
            })?;
            if !test.arguments.iter().all(super::primitive_runtime_value) {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "release format 2 test arguments must be primitive values",
                ));
            }
            if let ReleaseTestExpectation::Value(value) = &test.expected
                && !super::primitive_runtime_value(value)
            {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "release format 2 test expectations must be primitive values",
                ));
            }
            let expected = match test.expected.clone() {
                ReleaseTestExpectation::Value(value) => ReleaseTestExpectation::Value(value),
                ReleaseTestExpectation::Trap(mut trap) => {
                    trap.target = trap.target.map(|target| map_id(map, target)).transpose()?;
                    ReleaseTestExpectation::Trap(trap)
                }
            };
            Ok(CanonicalReleaseTest {
                name: test.name.clone(),
                target: release_item(map_id(map, test.target)?)?,
                arguments: test.arguments.clone(),
                expected,
                policy: test.policy,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if total_fuel > MAXIMUM_RELEASE_SUITE_FUEL {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "release test suite fuel exceeds policy",
        ));
    }
    tests.sort_by(|left, right| left.name.cmp(&right.name));
    if tests.windows(2).any(|pair| pair[0].name == pair[1].name) {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "release contains a duplicate test name",
        ));
    }
    Ok(tests)
}

fn ensure_one_package(source: &Snapshot, root: NodeId, selected: &BTreeSet<NodeId>) -> Result<()> {
    for id in selected {
        if matches!(source.node(*id)?, Node::Package { .. }) && *id != root {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "a reusable release closure cannot cross its selected package boundary",
            )
            .for_node(*id));
        }
        let mut current = *id;
        let mut package = None;
        loop {
            let node = source.node(current)?;
            if matches!(node, Node::Package { .. }) {
                package = Some(current);
                break;
            }
            let Some(owner) = node.owner() else { break };
            current = owner;
        }
        if package.is_some_and(|package| package != root) {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "a reusable release reference escapes its selected package",
            )
            .for_node(*id));
        }
    }
    Ok(())
}

fn semantic_closure_ids(snapshot: &Snapshot, roots: &[NodeId]) -> Result<BTreeSet<NodeId>> {
    let mut selected = BTreeSet::new();
    let mut pending = roots.iter().copied().collect::<BTreeSet<_>>();
    while let Some(target) = pending.pop_first() {
        let definition = closure_definition(snapshot, target)?;
        let mut stack = vec![definition];
        let mut added = Vec::new();
        while let Some(id) = stack.pop() {
            if !selected.insert(id) {
                continue;
            }
            let node = snapshot.node(id)?;
            added.push(id);
            for index in (0..node.owned_child_count()).rev() {
                if let Some(child) = node.owned_child(index) {
                    stack.push(child);
                }
            }
        }
        for id in added {
            let node = snapshot.node(id)?;
            for index in 0..node.direct_reference_count() {
                let reference = node.direct_reference(index).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "semantic direct-reference count is inconsistent",
                    )
                    .for_node(id)
                })?;
                match reference {
                    DirectReference::Definition { target }
                    | DirectReference::Type { target, .. } => {
                        let dependency = closure_definition(snapshot, target)?;
                        if !selected.contains(&dependency) {
                            pending.insert(dependency);
                        }
                    }
                    DirectReference::ValueOperand { value, .. } => {
                        let value = value.referenced_node();
                        if !selected.contains(&value) {
                            return Err(LkError::new(
                                ErrorCode::InvalidContainment,
                                "release function contains a foreign value reference",
                            )
                            .for_node(value)
                            .with_related([id]));
                        }
                    }
                }
            }
        }
    }
    let semantic = selected.iter().copied().collect::<Vec<_>>();
    for id in semantic {
        let mut current = snapshot.node(id)?.owner();
        while let Some(owner) = current {
            selected.insert(owner);
            current = snapshot.node(owner)?.owner();
        }
    }
    selected.insert(snapshot.root());
    if selected.len() > MAXIMUM_RELEASE_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "release semantic item count exceeds policy",
        ));
    }
    Ok(selected)
}

fn closure_definition(snapshot: &Snapshot, target: NodeId) -> Result<NodeId> {
    Ok(match snapshot.node(target)? {
        Node::Function { .. }
        | Node::ProductType { .. }
        | Node::SumType { .. }
        | Node::SequenceType { .. } => target,
        Node::ProductField { owner, .. } | Node::SumVariant { owner, .. } => *owner,
        node => {
            let mut current = node.owner();
            let mut function = None;
            while let Some(owner) = current {
                match snapshot.node(owner)? {
                    Node::Function { .. } => {
                        function = Some(owner);
                        break;
                    }
                    parent => current = parent.owner(),
                }
            }
            function.ok_or_else(|| {
                LkError::new(
                    ErrorCode::WrongKind,
                    "release closure root is not a function or nominal declaration",
                )
                .for_node(target)
            })?
        }
    })
}

fn build_id_map(
    source: &Snapshot,
    package: NodeId,
    selected: &BTreeSet<NodeId>,
) -> Result<BTreeMap<NodeId, NodeId>> {
    let mut map = BTreeMap::new();
    let mut next = 1_u64;
    assign_durable(&mut map, source.root(), &mut next)?;
    assign_durable(&mut map, package, &mut next)?;
    let Node::Package { modules, .. } = source.node(package)? else {
        unreachable!("package was validated")
    };
    let mut modules = modules
        .iter()
        .copied()
        .filter(|id| selected.contains(id))
        .collect::<Vec<_>>();
    sort_named(source, &mut modules)?;
    for module in modules {
        assign_durable(&mut map, module, &mut next)?;
        let Node::Module {
            types, functions, ..
        } = source.node(module)?
        else {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "release package contains a non-module",
            ));
        };
        let mut definitions = types
            .iter()
            .chain(functions)
            .copied()
            .filter(|id| selected.contains(id))
            .collect::<Vec<_>>();
        definitions.sort_by(|left, right| {
            definition_key(source, *left).cmp(&definition_key(source, *right))
        });
        for definition in definitions {
            assign_durable(&mut map, definition, &mut next)?;
            match source.node(definition)? {
                Node::ProductType { fields, .. } => {
                    for child in fields.iter().copied().filter(|id| selected.contains(id)) {
                        assign_durable(&mut map, child, &mut next)?;
                    }
                }
                Node::SumType { variants, .. } => {
                    for child in variants.iter().copied().filter(|id| selected.contains(id)) {
                        assign_durable(&mut map, child, &mut next)?;
                    }
                }
                Node::SequenceType { .. } => {}
                Node::Function {
                    parameters, body, ..
                } => {
                    for child in parameters
                        .iter()
                        .copied()
                        .filter(|id| selected.contains(id))
                    {
                        assign_durable(&mut map, child, &mut next)?;
                    }
                    if let Some(body) = body.filter(|body| selected.contains(body)) {
                        assign_function_locals(source, definition, body, selected, &mut map)?;
                    }
                }
                _ => {
                    return Err(LkError::new(
                        ErrorCode::InvalidContainment,
                        "release module contains an unsupported definition",
                    ));
                }
            }
        }
    }
    if map.len() != selected.len() {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "release closure contains an item outside its canonical ownership tree",
        ));
    }
    Ok(map)
}

fn assign_durable(
    map: &mut BTreeMap<NodeId, NodeId>,
    source: NodeId,
    next: &mut u64,
) -> Result<()> {
    let target = NodeId::new(RELEASE_LOCAL_WORKSPACE, *next).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("release-local identity allocation failed: {error}"),
        )
    })?;
    *next = next.checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "release-local identity allocation overflowed",
        )
    })?;
    if map.insert(source, target).is_some() {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "release canonicalizer assigned one source item twice",
        ));
    }
    Ok(())
}

fn assign_function_locals(
    source: &Snapshot,
    source_function: NodeId,
    body: NodeId,
    selected: &BTreeSet<NodeId>,
    map: &mut BTreeMap<NodeId, NodeId>,
) -> Result<()> {
    let target_function = map_id(map, source_function)?;
    let mut ordinal = 1_u32;
    let mut stack = vec![body];
    while let Some(id) = stack.pop() {
        if !selected.contains(&id) || map.contains_key(&id) {
            continue;
        }
        let target = NodeId::new_function_local(RELEASE_LOCAL_WORKSPACE, target_function, ordinal)
            .map_err(|error| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    format!("release function-local identity allocation failed: {error}"),
                )
            })?;
        ordinal = ordinal.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "release function-local identity allocation overflowed",
            )
        })?;
        map.insert(id, target);
        let node = source.node(id)?;
        for index in (0..node.owned_child_count()).rev() {
            if let Some(child) = node.owned_child(index) {
                stack.push(child);
            }
        }
    }
    Ok(())
}

fn sort_named(source: &Snapshot, values: &mut [NodeId]) -> Result<()> {
    let mut keyed = values
        .iter()
        .copied()
        .map(|id| {
            source
                .node(id)?
                .name()
                .map(|name| (name.to_owned(), id))
                .ok_or_else(|| {
                    LkError::new(ErrorCode::WrongKind, "release item has no canonical name")
                        .for_node(id)
                })
        })
        .collect::<Result<Vec<_>>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    for (slot, (_, id)) in values.iter_mut().zip(keyed) {
        *slot = id;
    }
    Ok(())
}

fn definition_key(source: &Snapshot, id: NodeId) -> (u8, String) {
    source.node(id).map_or((u8::MAX, String::new()), |node| {
        (
            match node {
                Node::ProductType { .. } => 1,
                Node::SumType { .. } => 2,
                Node::SequenceType { .. } => 3,
                Node::Function { .. } => 4,
                _ => u8::MAX,
            },
            node.name().unwrap_or_default().to_owned(),
        )
    })
}

fn remap_snapshot(
    source: &Snapshot,
    package: NodeId,
    selected: &BTreeSet<NodeId>,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<Snapshot> {
    let mut nodes = BTreeMap::new();
    for source_id in selected {
        let target = map_id(map, *source_id)?;
        let node = normalize_and_remap(source.node(*source_id)?.clone(), selected, map)?;
        nodes.insert(target, node);
    }
    let maximum = nodes
        .keys()
        .filter(|id| id.is_durable())
        .map(|id| id.serial())
        .max()
        .unwrap_or(0);
    let snapshot = Snapshot::from_parts(
        RELEASE_LOCAL_WORKSPACE,
        Revision::INITIAL,
        map_id(map, source.root())?,
        maximum.checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "release identity frontier overflowed",
            )
        })?,
        BTreeSet::new(),
        nodes,
    )?;
    let expected_package = map_id(map, package)?;
    match snapshot.node(snapshot.root())? {
        Node::WorkspaceRoot { packages, targets }
            if packages == &[expected_package] && targets.is_empty() => {}
        _ => {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "canonical release root does not contain exactly its selected unit",
            ));
        }
    }
    Ok(snapshot)
}

fn normalize_and_remap(
    mut node: Node,
    selected: &BTreeSet<NodeId>,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<Node> {
    match &mut node {
        Node::WorkspaceRoot { packages, targets } => {
            retain_and_map(packages, selected, map)?;
            targets.clear();
        }
        Node::BuildTarget { .. } => {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "build targets cannot enter reusable release meaning",
            ));
        }
        Node::Package {
            owner,
            modules,
            entry,
            ..
        } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(modules, selected, map)?;
            modules.sort_by(|left, right| {
                name_for_mapped(map, *left).cmp(&name_for_mapped(map, *right))
            });
            *entry = None;
        }
        Node::Module {
            owner,
            types,
            functions,
            ..
        } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(types, selected, map)?;
            retain_and_map(functions, selected, map)?;
            types.sort();
            functions.sort();
        }
        Node::ProductType { owner, fields, .. } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(fields, selected, map)?;
        }
        Node::ProductField { owner, ty, .. } => {
            *owner = map_id(map, *owner)?;
            *ty = remap_type(*ty, map)?;
        }
        Node::SumType {
            owner, variants, ..
        } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(variants, selected, map)?;
        }
        Node::SumVariant { owner, payload, .. } => {
            *owner = map_id(map, *owner)?;
            *payload = payload.map(|ty| remap_type(ty, map)).transpose()?;
        }
        Node::SequenceType { owner, element, .. } => {
            *owner = map_id(map, *owner)?;
            *element = remap_type(*element, map)?;
        }
        Node::Function {
            owner,
            parameters,
            result,
            body,
            ..
        } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(parameters, selected, map)?;
            *result = remap_type(*result, map)?;
            *body = body
                .filter(|id| selected.contains(id))
                .map(|id| map_id(map, id))
                .transpose()?;
        }
        Node::Parameter { owner, ty, .. } | Node::BlockArgument { owner, ty, .. } => {
            *owner = map_id(map, *owner)?;
            *ty = remap_type(*ty, map)?;
        }
        Node::Region { owner, blocks } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(blocks, selected, map)?;
        }
        Node::Block {
            owner,
            arguments,
            operations,
            terminator,
        } => {
            *owner = map_id(map, *owner)?;
            retain_and_map(arguments, selected, map)?;
            retain_and_map(operations, selected, map)?;
            *terminator = terminator
                .filter(|id| selected.contains(id))
                .map(|id| map_id(map, id))
                .transpose()?;
        }
        Node::Operation { owner, operation } => {
            *owner = map_id(map, *owner)?;
            *operation = remap_operation(operation.clone(), map)?;
        }
    }
    Ok(node)
}

fn retain_and_map(
    values: &mut Vec<NodeId>,
    selected: &BTreeSet<NodeId>,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<()> {
    *values = values
        .iter()
        .copied()
        .filter(|id| selected.contains(id))
        .map(|id| map_id(map, id))
        .collect::<Result<Vec<_>>>()?;
    Ok(())
}

// The target IDs themselves already encode the canonical name order; this helper keeps the
// normalization expression explicit without re-reading source nodes after remapping.
const fn name_for_mapped(_map: &BTreeMap<NodeId, NodeId>, id: NodeId) -> u64 {
    id.serial()
}

pub(super) fn remap_node_with(
    node: Node,
    mut remap: impl FnMut(NodeId) -> Result<NodeId>,
) -> Result<Node> {
    fn ty(
        value: SemanticType,
        remap: &mut impl FnMut(NodeId) -> Result<NodeId>,
    ) -> Result<SemanticType> {
        Ok(match value {
            SemanticType::Nominal(target) => SemanticType::Nominal(remap(target)?),
            primitive => primitive,
        })
    }
    fn value(
        value: ValueRef,
        remap: &mut impl FnMut(NodeId) -> Result<NodeId>,
    ) -> Result<ValueRef> {
        Ok(match value {
            ValueRef::FunctionParameter(id) => ValueRef::FunctionParameter(remap(id)?),
            ValueRef::BlockArgument(id) => ValueRef::BlockArgument(remap(id)?),
            ValueRef::OperationResult { operation, output } => ValueRef::OperationResult {
                operation: remap(operation)?,
                output,
            },
        })
    }
    fn operation(
        operation: OperationKind,
        remap: &mut impl FnMut(NodeId) -> Result<NodeId>,
    ) -> Result<OperationKind> {
        Ok(match operation {
            OperationKind::ConstUnit => OperationKind::ConstUnit,
            OperationKind::ConstI64(value) => OperationKind::ConstI64(value),
            OperationKind::ConstBool(value) => OperationKind::ConstBool(value),
            OperationKind::ConstBytes(value) => OperationKind::ConstBytes(value),
            OperationKind::ConstText(value) => OperationKind::ConstText(value),
            OperationKind::AddI64 { lhs, rhs } => OperationKind::AddI64 {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::LtI64 { lhs, rhs } => OperationKind::LtI64 {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::EqualI64 { lhs, rhs } => OperationKind::EqualI64 {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::NotBool { value: input } => OperationKind::NotBool {
                value: value(input, remap)?,
            },
            OperationKind::AndBool { lhs, rhs } => OperationKind::AndBool {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::OrBool { lhs, rhs } => OperationKind::OrBool {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::BytesLen { value: input } => OperationKind::BytesLen {
                value: value(input, remap)?,
            },
            OperationKind::BytesAt {
                value: input,
                index,
            } => OperationKind::BytesAt {
                value: value(input, remap)?,
                index: value(index, remap)?,
            },
            OperationKind::BytesSlice {
                value: input,
                start,
                length,
            } => OperationKind::BytesSlice {
                value: value(input, remap)?,
                start: value(start, remap)?,
                length: value(length, remap)?,
            },
            OperationKind::BytesEqual { lhs, rhs } => OperationKind::BytesEqual {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::BytesConcat { lhs, rhs } => OperationKind::BytesConcat {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::TextLen { value: input } => OperationKind::TextLen {
                value: value(input, remap)?,
            },
            OperationKind::TextEqual { lhs, rhs } => OperationKind::TextEqual {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::TextConcat { lhs, rhs } => OperationKind::TextConcat {
                lhs: value(lhs, remap)?,
                rhs: value(rhs, remap)?,
            },
            OperationKind::SequenceEmpty { sequence } => OperationKind::SequenceEmpty {
                sequence: remap(sequence)?,
            },
            OperationKind::SequenceLen {
                sequence,
                value: input,
            } => OperationKind::SequenceLen {
                sequence: remap(sequence)?,
                value: value(input, remap)?,
            },
            OperationKind::SequenceGet {
                sequence,
                value: input,
                index,
            } => OperationKind::SequenceGet {
                sequence: remap(sequence)?,
                value: value(input, remap)?,
                index: value(index, remap)?,
            },
            OperationKind::SequenceAppend {
                sequence,
                value: input,
                element,
            } => OperationKind::SequenceAppend {
                sequence: remap(sequence)?,
                value: value(input, remap)?,
                element: value(element, remap)?,
            },
            OperationKind::SequenceReplace {
                sequence,
                value: input,
                index,
                element,
            } => OperationKind::SequenceReplace {
                sequence: remap(sequence)?,
                value: value(input, remap)?,
                index: value(index, remap)?,
                element: value(element, remap)?,
            },
            OperationKind::Call {
                function,
                arguments,
            } => OperationKind::Call {
                function: remap(function)?,
                arguments: arguments
                    .into_iter()
                    .map(|item| value(item, remap))
                    .collect::<Result<_>>()?,
            },
            OperationKind::Hole { expected } => OperationKind::Hole {
                expected: ty(expected, remap)?,
            },
            OperationKind::If {
                condition,
                result,
                then_region,
                else_region,
            } => OperationKind::If {
                condition: value(condition, remap)?,
                result: ty(result, remap)?,
                then_region: remap(then_region)?,
                else_region: remap(else_region)?,
            },
            OperationKind::ForI64 {
                start,
                end_exclusive,
                step,
                initial,
                carried,
                body_region,
            } => OperationKind::ForI64 {
                start: value(start, remap)?,
                end_exclusive: value(end_exclusive, remap)?,
                step,
                initial: value(initial, remap)?,
                carried: ty(carried, remap)?,
                body_region: remap(body_region)?,
            },
            OperationKind::Return { value: input } => OperationKind::Return {
                value: value(input, remap)?,
            },
            OperationKind::Yield { value: input } => OperationKind::Yield {
                value: value(input, remap)?,
            },
            OperationKind::ConstructProduct { product, fields } => {
                OperationKind::ConstructProduct {
                    product: remap(product)?,
                    fields: fields
                        .into_iter()
                        .map(|field| {
                            Ok(ProductFieldValue {
                                field: remap(field.field)?,
                                value: value(field.value, remap)?,
                            })
                        })
                        .collect::<Result<_>>()?,
                }
            }
            OperationKind::ProjectField {
                value: input,
                field,
            } => OperationKind::ProjectField {
                value: value(input, remap)?,
                field: remap(field)?,
            },
            OperationKind::ConstructVariant { variant, payload } => {
                OperationKind::ConstructVariant {
                    variant: remap(variant)?,
                    payload: payload.map(|item| value(item, remap)).transpose()?,
                }
            }
            OperationKind::MatchSum {
                scrutinee,
                result,
                arms,
            } => OperationKind::MatchSum {
                scrutinee: value(scrutinee, remap)?,
                result: ty(result, remap)?,
                arms: arms
                    .into_iter()
                    .map(|arm| {
                        Ok(MatchArm {
                            variant: remap(arm.variant)?,
                            region: remap(arm.region)?,
                        })
                    })
                    .collect::<Result<_>>()?,
            },
        })
    }
    Ok(match node {
        Node::WorkspaceRoot { packages, .. } => Node::WorkspaceRoot {
            packages: packages
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
            targets: Vec::new(),
        },
        Node::BuildTarget { .. } => {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "build targets cannot enter reusable release meaning",
            ));
        }
        Node::Package {
            owner,
            name,
            modules,
            entry,
        } => Node::Package {
            owner: remap(owner)?,
            name,
            modules: modules.into_iter().map(&mut remap).collect::<Result<_>>()?,
            entry: entry.map(&mut remap).transpose()?,
        },
        Node::Module {
            owner,
            name,
            types,
            functions,
        } => Node::Module {
            owner: remap(owner)?,
            name,
            types: types.into_iter().map(&mut remap).collect::<Result<_>>()?,
            functions: functions
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
        },
        Node::ProductType {
            owner,
            name,
            fields,
        } => Node::ProductType {
            owner: remap(owner)?,
            name,
            fields: fields.into_iter().map(&mut remap).collect::<Result<_>>()?,
        },
        Node::ProductField {
            owner,
            ordinal,
            name,
            ty: field_ty,
        } => Node::ProductField {
            owner: remap(owner)?,
            ordinal,
            name,
            ty: ty(field_ty, &mut remap)?,
        },
        Node::SumType {
            owner,
            name,
            variants,
        } => Node::SumType {
            owner: remap(owner)?,
            name,
            variants: variants
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
        },
        Node::SumVariant {
            owner,
            ordinal,
            name,
            payload,
        } => Node::SumVariant {
            owner: remap(owner)?,
            ordinal,
            name,
            payload: payload.map(|item| ty(item, &mut remap)).transpose()?,
        },
        Node::SequenceType {
            owner,
            name,
            element,
        } => Node::SequenceType {
            owner: remap(owner)?,
            name,
            element: ty(element, &mut remap)?,
        },
        Node::Function {
            owner,
            name,
            parameters,
            result,
            body,
        } => Node::Function {
            owner: remap(owner)?,
            name,
            parameters: parameters
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
            result: ty(result, &mut remap)?,
            body: body.map(&mut remap).transpose()?,
        },
        Node::Parameter {
            owner,
            ordinal,
            name,
            ty: parameter_ty,
        } => Node::Parameter {
            owner: remap(owner)?,
            ordinal,
            name,
            ty: ty(parameter_ty, &mut remap)?,
        },
        Node::Region { owner, blocks } => Node::Region {
            owner: remap(owner)?,
            blocks: blocks.into_iter().map(&mut remap).collect::<Result<_>>()?,
        },
        Node::Block {
            owner,
            arguments,
            operations,
            terminator,
        } => Node::Block {
            owner: remap(owner)?,
            arguments: arguments
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
            operations: operations
                .into_iter()
                .map(&mut remap)
                .collect::<Result<_>>()?,
            terminator: terminator.map(&mut remap).transpose()?,
        },
        Node::BlockArgument {
            owner,
            ordinal,
            ty: argument_ty,
        } => Node::BlockArgument {
            owner: remap(owner)?,
            ordinal,
            ty: ty(argument_ty, &mut remap)?,
        },
        Node::Operation {
            owner,
            operation: item,
        } => Node::Operation {
            owner: remap(owner)?,
            operation: operation(item, &mut remap)?,
        },
    })
}

fn remap_operation(
    operation: OperationKind,
    map: &BTreeMap<NodeId, NodeId>,
) -> Result<OperationKind> {
    let node = Node::Operation {
        owner: *map.keys().next().ok_or_else(|| {
            LkError::new(ErrorCode::ArtifactCorrupt, "empty release identity map")
        })?,
        operation,
    };
    match remap_node_with(node, |id| map_id(map, id))? {
        Node::Operation { operation, .. } => Ok(operation),
        _ => unreachable!("operation remapper preserves node kind"),
    }
}

fn remap_type(ty: SemanticType, map: &BTreeMap<NodeId, NodeId>) -> Result<SemanticType> {
    Ok(match ty {
        SemanticType::Nominal(target) => SemanticType::Nominal(map_id(map, target)?),
        primitive => primitive,
    })
}

fn map_id(map: &BTreeMap<NodeId, NodeId>, id: NodeId) -> Result<NodeId> {
    map.get(&id).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::InvalidContainment,
            "release closure omitted a referenced semantic item",
        )
        .for_node(id)
    })
}

fn release_item(id: NodeId) -> Result<ReleaseItemId> {
    ReleaseItemId::from_local_node(id)
}

pub(super) fn validate_release_model(release: &DecodedRelease, decoded: bool) -> Result<()> {
    let error_code = if decoded {
        ErrorCode::ArtifactCorrupt
    } else {
        ErrorCode::ProtocolMalformed
    };
    super::validate_coordinate(&release.coordinate).map_err(|mut error| {
        error.code = error_code;
        error
    })?;
    super::validate_user_version(&release.user_version).map_err(|mut error| {
        error.code = error_code;
        error
    })?;
    if release.snapshot.workspace() != RELEASE_LOCAL_WORKSPACE
        || release.snapshot.revision() != Revision::INITIAL
        || release.snapshot.node_count() > MAXIMUM_RELEASE_ITEMS
    {
        return Err(LkError::new(
            error_code,
            "release semantic closure has an invalid local authority or size",
        ));
    }
    let unit = release.unit_root.to_local_node()?;
    if !matches!(
        release.snapshot.node(unit)?,
        Node::Package { entry: None, .. }
    ) {
        return Err(LkError::new(
            error_code,
            "release unit root is not a package without an application entry",
        ));
    }
    let dependency_slots = release
        .dependencies
        .iter()
        .map(|item| item.slot.as_str())
        .collect::<BTreeSet<_>>();
    if dependency_slots.len() != release.dependencies.len()
        || release.dependencies.len() > MAXIMUM_RELEASE_DEPENDENCIES
    {
        return Err(LkError::new(
            error_code,
            "release dependency slots are duplicate or oversized",
        ));
    }
    let imported = release
        .imports
        .iter()
        .map(|item| item.local)
        .collect::<BTreeSet<_>>();
    if imported.len() != release.imports.len() || release.imports.len() > MAXIMUM_RELEASE_IMPORTS {
        return Err(LkError::new(
            error_code,
            "release imports are duplicate or oversized",
        ));
    }
    for import in &release.imports {
        if !dependency_slots.contains(import.dependency_slot.as_str()) {
            return Err(LkError::new(
                error_code,
                "release import names an undeclared dependency slot",
            ));
        }
        let node = release.snapshot.node(import.local.to_local_node()?)?;
        if ReleaseExportKind::for_node(node).is_none() {
            return Err(LkError::new(
                error_code,
                "release import proxy has an unsupported kind",
            ));
        }
        if matches!(node, Node::Function { body: Some(_), .. }) {
            return Err(LkError::new(
                error_code,
                "imported function proxy must not contain a local body",
            ));
        }
    }
    let export_names = release
        .exports
        .iter()
        .map(|item| item.name.as_str())
        .collect::<BTreeSet<_>>();
    if export_names.len() != release.exports.len()
        || release.exports.is_empty()
        || release.exports.len() > MAXIMUM_RELEASE_EXPORTS
    {
        return Err(LkError::new(
            error_code,
            "release exports are empty, duplicate, or oversized",
        ));
    }
    let exported = release
        .exports
        .iter()
        .map(|item| item.target)
        .collect::<BTreeSet<_>>();
    for export in &release.exports {
        if imported.contains(&export.target) {
            return Err(LkError::new(
                error_code,
                "release directly exports a dependency proxy",
            ));
        }
        let node = release.snapshot.node(export.target.to_local_node()?)?;
        if ReleaseExportKind::for_node(node) != Some(export.kind) {
            return Err(LkError::new(
                error_code,
                "release export kind does not match its target",
            ));
        }
        validate_public_signature(release, export.target, &exported, &imported, error_code)?;
    }
    let test_names = release
        .tests
        .iter()
        .map(|item| item.name.as_str())
        .collect::<BTreeSet<_>>();
    if test_names.len() != release.tests.len()
        || release.tests.is_empty()
        || release.tests.len() > MAXIMUM_RELEASE_TESTS
    {
        return Err(LkError::new(
            error_code,
            "release tests are empty, duplicate, or oversized",
        ));
    }
    let mut total_fuel = 0_u64;
    for test in &release.tests {
        super::validate_release_test_name(&test.name).map_err(|mut error| {
            error.code = error_code;
            error
        })?;
        total_fuel = total_fuel
            .checked_add(test.policy.fuel)
            .ok_or_else(|| LkError::new(error_code, "release test suite fuel overflows"))?;
        let target = test.target.to_local_node()?;
        if imported.contains(&test.target)
            || !matches!(release.snapshot.node(target)?, Node::Function { .. })
        {
            return Err(LkError::new(
                error_code,
                "release test must target a local function",
            ));
        }
        if !test.arguments.iter().all(super::primitive_runtime_value) {
            return Err(LkError::new(
                error_code,
                "release test arguments are not primitive",
            ));
        }
        if let ReleaseTestExpectation::Value(value) = &test.expected
            && !super::primitive_runtime_value(value)
        {
            return Err(LkError::new(
                error_code,
                "release test expected value is not primitive",
            ));
        }
    }
    if total_fuel > MAXIMUM_RELEASE_SUITE_FUEL {
        return Err(LkError::new(
            error_code,
            "release test suite fuel exceeds policy",
        ));
    }
    for (id, node) in release.snapshot.nodes() {
        if let Node::Operation {
            operation: OperationKind::Hole { .. },
            ..
        } = node
        {
            return Err(
                LkError::new(error_code, "release closure contains a reachable hole").for_node(id),
            );
        }
        if let Node::Function { body: None, .. } = node
            && !imported.contains(&release_item(id)?)
        {
            return Err(LkError::new(
                error_code,
                "release closure contains an incomplete local function",
            )
            .for_node(id));
        }
    }
    Ok(())
}

fn validate_public_signature(
    release: &DecodedRelease,
    target: ReleaseItemId,
    exported: &BTreeSet<ReleaseItemId>,
    imported: &BTreeSet<ReleaseItemId>,
    code: ErrorCode,
) -> Result<()> {
    let mut types = Vec::new();
    match release.snapshot.node(target.to_local_node()?)? {
        Node::Function {
            parameters, result, ..
        } => {
            for parameter in parameters {
                let Node::Parameter { ty, .. } = release.snapshot.node(*parameter)? else {
                    return Err(LkError::new(
                        code,
                        "release function parameter is malformed",
                    ));
                };
                types.push(*ty);
            }
            types.push(*result);
        }
        Node::ProductType { fields, .. } => {
            for field in fields {
                let Node::ProductField { ty, .. } = release.snapshot.node(*field)? else {
                    return Err(LkError::new(code, "release product field is malformed"));
                };
                types.push(*ty);
            }
        }
        Node::SumType { variants, .. } => {
            for variant in variants {
                let Node::SumVariant { payload, .. } = release.snapshot.node(*variant)? else {
                    return Err(LkError::new(code, "release sum variant is malformed"));
                };
                types.extend(*payload);
            }
        }
        Node::SequenceType { element, .. } => types.push(*element),
        _ => return Err(LkError::new(code, "release export kind is unsupported")),
    }
    for ty in types {
        if let SemanticType::Nominal(id) = ty {
            let item = release_item(id)?;
            if !exported.contains(&item) && !imported.contains(&item) {
                return Err(LkError::new(
                    code,
                    "a private nominal type leaks through an exported signature",
                )
                .for_node(id));
            }
        }
    }
    Ok(())
}
