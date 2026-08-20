use crate::artifact;
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{NodeId, Revision, SnapshotHash, WorkspaceId};
use crate::schema::{Node, NodeKind};
use crate::validate;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Snapshot {
    pub(crate) workspace: WorkspaceId,
    pub(crate) revision: Revision,
    pub(crate) root: NodeId,
    pub(crate) next_serial: u64,
    pub(crate) tombstones: BTreeSet<u64>,
    pub(crate) nodes: BTreeMap<NodeId, Node>,
    pub(crate) hash: SnapshotHash,
}

impl Snapshot {
    pub(crate) fn initial(workspace: WorkspaceId) -> Result<Self> {
        let root = NodeId::new(workspace, 1)
            .map_err(|error| LkError::new(ErrorCode::InvalidContainment, error.to_string()))?;
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root,
            Node::WorkspaceRoot {
                packages: Vec::new(),
                targets: Vec::new(),
            },
        );
        Self::from_parts(
            workspace,
            Revision::INITIAL,
            root,
            2,
            BTreeSet::new(),
            nodes,
        )
    }

    pub(crate) fn from_parts(
        workspace: WorkspaceId,
        revision: Revision,
        root: NodeId,
        next_serial: u64,
        tombstones: BTreeSet<u64>,
        nodes: BTreeMap<NodeId, Node>,
    ) -> Result<Self> {
        let mut snapshot = Self {
            workspace,
            revision,
            root,
            next_serial,
            tombstones,
            nodes,
            hash: SnapshotHash::from_bytes([0; 32]),
        };
        validate::validate_snapshot(&snapshot)?;
        snapshot.hash = artifact::compute_snapshot_hash(&snapshot)?;
        Ok(snapshot)
    }

    pub const fn workspace(&self) -> WorkspaceId {
        self.workspace
    }

    pub const fn revision(&self) -> Revision {
        self.revision
    }

    pub const fn root(&self) -> NodeId {
        self.root
    }

    pub const fn hash(&self) -> SnapshotHash {
        self.hash
    }

    pub const fn next_serial(&self) -> u64 {
        self.next_serial
    }

    pub fn node(&self, id: NodeId) -> Result<&Node> {
        if id.workspace() != self.workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "node identity belongs to another workspace",
            )
            .for_workspace(self.workspace)
            .for_node(id));
        }
        self.nodes.get(&id).ok_or_else(|| {
            LkError::new(ErrorCode::NodeNotFound, "node is absent from this snapshot")
                .for_workspace(self.workspace)
                .at_revision(self.revision)
                .for_node(id)
        })
    }

    pub fn nodes(&self) -> impl ExactSizeIterator<Item = (NodeId, &Node)> {
        self.nodes.iter().map(|(id, node)| (*id, node))
    }

    pub fn tombstones(&self) -> impl ExactSizeIterator<Item = u64> + '_ {
        self.tombstones.iter().copied()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn durable_identity_count(&self) -> usize {
        self.nodes.keys().filter(|id| id.is_durable()).count()
    }

    pub fn function_local_reference_count(&self) -> usize {
        self.nodes
            .keys()
            .filter(|id| id.is_function_local())
            .count()
    }

    pub fn anchor_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|(id, node)| id.is_durable() && matches!(node, Node::Operation { .. }))
            .count()
    }

    pub fn contains_tombstone(&self, serial: u64) -> bool {
        self.tombstones.contains(&serial)
    }
}

pub(crate) struct Workspace {
    id: WorkspaceId,
    head: Revision,
    snapshots: BTreeMap<Revision, Arc<Snapshot>>,
}

impl Workspace {
    pub(crate) fn new(id: WorkspaceId) -> Result<Self> {
        let initial = Arc::new(Snapshot::initial(id)?);
        let mut snapshots = BTreeMap::new();
        snapshots.insert(Revision::INITIAL, initial);
        Ok(Self {
            id,
            head: Revision::INITIAL,
            snapshots,
        })
    }

