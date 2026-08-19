use super::{
    CanonicalReleaseTest, DecodedRelease, MAXIMUM_RELEASE_GRAPH_BYTES, MAXIMUM_RELEASE_GRAPH_DEPTH,
    MAXIMUM_RELEASE_GRAPH_EDGES, MAXIMUM_RELEASE_GRAPH_NODES, ReleaseExportKind, ReleaseId,
    ReleaseItemId, ReleaseTestExpectation, ReleaseTestReport, ReleaseTestResult, ReleaseTestStatus,
    ReleaseTrapCode, canonical,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::interpret;
use crate::schema::{Node, SemanticType};
use std::collections::{BTreeMap, BTreeSet};

const FLATTENED_GRAPH_DOMAIN: &str = "lkjscript.release-graph.flattened-workspace.v1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct GlobalItem {
    pub release: ReleaseId,
    pub item: ReleaseItemId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct GlobalNode {
    release: ReleaseId,
    serial: u64,
}

#[derive(Clone, Debug)]
pub(crate) struct ReleaseGraph {
    root: ReleaseId,
    releases: BTreeMap<ReleaseId, DecodedRelease>,
    redirects: BTreeMap<GlobalItem, GlobalItem>,
    edges: usize,
    depth: usize,
    aggregate_bytes: usize,
}

#[derive(Clone, Debug)]
pub(crate) struct FlattenedGraph {
    pub snapshot: Snapshot,
    node_map: BTreeMap<GlobalNode, NodeId>,
}

impl FlattenedGraph {
    pub(crate) fn item(&self, release: ReleaseId, item: ReleaseItemId) -> Result<NodeId> {
        self.node_map
            .get(&GlobalNode {
                release,
                serial: item.get(),
            })
            .copied()
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::NodeNotFound,
                    format!("exact release {release} item {} is absent", item.get()),
                )
            })
    }

    fn node(&self, release: ReleaseId, node: NodeId) -> Result<NodeId> {
        self.node_map
            .get(&GlobalNode {
                release,
                serial: node.serial(),
            })
            .copied()
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::NodeNotFound,
                    format!("exact release {release} semantic node is absent after composition"),
                )
            })
    }

    pub(crate) fn global_item(&self, node: NodeId) -> Option<GlobalItem> {
        if !node.is_durable() {
            return None;
        }
        self.node_map.iter().find_map(|(source, target)| {
            (*target == node && source.serial & (1_u64 << 63) == 0).then(|| {
                ReleaseItemId::new(source.serial)
                    .ok()
                    .map(|item| GlobalItem {
                        release: source.release,
                        item,
                    })
            })?
        })
    }
}

