use crate::graph::Snapshot;
use crate::ids::NodeId;
use crate::query;
use crate::schema::{Node, NodeKind, OperationKind};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiff {
    pub changes: Vec<Change>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub node: NodeId,
    pub kind: ChangeKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeKind {
    Created { kind: NodeKind },
    Deleted { kind: NodeKind },
    Renamed { before: String, after: String },
    ScalarAttributeChanged,
    ContainmentChanged,
    OperandChanged,
    DirectReferenceChanged,
    EntryFunctionChanged,
    CompletenessChanged { complete: bool },
}

impl ChangeKind {
    pub const fn stable_tag(&self) -> u8 {
        match self {
            Self::Created { .. } => 1,
            Self::Deleted { .. } => 2,
            Self::Renamed { .. } => 3,
            Self::ScalarAttributeChanged => 4,
            Self::ContainmentChanged => 5,
            Self::OperandChanged => 6,
            Self::DirectReferenceChanged => 7,
            Self::EntryFunctionChanged => 8,
            Self::CompletenessChanged { .. } => 9,
        }
    }
}

pub(crate) fn between(before: &Snapshot, after: &Snapshot) -> SemanticDiff {
    let ids: BTreeSet<NodeId> = before
        .nodes()
        .map(|(id, _)| id)
        .chain(after.nodes().map(|(id, _)| id))
        .collect();
    let mut changes = Vec::new();
    for id in ids {
        match (before.nodes.get(&id), after.nodes.get(&id)) {
            (None, Some(node)) => changes.push(Change {
                node: id,
                kind: ChangeKind::Created { kind: node.kind() },
            }),
            (Some(node), None) => changes.push(Change {
                node: id,
                kind: ChangeKind::Deleted { kind: node.kind() },
            }),
            (Some(old), Some(new)) if old != new => classify_change(id, old, new, &mut changes),
            _ => {}
        }
    }
    let old_complete = query::workspace_blockers(before).is_empty();
    let new_complete = query::workspace_blockers(after).is_empty();
    if old_complete != new_complete {
        changes.push(Change {
            node: after.root(),
            kind: ChangeKind::CompletenessChanged {
                complete: new_complete,
            },
        });
    }
    changes.sort_by(|left, right| {
        (left.node, left.kind.stable_tag()).cmp(&(right.node, right.kind.stable_tag()))
    });
    SemanticDiff { changes }
}

fn classify_change(id: NodeId, old: &Node, new: &Node, changes: &mut Vec<Change>) {
    if old.name() != new.name()
        && let (Some(before), Some(after)) = (old.name(), new.name())
    {
        changes.push(Change {
            node: id,
            kind: ChangeKind::Renamed {
                before: before.to_owned(),
                after: after.to_owned(),
            },
        });
    }
    if old.owned_children() != new.owned_children() {
        changes.push(Change {
            node: id,
            kind: ChangeKind::ContainmentChanged,
        });
    }
    if old.direct_references() != new.direct_references() {
        changes.push(Change {
            node: id,
            kind: ChangeKind::DirectReferenceChanged,
        });
    }
    if let (
        Node::Package {
            entry: old_entry, ..
        },
        Node::Package {
            entry: new_entry, ..
        },
    ) = (old, new)
        && old_entry != new_entry
    {
        changes.push(Change {
            node: id,
            kind: ChangeKind::EntryFunctionChanged,
        });
    }
    if let (
        Node::Operation {
            operation: old_operation,
            ..
        },
        Node::Operation {
            operation: new_operation,
            ..
        },
    ) = (old, new)
    {
        if old_operation.operands() != new_operation.operands() {
            changes.push(Change {
                node: id,
                kind: ChangeKind::OperandChanged,
            });
        }
        if scalar_operation_changed(old_operation, new_operation) {
            changes.push(Change {
                node: id,
                kind: ChangeKind::ScalarAttributeChanged,
            });
        }
    } else if old != new
        && old.name() == new.name()
        && old.owned_children() == new.owned_children()
        && old.direct_references() == new.direct_references()
    {
        changes.push(Change {
            node: id,
            kind: ChangeKind::ScalarAttributeChanged,
        });
    }
}

fn scalar_operation_changed(old: &OperationKind, new: &OperationKind) -> bool {
    match (old, new) {
        (OperationKind::ConstI64(left), OperationKind::ConstI64(right)) => left != right,
        (OperationKind::ConstBool(left), OperationKind::ConstBool(right)) => left != right,
        (OperationKind::Hole { expected: left }, OperationKind::Hole { expected: right }) => {
            left != right
        }
        _ => old.stable_tag() != new.stable_tag(),
    }
}