    pub(crate) fn from_snapshots(
        id: WorkspaceId,
        head: Revision,
        snapshots: BTreeMap<Revision, Arc<Snapshot>>,
    ) -> Result<Self> {
        if !snapshots.contains_key(&head) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace head does not name a retained snapshot",
            )
            .for_workspace(id)
            .at_revision(head));
        }
        let expected_count = head.get().checked_add(1).ok_or_else(|| {
            history_error(id, head, "workspace history length overflows revisions")
        })?;
        if u64::try_from(snapshots.len()).ok() != Some(expected_count) {
            return Err(history_error(
                id,
                head,
                "workspace history is not contiguous from revision zero",
            ));
        }
        let mut expected_revision = Revision::INITIAL;
        let mut previous: Option<&Snapshot> = None;
        for (revision, snapshot) in &snapshots {
            if *revision != expected_revision
                || snapshot.workspace() != id
                || snapshot.revision() != *revision
            {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "retained snapshot identity disagrees with its workspace path",
                )
                .for_workspace(id)
                .at_revision(*revision));
            }
            if let Some(previous) = previous {
                validate_history_transition(previous, snapshot)?;
            } else if snapshot.node_count() != 1
                || snapshot.next_serial() != 2
                || snapshot.tombstones().next().is_some()
            {
                return Err(history_error(
                    id,
                    *revision,
                    "revision zero is not the canonical empty workspace",
                ));
            }
            previous = Some(snapshot);
            expected_revision = expected_revision.next().unwrap_or(expected_revision);
        }
        Ok(Self {
            id,
            head,
            snapshots,
        })
    }

    /// Opens the current semantic authority without eagerly materializing retained history.
    /// Historical snapshots remain owned by the durable repository and are loaded through its
    /// exact revision paths when selected. `from_snapshots` remains the complete reconstruction
    /// oracle used by deep verification.
    pub(crate) fn from_head_snapshot(
        id: WorkspaceId,
        head: Revision,
        snapshot: Arc<Snapshot>,
    ) -> Result<Self> {
        if snapshot.workspace() != id || snapshot.revision() != head {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace head snapshot identity is inconsistent",
            )
            .for_workspace(id)
            .at_revision(head));
        }
        if head == Revision::INITIAL
            && (snapshot.node_count() != 1
                || snapshot.next_serial() != 2
                || snapshot.tombstones().next().is_some())
        {
            return Err(history_error(
                id,
                head,
                "revision zero is not the canonical empty workspace",
            ));
        }
        Ok(Self {
            id,
            head,
            snapshots: BTreeMap::from([(head, snapshot)]),
        })
    }

    pub(crate) const fn id(&self) -> WorkspaceId {
        self.id
    }

    pub(crate) const fn head_revision(&self) -> Revision {
        self.head
    }

    pub(crate) fn head(&self) -> Result<&Arc<Snapshot>> {
        self.snapshots.get(&self.head).ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace head invariant is broken",
            )
            .for_workspace(self.id)
            .at_revision(self.head)
        })
    }

    pub(crate) fn snapshot(&self, revision: Revision) -> Result<&Arc<Snapshot>> {
        self.snapshots.get(&revision).ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionNotFound,
                "requested revision is not retained",
            )
            .for_workspace(self.id)
            .at_revision(revision)
        })
    }

    pub(crate) fn publish(&mut self, snapshot: Arc<Snapshot>) -> Result<()> {
        let expected = self.head.next().ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionConflict,
                "workspace revision is exhausted",
            )
            .for_workspace(self.id)
            .at_revision(self.head)
        })?;
        if snapshot.workspace() != self.id || snapshot.revision() != expected {
            return Err(LkError::new(
                ErrorCode::RevisionConflict,
                "prepared snapshot is not the next workspace revision",
            )
            .for_workspace(self.id)
            .at_revision(snapshot.revision()));
        }
        self.head = snapshot.revision();
        self.snapshots.insert(snapshot.revision(), snapshot);
        Ok(())
    }
}