impl ReleaseGraph {
    pub(crate) fn new(root: DecodedRelease, supplied: Vec<DecodedRelease>) -> Result<Self> {
        let root_id = root.id;
        let mut all = BTreeMap::new();
        insert_exact(&mut all, root)?;
        for release in supplied {
            insert_exact(&mut all, release)?;
        }
        if all.len() > MAXIMUM_RELEASE_GRAPH_NODES {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "exact release graph node count exceeds policy",
            ));
        }

        let mut reachable = BTreeSet::new();
        let mut pending = vec![root_id];
        let mut edges = 0_usize;
        while let Some(id) = pending.pop() {
            if !reachable.insert(id) {
                continue;
            }
            let release = all.get(&id).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    format!("exact release graph is missing release {id}"),
                )
            })?;
            edges = edges
                .checked_add(release.dependencies.len())
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "exact release graph edge count overflows",
                    )
                })?;
            if edges > MAXIMUM_RELEASE_GRAPH_EDGES {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "exact release graph edge count exceeds policy",
                ));
            }
            for dependency in release.dependencies.iter().rev() {
                if !all.contains_key(&dependency.release) {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        format!(
                            "exact release {} is missing dependency {} for slot '{}'",
                            id, dependency.release, dependency.slot
                        ),
                    ));
                }
                pending.push(dependency.release);
            }
        }
        if reachable.len() != all.len() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "exact release graph contains an unrelated supplied release",
            ));
        }
        detect_cycles(root_id, &all)?;
        let maximum_depth = longest_depth(root_id, &all)?;

        let mut releases = BTreeMap::new();
        let mut aggregate_bytes = 0_usize;
        for id in reachable {
            let release = all.remove(&id).ok_or_else(|| {
                LkError::new(ErrorCode::ArtifactCorrupt, "release graph node disappeared")
            })?;
            aggregate_bytes = checked_graph_bytes(aggregate_bytes, release.bytes.len())?;
            releases.insert(id, release);
        }
        let redirects = validate_imports(&releases)?;
        Ok(Self {
            root: root_id,
            releases,
            redirects,
            edges,
            depth: maximum_depth,
            aggregate_bytes,
        })
    }

    pub(crate) const fn root(&self) -> ReleaseId {
        self.root
    }

    pub(crate) fn release(&self, id: ReleaseId) -> Result<&DecodedRelease> {
        self.releases.get(&id).ok_or_else(|| {
            LkError::new(
                ErrorCode::NodeNotFound,
                format!("exact release {id} is absent from the graph"),
            )
        })
    }

    pub(crate) fn releases(&self) -> impl ExactSizeIterator<Item = &DecodedRelease> {
        self.releases.values()
    }

    pub(crate) const fn edge_count(&self) -> usize {
        self.edges
    }

    pub(crate) const fn depth(&self) -> usize {
        self.depth
    }

    pub(crate) const fn aggregate_bytes(&self) -> usize {
        self.aggregate_bytes
    }

    pub(crate) fn flatten(&self) -> Result<FlattenedGraph> {
        let workspace = flattened_workspace(self.root, self.releases.keys().copied());
        let root = NodeId::new(workspace, 1).map_err(identity_error)?;
        let imported_subtrees = imported_subtrees(&self.releases)?;
        let mut node_map = BTreeMap::new();
        for release in self.releases.values() {
            node_map.insert(
                GlobalNode {
                    release: release.id,
                    serial: release.snapshot.root().serial(),
                },
                root,
            );
        }

        let mut next = 2_u64;
        for release in self.releases.values() {
            for (id, _) in release.snapshot.nodes().filter(|(id, _)| id.is_durable()) {
                if id == release.snapshot.root()
                    || imported_subtrees.contains(&GlobalItem {
                        release: release.id,
                        item: ReleaseItemId::new(id.serial())?,
                    })
                {
                    continue;
                }
                let target = NodeId::new(workspace, next).map_err(identity_error)?;
                next = next.checked_add(1).ok_or_else(|| {
                    LkError::new(
                        ErrorCode::PolicyExceeded,
                        "flattened release graph identity frontier overflows",
                    )
                })?;
                node_map.insert(
                    GlobalNode {
                        release: release.id,
                        serial: id.serial(),
                    },
                    target,
                );
            }
        }
        for release in self.releases.values() {
            for (id, _) in release
                .snapshot
                .nodes()
                .filter(|(id, _)| id.is_function_local())
            {
                let function = id.local_function_serial().ok_or_else(|| {
                    LkError::new(ErrorCode::ArtifactCorrupt, "release local owner is invalid")
                })?;
                if imported_subtrees.contains(&GlobalItem {
                    release: release.id,
                    item: ReleaseItemId::new(function)?,
                }) {
                    continue;
                }
                let owner = resolve_global(
                    &node_map,
                    &self.redirects,
                    GlobalNode {
                        release: release.id,
                        serial: function,
                    },
                )?;
                let target = NodeId::new_function_local(
                    workspace,
                    owner,
                    id.local_ordinal().ok_or_else(|| {
                        LkError::new(
                            ErrorCode::ArtifactCorrupt,
                            "release local ordinal is invalid",
                        )
                    })?,
                )
                .map_err(identity_error)?;
                node_map.insert(
                    GlobalNode {
                        release: release.id,
                        serial: id.serial(),
                    },
                    target,
                );
            }
        }

        let package_nodes = self
            .releases
            .values()
            .map(|release| {
                resolve_global(
                    &node_map,
                    &self.redirects,
                    GlobalNode {
                        release: release.id,
                        serial: release.unit_root.get(),
                    },
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            Node::WorkspaceRoot {
                packages: package_nodes,
                targets: Vec::new(),
            },
        );
        for release in self.releases.values() {
            for (id, node) in release.snapshot.nodes() {
                if id == release.snapshot.root()
                    || (id.is_durable()
                        && imported_subtrees.contains(&GlobalItem {
                            release: release.id,
                            item: ReleaseItemId::new(id.serial())?,
                        }))
                    || (id.is_function_local()
                        && imported_subtrees.contains(&GlobalItem {
                            release: release.id,
                            item: ReleaseItemId::new(id.local_function_serial().ok_or_else(
                                || {
                                    LkError::new(
                                        ErrorCode::ArtifactCorrupt,
                                        "release local owner is invalid",
                                    )
                                },
                            )?)?,
                        }))
                {
                    continue;
                }
                let target = resolve_global(
                    &node_map,
                    &self.redirects,
                    GlobalNode {
                        release: release.id,
                        serial: id.serial(),
                    },
                )?;
                let normalized = strip_imported_children(node.clone(), release.id, &self.redirects);
                let mut mapped = canonical::remap_node_with(normalized, |local| {
                    resolve_global(
                        &node_map,
                        &self.redirects,
                        GlobalNode {
                            release: release.id,
                            serial: local.serial(),
                        },
                    )
                })?;
                if let Node::Package { name, .. } = &mut mapped {
                    *name = format!("r_{}", release.id);
                }
                nodes.insert(target, mapped);
            }
        }
        let snapshot = Snapshot::from_parts(
            workspace,
            Revision::INITIAL,
            root,
            next,
            BTreeSet::new(),
            nodes,
        )?;
        Ok(FlattenedGraph { snapshot, node_map })
    }

    pub(crate) fn run_release_tests(&self, release: ReleaseId) -> Result<ReleaseTestReport> {
        let flattened = self.flatten()?;
        let selected = self.release(release)?;
        let mut results = Vec::with_capacity(selected.tests.len());
        let mut passed = 0_u64;
        for test in &selected.tests {
            let result = run_test(&flattened, release, test)?;
            if result.status == ReleaseTestStatus::Passed {
                passed = passed.saturating_add(1);
            }
            results.push(result);
        }
        Ok(ReleaseTestReport {
            total: u64::try_from(results.len()).unwrap_or(u64::MAX),
            passed,
            results,
        })
    }
}

pub(super) fn checked_graph_bytes(current: usize, added: usize) -> Result<usize> {
    let total = current.checked_add(added).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "exact release graph byte count overflows",
        )
    })?;
    if total > MAXIMUM_RELEASE_GRAPH_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "exact release graph aggregate bytes exceed policy",
        ));
    }
    Ok(total)
}

