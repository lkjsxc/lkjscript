use std::collections::HashSet;

use crate::semantic::codec::error;
use crate::semantic::schema::{
    IdentityRelation, IdentityRelationKind, ProtocolError, ProtocolErrorCode,
};
use crate::semantic::transaction::ResolvedOperation;
use crate::source::ValidatedSourceTree;

pub(super) fn identity_relations(
    base: &ValidatedSourceTree,
    next: &ValidatedSourceTree,
    operations: &[ResolvedOperation],
) -> Result<Vec<IdentityRelation>, ProtocolError> {
    let mut output = Vec::new();
    for operation in operations {
        match operation {
            ResolvedOperation::Rename { key, new_name, .. } => {
                let old = crate::semantic::operations::entity::find(base, key)?;
                let new = next
                    .declarations()
                    .iter()
                    .find(|decl| {
                        decl.kind() == old.kind()
                            && decl.name() == new_name
                            && decl.origin().logical_path() == old.origin().logical_path()
                    })
                    .ok_or_else(|| {
                        error(
                            ProtocolErrorCode::ValidationFailed,
                            "renamed identity was not rebuilt",
                        )
                    })?;
                output.push(IdentityRelation {
                    relation: IdentityRelationKind::RenamedDeclaration,
                    old_key: Some(key.clone()),
                    new_key: Some(new.key().to_hex()),
                    old_node: Some(old.node().index()),
                    new_node: Some(new.node().index()),
                });
            }
            ResolvedOperation::Replace {
                key, node, path, ..
            } => {
                let old = crate::semantic::operations::entity::find(base, key)?;
                let new_name = operations
                    .iter()
                    .find_map(|candidate| match candidate {
                        ResolvedOperation::Rename {
                            key: renamed,
                            new_name,
                            ..
                        } if renamed == key => Some(new_name.as_str()),
                        _ => None,
                    })
                    .unwrap_or_else(|| old.name());
                let new = next
                    .declarations()
                    .iter()
                    .find(|candidate| {
                        candidate.kind() == old.kind()
                            && candidate.name() == new_name
                            && candidate.origin().logical_path() == old.origin().logical_path()
                    })
                    .ok_or_else(|| {
                        error(
                            ProtocolErrorCode::ValidationFailed,
                            "replacement owner identity was not rebuilt",
                        )
                    })?;
                let new_node = follow_path(next, new.node().index(), path)?;
                output.push(IdentityRelation {
                    relation: IdentityRelationKind::ReplacedExpression,
                    old_key: Some(key.clone()),
                    new_key: Some(new.key().to_hex()),
                    old_node: Some(*node),
                    new_node: Some(new_node),
                });
            }
        }
    }
    append_changed_owners(base, next, operations, &mut output);
    output.sort_by(|left, right| {
        left.old_key
            .cmp(&right.old_key)
            .then_with(|| left.relation.cmp(&right.relation))
    });
    Ok(output)
}

fn append_changed_owners(
    base: &ValidatedSourceTree,
    next: &ValidatedSourceTree,
    operations: &[ResolvedOperation],
    output: &mut Vec<IdentityRelation>,
) {
    let direct: HashSet<_> = output
        .iter()
        .filter_map(|item| item.old_key.clone())
        .collect();
    let base_nodes = crate::semantic::tree::source_nodes(base);
    let next_nodes = crate::semantic::tree::source_nodes(next);
    for old in base.declarations() {
        let old_key = old.key().to_hex();
        if direct.contains(&old_key) {
            continue;
        }
        let renamed = operations.iter().find_map(|operation| match operation {
            ResolvedOperation::Rename { key, new_name, .. } if key == &old_key => Some(new_name),
            _ => None,
        });
        let expected_name = renamed.map_or_else(|| old.name(), String::as_str);
        let Some(new) = next.declarations().iter().find(|candidate| {
            candidate.kind() == old.kind()
                && candidate.name() == expected_name
                && candidate.origin().logical_path() == old.origin().logical_path()
        }) else {
            continue;
        };
        let old_index = usize::try_from(old.node().index()).ok();
        let new_index = usize::try_from(new.node().index()).ok();
        let changed = old_index
            .and_then(|index| base_nodes.get(index))
            .zip(new_index.and_then(|index| next_nodes.get(index)))
            .is_some_and(|(left, right)| {
                crate::semantic::tree::fingerprint(left)
                    != crate::semantic::tree::fingerprint(right)
            });
        if changed {
            output.push(IdentityRelation {
                relation: IdentityRelationKind::ChangedReferenceOwner,
                old_key: Some(old_key),
                new_key: Some(new.key().to_hex()),
                old_node: Some(old.node().index()),
                new_node: Some(new.node().index()),
            });
        }
    }
}

fn follow_path(
    tree: &ValidatedSourceTree,
    mut node: u32,
    path: &[usize],
) -> Result<u32, ProtocolError> {
    for child in path {
        node = tree
            .nodes()
            .get(node as usize)
            .and_then(|summary| summary.children().get(*child))
            .map(|id| id.index())
            .ok_or_else(|| {
                error(
                    ProtocolErrorCode::ValidationFailed,
                    "replacement node relation was not rebuilt",
                )
            })?;
    }
    Ok(node)
}