pub(crate) fn validate_history_transition(previous: &Snapshot, next: &Snapshot) -> Result<()> {
    if next.revision() != previous.revision().next().unwrap_or(previous.revision()) {
        return Err(history_error(
            next.workspace(),
            next.revision(),
            "retained revisions are not adjacent",
        ));
    }
    if next.root() != previous.root() || next.next_serial() < previous.next_serial() {
        return Err(history_error(
            next.workspace(),
            next.revision(),
            "root identity or allocator state moved backward",
        ));
    }
    if !previous
        .tombstones
        .iter()
        .all(|serial| next.tombstones.contains(serial))
    {
        return Err(history_error(
            next.workspace(),
            next.revision(),
            "published tombstones are not monotonic",
        ));
    }
    for (id, old_node) in &previous.nodes {
        if id.is_function_local() {
            continue;
        }
        match next.nodes.get(id) {
            Some(new_node) => {
                if old_node.kind() != new_node.kind()
                    || old_node.owner() != new_node.owner()
                    || !identity_shape_is_stable(previous, next, *id, old_node, new_node)
                {
                    return Err(history_error(
                        next.workspace(),
                        next.revision(),
                        "surviving node changed its identity-defining kind, owner, or contract",
                    )
                    .for_node(*id));
                }
                if !surviving_child_order_is_stable(old_node, new_node, previous, next) {
                    return Err(history_error(
                        next.workspace(),
                        next.revision(),
                        "surviving owned children changed relative semantic order",
                    )
                    .for_node(*id));
                }
            }
            None if next.tombstones.contains(&id.serial()) => {}
            None => {
                return Err(history_error(
                    next.workspace(),
                    next.revision(),
                    "removed live node was not tombstoned",
                )
                .for_node(*id));
            }
        }
    }
    for (id, node) in &next.nodes {
        if id.is_function_local() {
            continue;
        }
        if !previous.nodes.contains_key(id) && id.serial() < previous.next_serial() {
            return Err(history_error(
                next.workspace(),
                next.revision(),
                "a prior identity was resurrected or reused",
            )
            .for_node(*id));
        }
        if !previous.nodes.contains_key(id)
            && matches!(node, Node::Operation { operation, .. } if !matches!(operation, crate::schema::OperationKind::Hole { .. }))
        {
            return Err(history_error(
                next.workspace(),
                next.revision(),
                "a new durable body anchor must begin as a typed hole",
            )
            .for_node(*id));
        }
    }
    Ok(())
}

fn surviving_child_order_is_stable(
    old: &Node,
    new: &Node,
    previous: &Snapshot,
    next: &Snapshot,
) -> bool {
    let mut new_index = 0;
    for old_index in 0..old.owned_child_count() {
        let Some(old_child) = old.owned_child(old_index) else {
            return false;
        };
        if old_child.is_function_local() {
            continue;
        }
        if !next.nodes.contains_key(&old_child) {
            continue;
        }
        loop {
            let Some(new_child) = new.owned_child(new_index) else {
                return false;
            };
            new_index += 1;
            if new_child.is_function_local() {
                continue;
            }
            if previous.nodes.contains_key(&new_child) {
                if new_child != old_child {
                    return false;
                }
                break;
            }
        }
    }
    while let Some(new_child) = new.owned_child(new_index) {
        if new_child.is_durable() && previous.nodes.contains_key(&new_child) {
            return false;
        }
        new_index += 1;
    }
    true
}

fn identity_shape_is_stable(
    previous: &Snapshot,
    next: &Snapshot,
    id: NodeId,
    old: &Node,
    new: &Node,
) -> bool {
    match (old, new) {
        (Node::Package { entry: Some(_), .. }, Node::Package { entry: None, .. }) => false,
        (
            Node::BuildTarget {
                definition: old, ..
            },
            Node::BuildTarget {
                definition: new, ..
            },
        ) => old.kind() == new.kind(),
        (Node::ProductType { fields: old, .. }, Node::ProductType { fields: new, .. }) => {
            // A product may evolve only by atomically appending newly allocated fields. Existing
            // field identity, order, type, and ownership remain stable, and complete candidate
            // validation requires every construction to supply the expanded contract. Removal,
            // reordering, replacement, and type changes remain incompatible with continuity.
            new.starts_with(old)
        }
        (Node::SumType { variants: old, .. }, Node::SumType { variants: new, .. }) => old == new,
        (Node::SequenceType { element: old, .. }, Node::SequenceType { element: new, .. }) => {
            old == new
        }
        (
            Node::ProductField {
                ordinal: old_ordinal,
                ty: old_type,
                ..
            },
            Node::ProductField {
                ordinal: new_ordinal,
                ty: new_type,
                ..
            },
        ) => old_ordinal == new_ordinal && old_type == new_type,
        (
            Node::SumVariant {
                ordinal: old_ordinal,
                payload: old_payload,
                ..
            },
            Node::SumVariant {
                ordinal: new_ordinal,
                payload: new_payload,
                ..
            },
        ) => old_ordinal == new_ordinal && old_payload == new_payload,
        (Node::Function { result: old, .. }, Node::Function { result: new, .. }) => old == new,
        (
            Node::Parameter {
                ordinal: old_ordinal,
                ty: old_type,
                ..
            },
            Node::Parameter {
                ordinal: new_ordinal,
                ty: new_type,
                ..
            },
        )
        | (
            Node::BlockArgument {
                ordinal: old_ordinal,
                ty: old_type,
                ..
            },
            Node::BlockArgument {
                ordinal: new_ordinal,
                ty: new_type,
                ..
            },
        ) => old_ordinal == new_ordinal && old_type == new_type,
        (Node::Operation { operation: old, .. }, Node::Operation { operation: new, .. }) => {
            operation_identity_shape_is_stable(previous, next, id, old, new)
        }
        _ => true,
    }
}