fn strip_imported_children(
    mut node: Node,
    release: ReleaseId,
    redirects: &BTreeMap<GlobalItem, GlobalItem>,
) -> Node {
    let imported = |id: &NodeId| {
        id.is_durable()
            && ReleaseItemId::new(id.serial())
                .is_ok_and(|item| redirects.contains_key(&GlobalItem { release, item }))
    };
    match &mut node {
        Node::Module {
            types, functions, ..
        } => {
            types.retain(|id| !imported(id));
            functions.retain(|id| !imported(id));
        }
        Node::WorkspaceRoot { .. }
        | Node::BuildTarget { .. }
        | Node::Package { .. }
        | Node::ProductType { .. }
        | Node::ProductField { .. }
        | Node::SumType { .. }
        | Node::SumVariant { .. }
        | Node::SequenceType { .. }
        | Node::Function { .. }
        | Node::Parameter { .. }
        | Node::Region { .. }
        | Node::Block { .. }
        | Node::BlockArgument { .. }
        | Node::Operation { .. } => {}
    }
    node
}

fn insert_exact(
    releases: &mut BTreeMap<ReleaseId, DecodedRelease>,
    candidate: DecodedRelease,
) -> Result<()> {
    if let Some(existing) = releases.get(&candidate.id) {
        if existing.bytes != candidate.bytes {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!(
                    "exact release identity {} is claimed by conflicting bytes",
                    candidate.id
                ),
            ));
        }
        return Ok(());
    }
    releases.insert(candidate.id, candidate);
    Ok(())
}

