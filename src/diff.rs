use crate::graph::Snapshot;
use crate::ids::{ChangeDigest, NodeId};
use crate::query;
use crate::schema::{
    ByteString, Node, NodeKind, OperationCode, OperationKind, SemanticType, ValueRef,
};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

const DIGEST_DOMAIN: &[u8] = b"lkjscript.semantic-diff.v3\0";

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticDiff {
    pub changes: Vec<Change>,
    pub digest: ChangeDigest,
}

impl SemanticDiff {
    pub fn change_count(&self) -> u64 {
        u64::try_from(self.changes.len()).unwrap_or(u64::MAX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    pub node: NodeId,
    pub kind: ChangeKind,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ChangeKind {
    Created {
        kind: NodeKind,
    },
    Deleted {
        kind: NodeKind,
    },
    Renamed {
        before: String,
        after: String,
    },
    ScalarAttributeChanged {
        before: ScalarValue,
        after: ScalarValue,
    },
    ContainmentChanged {
        before_count: u64,
        after_count: u64,
    },
    OperandChanged {
        index: u64,
        before: Option<ValueRef>,
        after: Option<ValueRef>,
    },
    DefinitionChanged {
        before: NodeId,
        after: NodeId,
    },
    EntryFunctionChanged {
        before: Option<NodeId>,
        after: Option<NodeId>,
    },
    CompletenessChanged {
        complete: bool,
    },
    OperationRefined {
        before: OperationCode,
        after: OperationCode,
        result_type: SemanticType,
        replacement: OperationKind,
    },
    AllocatedAndTombstoned,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ScalarValue {
    I64(i64),
    Bool(bool),
    Type(SemanticType),
    Bytes(ByteString),
}

impl ChangeKind {
    pub const fn stable_tag(&self) -> u8 {
        match self {
            Self::Created { .. } => 1,
            Self::Deleted { .. } => 2,
            Self::Renamed { .. } => 3,
            Self::ScalarAttributeChanged { .. } => 4,
            Self::ContainmentChanged { .. } => 5,
            Self::OperandChanged { .. } => 6,
            Self::DefinitionChanged { .. } => 7,
            Self::EntryFunctionChanged { .. } => 8,
            Self::CompletenessChanged { .. } => 9,
            Self::OperationRefined { .. } => 10,
            Self::AllocatedAndTombstoned => 11,
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
    for serial in after.tombstones.difference(&before.tombstones) {
        if *serial >= before.next_serial
            && let Ok(node) = NodeId::new(after.workspace(), *serial)
        {
            changes.push(Change {
                node,
                kind: ChangeKind::AllocatedAndTombstoned,
            });
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
    changes.sort_by(compare_changes);
    let digest = digest_changes(before, after, &changes);
    SemanticDiff { changes, digest }
}

fn compare_changes(left: &Change, right: &Change) -> Ordering {
    left.node
        .cmp(&right.node)
        .then_with(|| left.kind.stable_tag().cmp(&right.kind.stable_tag()))
        .then_with(|| left.kind.cmp(&right.kind))
}

fn digest_changes(before: &Snapshot, after: &Snapshot, changes: &[Change]) -> ChangeDigest {
    let mut hasher = blake3::Hasher::new();
    hasher.update(DIGEST_DOMAIN);
    hasher.update(&before.workspace().as_bytes());
    hasher.update(&before.revision().get().to_le_bytes());
    hasher.update(&after.revision().get().to_le_bytes());
    hasher.update(&before.hash().as_bytes());
    hasher.update(&after.hash().as_bytes());
    hasher.update(
        &u64::try_from(changes.len())
            .unwrap_or(u64::MAX)
            .to_le_bytes(),
    );
    for change in changes {
        hasher.update(&change.node.workspace().as_bytes());
        hasher.update(&change.node.serial().to_le_bytes());
        hasher.update(&[change.kind.stable_tag()]);
        hash_change_kind(&mut hasher, &change.kind);
    }
    ChangeDigest::from_bytes(*hasher.finalize().as_bytes())
}

fn hash_change_kind(hasher: &mut blake3::Hasher, kind: &ChangeKind) {
    match kind {
        ChangeKind::Created { kind } | ChangeKind::Deleted { kind } => {
            hasher.update(&[kind.stable_tag()]);
        }
        ChangeKind::Renamed { before, after } => {
            hash_bytes(hasher, before.as_bytes());
            hash_bytes(hasher, after.as_bytes());
        }
        ChangeKind::ScalarAttributeChanged { before, after } => {
            hash_scalar_value(hasher, before);
            hash_scalar_value(hasher, after);
        }
        ChangeKind::ContainmentChanged {
            before_count,
            after_count,
        } => {
            hasher.update(&before_count.to_le_bytes());
            hasher.update(&after_count.to_le_bytes());
        }
        ChangeKind::OperandChanged {
            index,
            before,
            after,
        } => {
            hasher.update(&index.to_le_bytes());
            hash_optional_value(hasher, *before);
            hash_optional_value(hasher, *after);
        }
        ChangeKind::DefinitionChanged { before, after } => {
            hash_node(hasher, *before);
            hash_node(hasher, *after);
        }
        ChangeKind::EntryFunctionChanged { before, after } => {
            hash_optional_node(hasher, *before);
            hash_optional_node(hasher, *after);
        }
        ChangeKind::CompletenessChanged { complete } => {
            hasher.update(&[u8::from(*complete)]);
        }
        ChangeKind::OperationRefined {
            before,
            after,
            result_type,
            replacement,
        } => {
            hasher.update(&[
                before.stable_tag(),
                after.stable_tag(),
                result_type.stable_tag(),
            ]);
            hash_operation(hasher, replacement);
        }
        ChangeKind::AllocatedAndTombstoned => {}
    }
}

fn hash_scalar_value(hasher: &mut blake3::Hasher, value: &ScalarValue) {
    match value {
        ScalarValue::I64(value) => {
            hasher.update(&[1]);
            hasher.update(&value.to_le_bytes());
        }
        ScalarValue::Bool(value) => {
            hasher.update(&[2, u8::from(*value)]);
        }
        ScalarValue::Type(value) => {
            hasher.update(&[3]);
            hash_type(hasher, *value);
        }
        ScalarValue::Bytes(value) => {
            hasher.update(&[4]);
            hasher.update(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
            hasher.update(value.as_slice());
        }
    }
}

fn hash_operation(hasher: &mut blake3::Hasher, operation: &OperationKind) {
    hasher.update(&[operation.code().stable_tag()]);
    match operation {
        OperationKind::ConstUnit => {}
        OperationKind::ConstI64(value) => {
            hasher.update(&value.to_le_bytes());
        }
        OperationKind::ConstBool(value) => {
            hasher.update(&[u8::from(*value)]);
        }
        OperationKind::ConstBytes(value) => {
            hasher.update(&u64::try_from(value.len()).unwrap_or(u64::MAX).to_le_bytes());
            hasher.update(value.as_slice());
        }
        OperationKind::AddI64 { lhs, rhs }
        | OperationKind::LtI64 { lhs, rhs }
        | OperationKind::BytesEqual { lhs, rhs } => {
            hash_value(hasher, *lhs);
            hash_value(hasher, *rhs);
        }
        OperationKind::BytesLen { value } => hash_value(hasher, *value),
        OperationKind::BytesAt { value, index } => {
            hash_value(hasher, *value);
            hash_value(hasher, *index);
        }
        OperationKind::BytesSlice {
            value,
            start,
            length,
        } => {
            hash_value(hasher, *value);
            hash_value(hasher, *start);
            hash_value(hasher, *length);
        }
        OperationKind::Call {
            function,
            arguments,
        } => {
            hash_node(hasher, *function);
            hasher.update(
                &u64::try_from(arguments.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            for argument in arguments {
                hash_value(hasher, *argument);
            }
        }
        OperationKind::Hole { expected } => {
            hash_type(hasher, *expected);
        }
        OperationKind::If {
            condition,
            result,
            then_region,
            else_region,
        } => {
            hash_value(hasher, *condition);
            hash_type(hasher, *result);
            hash_node(hasher, *then_region);
            hash_node(hasher, *else_region);
        }
        OperationKind::ForI64 {
            start,
            end_exclusive,
            step,
            initial,
            carried,
            body_region,
        } => {
            hash_value(hasher, *start);
            hash_value(hasher, *end_exclusive);
            hasher.update(&step.to_le_bytes());
            hash_value(hasher, *initial);
            hash_type(hasher, *carried);
            hash_node(hasher, *body_region);
        }
        OperationKind::Return { value } | OperationKind::Yield { value } => {
            hash_value(hasher, *value)
        }
        OperationKind::ConstructProduct { product, fields } => {
            hash_node(hasher, *product);
            hasher.update(
                &u64::try_from(fields.len())
                    .unwrap_or(u64::MAX)
                    .to_le_bytes(),
            );
            for field in fields {
                hash_node(hasher, field.field);
                hash_value(hasher, field.value);
            }
        }
        OperationKind::ProjectField { value, field } => {
            hash_value(hasher, *value);
            hash_node(hasher, *field);
        }
        OperationKind::ConstructVariant { variant, payload } => {
            hash_node(hasher, *variant);
            hash_optional_value(hasher, *payload);
        }
        OperationKind::MatchSum {
            scrutinee,
            result,
            arms,
        } => {
            hash_value(hasher, *scrutinee);
            hash_type(hasher, *result);
            hasher.update(&u64::try_from(arms.len()).unwrap_or(u64::MAX).to_le_bytes());
            for arm in arms {
                hash_node(hasher, arm.variant);
                hash_node(hasher, arm.region);
            }
        }
    }
}

fn hash_type(hasher: &mut blake3::Hasher, ty: crate::schema::SemanticType) {
    hasher.update(&[ty.stable_tag()]);
    if let crate::schema::SemanticType::Nominal(target) = ty {
        hash_node(hasher, target);
    }
}

fn hash_optional_value(hasher: &mut blake3::Hasher, value: Option<ValueRef>) {
    hasher.update(&[u8::from(value.is_some())]);
    if let Some(value) = value {
        hash_value(hasher, value);
    }
}

fn hash_value(hasher: &mut blake3::Hasher, value: ValueRef) {
    match value {
        ValueRef::FunctionParameter(parameter) => {
            hasher.update(&[1]);
            hash_node(hasher, parameter);
        }
        ValueRef::OperationResult { operation, output } => {
            hasher.update(&[2]);
            hash_node(hasher, operation);
            hasher.update(&[output]);
        }
        ValueRef::BlockArgument(argument) => {
            hasher.update(&[3]);
            hash_node(hasher, argument);
        }
    }
}

fn hash_optional_node(hasher: &mut blake3::Hasher, node: Option<NodeId>) {
    hasher.update(&[u8::from(node.is_some())]);
    if let Some(node) = node {
        hash_node(hasher, node);
    }
}

fn hash_node(hasher: &mut blake3::Hasher, node: NodeId) {
    hasher.update(&node.workspace().as_bytes());
    hasher.update(&node.serial().to_le_bytes());
}

fn hash_bytes(hasher: &mut blake3::Hasher, bytes: &[u8]) {
    hasher.update(&u64::try_from(bytes.len()).unwrap_or(u64::MAX).to_le_bytes());
    hasher.update(bytes);
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
    if !owned_children_equal(old, new) {
        changes.push(Change {
            node: id,
            kind: ChangeKind::ContainmentChanged {
                before_count: u64::try_from(old.owned_child_count()).unwrap_or(u64::MAX),
                after_count: u64::try_from(new.owned_child_count()).unwrap_or(u64::MAX),
            },
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
            kind: ChangeKind::EntryFunctionChanged {
                before: *old_entry,
                after: *new_entry,
            },
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
        let operand_count = old_operation
            .operand_count()
            .max(new_operation.operand_count());
        for index in 0..operand_count {
            let before = old_operation.operand(index);
            let after = new_operation.operand(index);
            if before != after {
                let Ok(index) = u64::try_from(index) else {
                    continue;
                };
                changes.push(Change {
                    node: id,
                    kind: ChangeKind::OperandChanged {
                        index,
                        before,
                        after,
                    },
                });
            }
        }
        let definition_count = old_operation
            .definition_target_count()
            .max(new_operation.definition_target_count());
        for index in 0..definition_count {
            if let (Some(before), Some(after)) = (
                old_operation.definition_target(index),
                new_operation.definition_target(index),
            ) && before != after
            {
                changes.push(Change {
                    node: id,
                    kind: ChangeKind::DefinitionChanged { before, after },
                });
            }
        }
        if let OperationKind::Hole { expected } = old_operation
            && old_operation.code() != new_operation.code()
        {
            changes.push(Change {
                node: id,
                kind: ChangeKind::OperationRefined {
                    before: old_operation.code(),
                    after: new_operation.code(),
                    result_type: *expected,
                    replacement: new_operation.clone(),
                },
            });
        } else if let Some((before, after)) = scalar_operation_change(old_operation, new_operation)
        {
            changes.push(Change {
                node: id,
                kind: ChangeKind::ScalarAttributeChanged { before, after },
            });
        }
    }
}

fn scalar_operation_change(
    old: &OperationKind,
    new: &OperationKind,
) -> Option<(ScalarValue, ScalarValue)> {
    match (old, new) {
        (OperationKind::ConstI64(left), OperationKind::ConstI64(right)) if left != right => {
            Some((ScalarValue::I64(*left), ScalarValue::I64(*right)))
        }
        (OperationKind::ConstBool(left), OperationKind::ConstBool(right)) if left != right => {
            Some((ScalarValue::Bool(*left), ScalarValue::Bool(*right)))
        }
        (OperationKind::ConstBytes(left), OperationKind::ConstBytes(right)) if left != right => {
            Some((
                ScalarValue::Bytes(left.clone()),
                ScalarValue::Bytes(right.clone()),
            ))
        }
        (OperationKind::Hole { expected: left }, OperationKind::Hole { expected: right })
            if left != right =>
        {
            Some((ScalarValue::Type(*left), ScalarValue::Type(*right)))
        }
        (OperationKind::ForI64 { step: left, .. }, OperationKind::ForI64 { step: right, .. })
            if left != right =>
        {
            Some((ScalarValue::I64(*left), ScalarValue::I64(*right)))
        }
        _ => None,
    }
}

fn owned_children_equal(old: &Node, new: &Node) -> bool {
    old.owned_child_count() == new.owned_child_count()
        && (0..old.owned_child_count())
            .all(|index| old.owned_child(index) == new.owned_child(index))
}