fn operation_identity_shape_is_stable(
    previous: &Snapshot,
    next: &Snapshot,
    id: NodeId,
    old: &crate::schema::OperationKind,
    new: &crate::schema::OperationKind,
) -> bool {
    let same_results = operation_result_types(previous, id, old).ok()
        == operation_result_types(next, id, new).ok();
    if old.code() == new.code() {
        return same_results
            && old.is_terminator() == new.is_terminator()
            && old.owned_region_count() == new.owned_region_count()
            && (0..old.owned_region_count())
                .all(|index| old.owned_region(index) == new.owned_region(index));
    }
    matches!(old, crate::schema::OperationKind::Hole { .. })
        && new.is_complete()
        && !new.is_terminator()
        && new.owned_region_count() == 0
        && !matches!(new, crate::schema::OperationKind::Hole { .. })
        && match old {
            crate::schema::OperationKind::Hole {
                expected: crate::schema::SemanticType::Nominal(_),
            } => matches!(
                new,
                crate::schema::OperationKind::ConstructProduct { .. }
                    | crate::schema::OperationKind::ConstructVariant { .. }
                    | crate::schema::OperationKind::ProjectField { .. }
                    | crate::schema::OperationKind::SequenceEmpty { .. }
                    | crate::schema::OperationKind::SequenceGet { .. }
                    | crate::schema::OperationKind::SequenceAppend { .. }
                    | crate::schema::OperationKind::SequenceReplace { .. }
                    | crate::schema::OperationKind::SequenceSlice { .. }
                    | crate::schema::OperationKind::SequenceConcat { .. }
            ),
            _ => true,
        }
        && same_results
}

pub(crate) fn operation_result_types(
    snapshot: &Snapshot,
    operation_id: NodeId,
    operation: &crate::schema::OperationKind,
) -> Result<Vec<crate::schema::SemanticType>> {
    (0..operation.result_count())
        .map(|index| {
            operation_result_type(snapshot, operation_id, operation, index).ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "operation result type cannot be resolved",
                )
                .for_node(operation_id)
            })
        })
        .collect()
}

pub(crate) fn operation_result_type(
    snapshot: &Snapshot,
    _operation_id: NodeId,
    operation: &crate::schema::OperationKind,
    index: usize,
) -> Option<crate::schema::SemanticType> {
    if index >= operation.result_count() {
        return None;
    }
    match operation {
        crate::schema::OperationKind::Call { function, .. } => match snapshot.nodes.get(function) {
            Some(Node::Function { result, .. }) => Some(*result),
            _ => None,
        },
        crate::schema::OperationKind::ConstructProduct { product, .. } => {
            matches!(snapshot.nodes.get(product), Some(Node::ProductType { .. }))
                .then_some(crate::schema::SemanticType::Nominal(*product))
        }
        crate::schema::OperationKind::ProjectField { field, .. } => match snapshot.nodes.get(field)
        {
            Some(Node::ProductField { ty, .. }) => Some(*ty),
            _ => None,
        },
        crate::schema::OperationKind::ConstructVariant { variant, .. } => {
            match snapshot.nodes.get(variant) {
                Some(Node::SumVariant { owner, .. }) => {
                    Some(crate::schema::SemanticType::Nominal(*owner))
                }
                _ => None,
            }
        }
        crate::schema::OperationKind::MatchSum { result, .. } => Some(*result),
        crate::schema::OperationKind::SequenceEmpty { sequence }
        | crate::schema::OperationKind::SequenceAppend { sequence, .. }
        | crate::schema::OperationKind::SequenceReplace { sequence, .. }
        | crate::schema::OperationKind::SequenceSlice { sequence, .. }
        | crate::schema::OperationKind::SequenceConcat { sequence, .. } => matches!(
            snapshot.nodes.get(sequence),
            Some(Node::SequenceType { .. })
        )
        .then_some(crate::schema::SemanticType::Nominal(*sequence)),
        crate::schema::OperationKind::SequenceGet { sequence, .. } => {
            match snapshot.nodes.get(sequence) {
                Some(Node::SequenceType { element, .. }) => Some(*element),
                _ => None,
            }
        }
        _ => operation.result_type(index, None),
    }
}