fn detect_cycles(root: ReleaseId, releases: &BTreeMap<ReleaseId, DecodedRelease>) -> Result<()> {
    let mut state = BTreeMap::<ReleaseId, u8>::new();
    let mut stack = vec![(root, 0_usize)];
    while let Some((id, index)) = stack.last_mut() {
        state.entry(*id).or_insert(1);
        let release = releases.get(id).ok_or_else(|| {
            LkError::new(ErrorCode::ArtifactCorrupt, "release graph node is missing")
        })?;
        if *index < release.dependencies.len() {
            let child = release.dependencies[*index].release;
            *index += 1;
            match state.get(&child).copied().unwrap_or(0) {
                0 => stack.push((child, 0)),
                1 => {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        format!("exact release dependency graph contains a cycle at {child}"),
                    ));
                }
                _ => {}
            }
        } else {
            state.insert(*id, 2);
            stack.pop();
        }
    }
    Ok(())
}

fn longest_depth(root: ReleaseId, releases: &BTreeMap<ReleaseId, DecodedRelease>) -> Result<usize> {
    let mut visited = BTreeSet::new();
    let mut postorder = Vec::with_capacity(releases.len());
    let mut stack = vec![(root, false)];
    while let Some((id, expanded)) = stack.pop() {
        if expanded {
            postorder.push(id);
            continue;
        }
        if !visited.insert(id) {
            continue;
        }
        stack.push((id, true));
        let release = releases.get(&id).ok_or_else(|| {
            LkError::new(ErrorCode::ArtifactCorrupt, "release graph node is missing")
        })?;
        for dependency in release.dependencies.iter().rev() {
            stack.push((dependency.release, false));
        }
    }

    let mut depths = BTreeMap::from([(root, 1_usize)]);
    let mut maximum = 1_usize;
    for id in postorder.into_iter().rev() {
        let depth = depths.get(&id).copied().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "release graph topological depth is incomplete",
            )
        })?;
        let release = releases.get(&id).ok_or_else(|| {
            LkError::new(ErrorCode::ArtifactCorrupt, "release graph node is missing")
        })?;
        for dependency in &release.dependencies {
            let dependency_depth = depth.checked_add(1).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "exact release graph depth overflows",
                )
            })?;
            if dependency_depth > MAXIMUM_RELEASE_GRAPH_DEPTH {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "exact release graph depth exceeds policy",
                ));
            }
            maximum = maximum.max(dependency_depth);
            depths
                .entry(dependency.release)
                .and_modify(|prior| *prior = (*prior).max(dependency_depth))
                .or_insert(dependency_depth);
        }
    }
    Ok(maximum)
}

fn validate_imports(
    releases: &BTreeMap<ReleaseId, DecodedRelease>,
) -> Result<BTreeMap<GlobalItem, GlobalItem>> {
    let mut redirects = BTreeMap::new();
    for release in releases.values() {
        let slots = release
            .dependencies
            .iter()
            .map(|dependency| (dependency.slot.as_str(), dependency.release))
            .collect::<BTreeMap<_, _>>();
        for import in &release.imports {
            let target_release = slots
                .get(import.dependency_slot.as_str())
                .copied()
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "release import names an undeclared exact dependency slot",
                    )
                })?;
            let dependency = releases.get(&target_release).ok_or_else(|| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "release import exact dependency is missing",
                )
            })?;
            let target_export = dependency
                .exports
                .iter()
                .find(|export| export.target == import.target)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "cross-release reference targets a private or absent dependency item",
                    )
                })?;
            let local_node = release.snapshot.node(import.local.to_local_node()?)?;
            if ReleaseExportKind::for_node(local_node) != Some(target_export.kind) {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "cross-release proxy and export kinds differ",
                ));
            }
            let local = GlobalItem {
                release: release.id,
                item: import.local,
            };
            let target = GlobalItem {
                release: target_release,
                item: import.target,
            };
            if redirects.insert(local, target).is_some() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "cross-release proxy is bound more than once",
                ));
            }
            validate_proxy(releases, release, import.local, dependency, import.target)?;
            add_member_redirects(
                release,
                import.local,
                dependency,
                import.target,
                &mut redirects,
            )?;
        }
    }
    Ok(redirects)
}

