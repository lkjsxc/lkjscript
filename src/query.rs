use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, SnapshotHash, WorkspaceId};
use crate::schema::{Node, NodeKind, OperationKind, SemanticType};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedCategory {
    EntryFunction,
    FunctionBody,
    Expression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompletenessBlocker {
    pub owner: NodeId,
    pub target: Option<NodeId>,
    pub category: ExpectedCategory,
    pub expected_type: Option<SemanticType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceSummary {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub hash: SnapshotHash,
    pub root: NodeId,
    pub node_count: u64,
    pub complete: bool,
    pub blocker_count: u64,
    pub entries: Vec<NodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionSignature {
    pub parameters: Vec<(NodeId, SemanticType)>,
    pub result: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeSummary {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub node: NodeId,
    pub kind: NodeKind,
    pub owner: Option<NodeId>,
    pub display_name: Option<String>,
    pub signature: Option<FunctionSignature>,
    pub value_type: Option<SemanticType>,
    pub complete: bool,
    pub diagnostic_count: u64,
    pub child_count: u64,
    pub reference_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeView {
    pub summary: NodeSummary,
    pub record: Option<Node>,
}

pub fn workspace_summary(snapshot: &Snapshot) -> WorkspaceSummary {
    let blockers = workspace_blockers(snapshot);
    let mut entries: Vec<NodeId> = snapshot
        .nodes()
        .filter_map(|(_, node)| match node {
            Node::Package { entry, .. } => *entry,
            _ => None,
        })
        .collect();
    entries.sort();
    WorkspaceSummary {
        workspace: snapshot.workspace(),
        revision: snapshot.revision(),
        hash: snapshot.hash(),
        root: snapshot.root(),
        node_count: u64::try_from(snapshot.node_count()).unwrap_or(u64::MAX),
        complete: blockers.is_empty(),
        blocker_count: u64::try_from(blockers.len()).unwrap_or(u64::MAX),
        entries,
    }
}

pub fn node_view(snapshot: &Snapshot, id: NodeId, expand: bool) -> Result<NodeView> {
    let node = snapshot.node(id)?;
    let blockers = blockers_for_node(snapshot, id);
    let signature = match node {
        Node::Function {
            parameters, result, ..
        } => {
            let mut values = Vec::with_capacity(parameters.len());
            for parameter in parameters {
                let parameter_node = snapshot.node(*parameter)?;
                let Node::Parameter { ty, .. } = parameter_node else {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "function signature contains a non-parameter node",
                    )
                    .for_node(*parameter));
                };
                values.push((*parameter, *ty));
            }
            Some(FunctionSignature {
                parameters: values,
                result: *result,
            })
        }
        _ => None,
    };
    let value_type = match node {
        Node::Parameter { ty, .. } => Some(*ty),
        Node::Operation { operation, .. } => operation.contract().result_types.first().copied(),
        _ => None,
    };
    let child_count = u64::try_from(node.owned_children().len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "node child count does not fit query representation",
        )
        .for_node(id)
    })?;
    let reference_count = u64::try_from(node.direct_references().len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "node reference count does not fit query representation",
        )
        .for_node(id)
    })?;
    let diagnostic_count = u64::try_from(blockers.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "node diagnostic count does not fit query representation",
        )
        .for_node(id)
    })?;
    Ok(NodeView {
        summary: NodeSummary {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            node: id,
            kind: node.kind(),
            owner: node.owner(),
            display_name: node.name().map(str::to_owned),
            signature,
            value_type,
            complete: blockers.is_empty(),
            diagnostic_count,
            child_count,
            reference_count,
        },
        record: expand.then(|| node.clone()),
    })
}

pub fn workspace_blockers(snapshot: &Snapshot) -> Vec<CompletenessBlocker> {
    let mut blockers = Vec::new();
    if !snapshot
        .nodes()
        .any(|(_, node)| matches!(node, Node::Package { .. }))
    {
        blockers.push(CompletenessBlocker {
            owner: snapshot.root(),
            target: None,
            category: ExpectedCategory::EntryFunction,
            expected_type: None,
        });
    }
    for (id, node) in snapshot.nodes() {
        match node {
            Node::Package { entry: None, .. } => blockers.push(CompletenessBlocker {
                owner: id,
                target: None,
                category: ExpectedCategory::EntryFunction,
                expected_type: None,
            }),
            Node::Function { body: None, .. } => blockers.push(CompletenessBlocker {
                owner: id,
                target: None,
                category: ExpectedCategory::FunctionBody,
                expected_type: None,
            }),
            Node::Operation {
                operation: OperationKind::Hole { expected },
                ..
            } => blockers.push(CompletenessBlocker {
                owner: id,
                target: Some(id),
                category: ExpectedCategory::Expression,
                expected_type: Some(*expected),
            }),
            _ => {}
        }
    }
    blockers.sort_by_key(|blocker| (blocker.owner, blocker.target));
    blockers
}

pub fn entry_blockers(snapshot: &Snapshot, entry: NodeId) -> Result<Vec<CompletenessBlocker>> {
    let function = snapshot.node(entry)?;
    let Node::Function { body, .. } = function else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "compile entry must be a function")
                .for_node(entry)
                .with_kinds(NodeKind::Function, function.kind()),
        );
    };
    let Some(body) = body else {
        return Ok(vec![CompletenessBlocker {
            owner: entry,
            target: None,
            category: ExpectedCategory::FunctionBody,
            expected_type: None,
        }]);
    };
    let mut blockers = Vec::new();
    let mut stack = vec![*body];
    while let Some(id) = stack.pop() {
        let node = snapshot.node(id)?;
        if let Node::Operation {
            operation: OperationKind::Hole { expected },
            ..
        } = node
        {
            blockers.push(CompletenessBlocker {
                owner: id,
                target: Some(id),
                category: ExpectedCategory::Expression,
                expected_type: Some(*expected),
            });
        }
        let mut children = node.owned_children();
        children.reverse();
        stack.extend(children);
    }
    blockers.sort_by_key(|blocker| (blocker.owner, blocker.target));
    Ok(blockers)
}

fn blockers_for_node(snapshot: &Snapshot, id: NodeId) -> Vec<CompletenessBlocker> {
    let mut descendants = std::collections::BTreeSet::new();
    let mut stack = vec![id];
    while let Some(current) = stack.pop() {
        if !descendants.insert(current) {
            continue;
        }
        if let Ok(node) = snapshot.node(current) {
            let mut children = node.owned_children();
            children.reverse();
            stack.extend(children);
        }
    }
    workspace_blockers(snapshot)
        .into_iter()
        .filter(|blocker| {
            descendants.contains(&blocker.owner)
                || blocker
                    .target
                    .is_some_and(|target| descendants.contains(&target))
        })
        .collect()
}