fn history_error(workspace: WorkspaceId, revision: Revision, message: &str) -> LkError {
    LkError::new(ErrorCode::ArtifactCorrupt, message)
        .for_workspace(workspace)
        .at_revision(revision)
}

pub(crate) fn require_kind(
    nodes: &BTreeMap<NodeId, Node>,
    id: NodeId,
    expected: NodeKind,
) -> Result<&Node> {
    let node = nodes.get(&id).ok_or_else(|| {
        LkError::new(ErrorCode::NodeNotFound, "target node does not exist").for_node(id)
    })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_history_allows_only_same_contract_or_one_way_hole_refinement() {
        use crate::schema::{OperationKind, SemanticType, ValueRef};

        let workspace = WorkspaceId::from_bytes([0x30; 16]);
        let snapshot = Snapshot::initial(workspace).expect("snapshot");
        let first = NodeId::new(workspace, 2).expect("first");
        let second = NodeId::new(workspace, 3).expect("second");
        let value = |operation| ValueRef::OperationResult {
            operation,
            output: 0,
        };
        let stable = |old: &OperationKind, new: &OperationKind| {
            operation_identity_shape_is_stable(&snapshot, &snapshot, first, old, new)
        };
        let hole = OperationKind::Hole {
            expected: SemanticType::I64,
        };
        assert!(stable(&hole, &OperationKind::ConstI64(1)));
        assert!(stable(
            &hole,
            &OperationKind::AddI64 {
                lhs: value(first),
                rhs: value(second)
            }
        ));
        assert!(!stable(&hole, &OperationKind::ConstBool(true)));
        assert!(!stable(
            &hole,
            &OperationKind::Hole {
                expected: SemanticType::Bool
            }
        ));
        assert!(!stable(
            &OperationKind::ConstI64(1),
            &OperationKind::AddI64 {
                lhs: value(first),
                rhs: value(second)
            }
        ));
        assert!(!stable(&OperationKind::ConstI64(1), &hole));
        assert!(!stable(
            &hole,
            &OperationKind::Return {
                value: value(first)
            }
        ));
    }

    #[test]
    fn retained_history_rejects_reordering_surviving_owned_children() {
        let workspace = WorkspaceId::from_bytes([0x32; 16]);
        let root = NodeId::new(workspace, 1).expect("root");
        let first = NodeId::new(workspace, 2).expect("first package");
        let second = NodeId::new(workspace, 3).expect("second package");
        let package = |name: &str| Node::Package {
            owner: root,
            name: name.to_owned(),
            modules: Vec::new(),
            entry: None,
        };
        let nodes = BTreeMap::from([
            (
                root,
                Node::WorkspaceRoot {
                    packages: vec![first, second],
                    targets: Vec::new(),
                },
            ),
            (first, package("first")),
            (second, package("second")),
        ]);
        let previous = Snapshot::from_parts(
            workspace,
            Revision::new(1),
            root,
            4,
            BTreeSet::new(),
            nodes.clone(),
        )
        .expect("ordered snapshot");
        let mut reordered = nodes;
        let Node::WorkspaceRoot { packages, .. } =
            reordered.get_mut(&root).expect("workspace root")
        else {
            panic!("root kind");
        };
        packages.swap(0, 1);
        let next = Snapshot::from_parts(
            workspace,
            Revision::new(2),
            root,
            4,
            BTreeSet::new(),
            reordered,
        )
        .expect("individually valid reordered snapshot");
        assert_eq!(
            validate_history_transition(&previous, &next)
                .expect_err("history must reject surviving child reorder")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn retained_history_rejects_clearing_a_surviving_package_entry() {
        use crate::schema::SemanticType;

        let workspace = WorkspaceId::from_bytes([0x33; 16]);
        let root = NodeId::new(workspace, 1).expect("root");
        let package = NodeId::new(workspace, 2).expect("package");
        let module = NodeId::new(workspace, 3).expect("module");
        let function = NodeId::new(workspace, 4).expect("function");
        let nodes = BTreeMap::from([
            (
                root,
                Node::WorkspaceRoot {
                    packages: vec![package],
                    targets: Vec::new(),
                },
            ),
            (
                package,
                Node::Package {
                    owner: root,
                    name: "package".to_owned(),
                    modules: vec![module],
                    entry: Some(function),
                },
            ),
            (
                module,
                Node::Module {
                    owner: package,
                    name: "module".to_owned(),
                    types: Vec::new(),
                    functions: vec![function],
                },
            ),
            (
                function,
                Node::Function {
                    owner: module,
                    name: "function".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64,
                    body: None,
                },
            ),
        ]);
        let previous = Arc::new(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                root,
                5,
                BTreeSet::new(),
                nodes.clone(),
            )
            .expect("selected entry snapshot"),
        );
        let mut cleared = nodes;
        let Node::Package { entry, .. } = cleared.get_mut(&package).expect("package node") else {
            panic!("package kind")
        };
        *entry = None;
        let next = Arc::new(
            Snapshot::from_parts(
                workspace,
                Revision::new(2),
                root,
                5,
                BTreeSet::new(),
                cleared,
            )
            .expect("individually valid cleared entry snapshot"),
        );
        let snapshots = BTreeMap::from([
            (
                Revision::INITIAL,
                Arc::new(Snapshot::initial(workspace).expect("initial snapshot")),
            ),
            (Revision::new(1), previous),
            (Revision::new(2), next),
        ]);
        assert_eq!(
            Workspace::from_snapshots(workspace, Revision::new(2), snapshots)
                .err()
                .expect("history must reject clearing a surviving entry")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn retained_history_rejects_identity_resurrection() {
        let workspace = WorkspaceId::from_bytes([0x31; 16]);
        let root = NodeId::new(workspace, 1).expect("root");
        let package = NodeId::new(workspace, 2).expect("package");
        let revision_zero = Arc::new(Snapshot::initial(workspace).expect("initial"));

        let mut live_nodes = BTreeMap::new();
        live_nodes.insert(
            root,
            Node::WorkspaceRoot {
                packages: vec![package],
                targets: Vec::new(),
            },
        );
        live_nodes.insert(
            package,
            Node::Package {
                owner: root,
                name: "package".to_owned(),
                modules: Vec::new(),
                entry: None,
            },
        );
        let revision_one = Arc::new(
            Snapshot::from_parts(
                workspace,
                Revision::new(1),
                root,
                3,
                BTreeSet::new(),
                live_nodes.clone(),
            )
            .expect("live package snapshot"),
        );

        let mut deleted_nodes = BTreeMap::new();
        deleted_nodes.insert(
            root,
            Node::WorkspaceRoot {
                packages: Vec::new(),
                targets: Vec::new(),
            },
        );
        let revision_two = Arc::new(
            Snapshot::from_parts(
                workspace,
                Revision::new(2),
                root,
                3,
                BTreeSet::from([2]),
                deleted_nodes,
            )
            .expect("deleted package snapshot"),
        );
        let revision_three = Arc::new(
            Snapshot::from_parts(
                workspace,
                Revision::new(3),
                root,
                3,
                BTreeSet::new(),
                live_nodes,
            )
            .expect("individually valid resurrected snapshot"),
        );
        let snapshots = BTreeMap::from([
            (Revision::INITIAL, revision_zero),
            (Revision::new(1), revision_one),
            (Revision::new(2), revision_two),
            (Revision::new(3), revision_three),
        ]);
        assert_eq!(
            Workspace::from_snapshots(workspace, Revision::new(3), snapshots)
                .err()
                .expect("history must reject resurrection")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }
}