fn validate_proxy(
    releases: &BTreeMap<ReleaseId, DecodedRelease>,
    local_release: &DecodedRelease,
    local: ReleaseItemId,
    target_release: &DecodedRelease,
    target: ReleaseItemId,
) -> Result<()> {
    let left = local_release.snapshot.node(local.to_local_node()?)?;
    let right = target_release.snapshot.node(target.to_local_node()?)?;
    match (left, right) {
        (
            Node::Function {
                parameters: left_parameters,
                result: left_result,
                body: None,
                ..
            },
            Node::Function {
                parameters: right_parameters,
                result: right_result,
                ..
            },
        ) => {
            if left_parameters.len() != right_parameters.len() {
                return Err(proxy_mismatch("function parameter count"));
            }
            for (left, right) in left_parameters.iter().zip(right_parameters) {
                let Node::Parameter { ty: left, .. } = local_release.snapshot.node(*left)? else {
                    return Err(proxy_mismatch("local function parameter structure"));
                };
                let Node::Parameter { ty: right, .. } = target_release.snapshot.node(*right)?
                else {
                    return Err(proxy_mismatch("target function parameter structure"));
                };
                compare_type(releases, local_release, *left, target_release, *right)?;
            }
            compare_type(
                releases,
                local_release,
                *left_result,
                target_release,
                *right_result,
            )
        }
        (Node::ProductType { fields: left, .. }, Node::ProductType { fields: right, .. }) => {
            if left.len() != right.len() {
                return Err(proxy_mismatch("product field count"));
            }
            for (left, right) in left.iter().zip(right) {
                let Node::ProductField {
                    ordinal: left_ordinal,
                    name: left_name,
                    ty: left_ty,
                    ..
                } = local_release.snapshot.node(*left)?
                else {
                    return Err(proxy_mismatch("local product structure"));
                };
                let Node::ProductField {
                    ordinal: right_ordinal,
                    name: right_name,
                    ty: right_ty,
                    ..
                } = target_release.snapshot.node(*right)?
                else {
                    return Err(proxy_mismatch("target product structure"));
                };
                if left_ordinal != right_ordinal || left_name != right_name {
                    return Err(proxy_mismatch("product field identity"));
                }
                compare_type(releases, local_release, *left_ty, target_release, *right_ty)?;
            }
            Ok(())
        }
        (
            Node::SumType { variants: left, .. },
            Node::SumType {
                variants: right, ..
            },
        ) => {
            if left.len() != right.len() {
                return Err(proxy_mismatch("sum variant count"));
            }
            for (left, right) in left.iter().zip(right) {
                let Node::SumVariant {
                    ordinal: left_ordinal,
                    name: left_name,
                    payload: left_payload,
                    ..
                } = local_release.snapshot.node(*left)?
                else {
                    return Err(proxy_mismatch("local sum structure"));
                };
                let Node::SumVariant {
                    ordinal: right_ordinal,
                    name: right_name,
                    payload: right_payload,
                    ..
                } = target_release.snapshot.node(*right)?
                else {
                    return Err(proxy_mismatch("target sum structure"));
                };
                if left_ordinal != right_ordinal || left_name != right_name {
                    return Err(proxy_mismatch("sum variant identity"));
                }
                match (left_payload, right_payload) {
                    (Some(left), Some(right)) => {
                        compare_type(releases, local_release, *left, target_release, *right)?
                    }
                    (None, None) => {}
                    _ => return Err(proxy_mismatch("sum variant payload")),
                }
            }
            Ok(())
        }
        (Node::SequenceType { element: left, .. }, Node::SequenceType { element: right, .. }) => {
            compare_type(releases, local_release, *left, target_release, *right)
        }
        _ => Err(proxy_mismatch("declaration kind")),
    }
}

fn compare_type(
    releases: &BTreeMap<ReleaseId, DecodedRelease>,
    left_release: &DecodedRelease,
    left: SemanticType,
    right_release: &DecodedRelease,
    right: SemanticType,
) -> Result<()> {
    let left = type_key(releases, left_release, left)?;
    let right = type_key(releases, right_release, right)?;
    if left == right {
        Ok(())
    } else {
        Err(proxy_mismatch("semantic type"))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TypeKey {
    Unit,
    Bool,
    I64,
    Bytes,
    Text,
    Nominal(GlobalItem),
}

fn type_key(
    releases: &BTreeMap<ReleaseId, DecodedRelease>,
    release: &DecodedRelease,
    ty: SemanticType,
) -> Result<TypeKey> {
    Ok(match ty {
        SemanticType::Unit => TypeKey::Unit,
        SemanticType::Bool => TypeKey::Bool,
        SemanticType::I64 => TypeKey::I64,
        SemanticType::Bytes => TypeKey::Bytes,
        SemanticType::Text => TypeKey::Text,
        SemanticType::Nominal(target) => {
            let local = ReleaseItemId::from_local_node(target)?;
            if let Some(import) = release.imports.iter().find(|import| import.local == local) {
                let dependency = release
                    .dependencies
                    .iter()
                    .find(|dependency| dependency.slot == import.dependency_slot)
                    .ok_or_else(|| proxy_mismatch("nominal dependency slot"))?;
                if !releases.contains_key(&dependency.release) {
                    return Err(proxy_mismatch("nominal dependency release"));
                }
                TypeKey::Nominal(GlobalItem {
                    release: dependency.release,
                    item: import.target,
                })
            } else {
                TypeKey::Nominal(GlobalItem {
                    release: release.id,
                    item: local,
                })
            }
        }
    })
}

fn add_member_redirects(
    local_release: &DecodedRelease,
    local: ReleaseItemId,
    target_release: &DecodedRelease,
    target: ReleaseItemId,
    redirects: &mut BTreeMap<GlobalItem, GlobalItem>,
) -> Result<()> {
    let left = local_release.snapshot.node(local.to_local_node()?)?;
    let right = target_release.snapshot.node(target.to_local_node()?)?;
    let (left, right): (&[NodeId], &[NodeId]) = match (left, right) {
        (Node::ProductType { fields: left, .. }, Node::ProductType { fields: right, .. }) => {
            (left, right)
        }
        (
            Node::SumType { variants: left, .. },
            Node::SumType {
                variants: right, ..
            },
        ) => (left, right),
        _ => return Ok(()),
    };
    for (left, right) in left.iter().zip(right) {
        redirects.insert(
            GlobalItem {
                release: local_release.id,
                item: ReleaseItemId::from_local_node(*left)?,
            },
            GlobalItem {
                release: target_release.id,
                item: ReleaseItemId::from_local_node(*right)?,
            },
        );
    }
    Ok(())
}

fn imported_subtrees(
    releases: &BTreeMap<ReleaseId, DecodedRelease>,
) -> Result<BTreeSet<GlobalItem>> {
    let mut result = BTreeSet::new();
    for release in releases.values() {
        for import in &release.imports {
            let root = import.local.to_local_node()?;
            let mut stack = vec![root];
            while let Some(id) = stack.pop() {
                if id.is_durable() {
                    result.insert(GlobalItem {
                        release: release.id,
                        item: ReleaseItemId::from_local_node(id)?,
                    });
                }
                let node = release.snapshot.node(id)?;
                for index in (0..node.owned_child_count()).rev() {
                    if let Some(child) = node.owned_child(index) {
                        stack.push(child);
                    }
                }
            }
        }
    }
    Ok(result)
}

fn resolve_global(
    map: &BTreeMap<GlobalNode, NodeId>,
    redirects: &BTreeMap<GlobalItem, GlobalItem>,
    mut source: GlobalNode,
) -> Result<NodeId> {
    if source.serial & (1_u64 << 63) == 0 {
        let mut seen = BTreeSet::new();
        loop {
            let item = GlobalItem {
                release: source.release,
                item: ReleaseItemId::new(source.serial)?,
            };
            let Some(target) = redirects.get(&item).copied() else {
                break;
            };
            if !seen.insert(item) {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "cross-release item redirects contain a cycle",
                ));
            }
            source = GlobalNode {
                release: target.release,
                serial: target.item.get(),
            };
        }
    }
    map.get(&source).copied().ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!(
                "release graph reference to {}:{} cannot be composed",
                source.release, source.serial
            ),
        )
    })
}

fn run_test(
    flattened: &FlattenedGraph,
    release: ReleaseId,
    test: &CanonicalReleaseTest,
) -> Result<ReleaseTestResult> {
    let target = flattened.item(release, test.target)?;
    let result =
        match interpret::compile_and_run(&flattened.snapshot, target, &test.arguments, test.policy)
        {
            Ok(run) => match &test.expected {
                ReleaseTestExpectation::Value(expected) if run.value == *expected => {
                    ReleaseTestResult {
                        name: test.name.clone(),
                        status: ReleaseTestStatus::Passed,
                        observed_trap: None,
                    }
                }
                ReleaseTestExpectation::Value(_) => ReleaseTestResult {
                    name: test.name.clone(),
                    status: ReleaseTestStatus::UnexpectedValue,
                    observed_trap: None,
                },
                ReleaseTestExpectation::Trap(_) => ReleaseTestResult {
                    name: test.name.clone(),
                    status: ReleaseTestStatus::MissingTrap,
                    observed_trap: None,
                },
            },
            Err(error) => classify_test_error(flattened, release, test, &error)?,
        };
    Ok(result)
}

fn classify_test_error(
    flattened: &FlattenedGraph,
    release: ReleaseId,
    test: &CanonicalReleaseTest,
    error: &LkError,
) -> Result<ReleaseTestResult> {
    let observed = ReleaseTrapCode::from_error(error.code);
    let status = if is_resource_error(error.code) {
        ReleaseTestStatus::ResourceFailure
    } else if error.code == ErrorCode::CompileIncomplete {
        ReleaseTestStatus::Incomplete
    } else if matches!(
        error.code,
        ErrorCode::RunArgumentMismatch
            | ErrorCode::TypeMismatch
            | ErrorCode::WrongKind
            | ErrorCode::WrongWorkspace
            | ErrorCode::NodeNotFound
    ) {
        ReleaseTestStatus::InvalidCase
    } else if let Some(actual) = observed {
        match test.expected {
            ReleaseTestExpectation::Value(_) => ReleaseTestStatus::UnexpectedTrap,
            ReleaseTestExpectation::Trap(expected)
                if expected.code == actual
                    && expected
                        .target
                        .map(|target| flattened.node(release, target))
                        .transpose()?
                        .is_none_or(|target| Some(target) == error.target) =>
            {
                ReleaseTestStatus::Passed
            }
            ReleaseTestExpectation::Trap(_) => ReleaseTestStatus::WrongTrap,
        }
    } else {
        ReleaseTestStatus::EngineFailure
    };
    Ok(ReleaseTestResult {
        name: test.name.clone(),
        status,
        observed_trap: observed,
    })
}

const fn is_resource_error(code: ErrorCode) -> bool {
    matches!(
        code,
        ErrorCode::ExecutionFuelExhausted
            | ErrorCode::ExecutionFrameExhausted
            | ErrorCode::RuntimeByteInputTooLarge
            | ErrorCode::ManagedObjectPolicyExceeded
            | ErrorCode::ManagedVisibleBytePolicyExceeded
            | ErrorCode::RetainedBytePolicyExceeded
            | ErrorCode::ResultBytePolicyExceeded
            | ErrorCode::ByteValueTooLarge
            | ErrorCode::ExecutionMemoryExhausted
    )
}

fn flattened_workspace(root: ReleaseId, releases: impl Iterator<Item = ReleaseId>) -> WorkspaceId {
    let mut hasher = blake3::Hasher::new_derive_key(FLATTENED_GRAPH_DOMAIN);
    hasher.update(&root.as_bytes());
    for release in releases {
        hasher.update(&release.as_bytes());
    }
    let digest = hasher.finalize();
    let mut bytes = [0_u8; WorkspaceId::BYTE_LEN];
    bytes.copy_from_slice(&digest.as_bytes()[..WorkspaceId::BYTE_LEN]);
    if bytes == [0; WorkspaceId::BYTE_LEN] {
        bytes[WorkspaceId::BYTE_LEN - 1] = 1;
    }
    WorkspaceId::from_bytes(bytes)
}

fn proxy_mismatch(detail: &str) -> LkError {
    LkError::new(
        ErrorCode::ArtifactCorrupt,
        format!("cross-release proxy signature mismatches dependency export: {detail}"),
    )
}

fn identity_error(error: crate::ids::IdentityError) -> LkError {
    LkError::new(
        ErrorCode::PolicyExceeded,
        format!("release graph identity allocation failed: {error}"),
    )
}
