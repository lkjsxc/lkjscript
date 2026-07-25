use crate::semantic::codec::error;
use crate::semantic::schema::{IdentityRelation, ProtocolError, ProtocolErrorCode};
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
                    relation: "renamed".to_string(),
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
                    relation: "replaced_expression".to_string(),
                    old_key: Some(key.clone()),
                    new_key: Some(new.key().to_hex()),
                    old_node: Some(*node),
                    new_node: Some(new_node),
                });
            }
        }
    }
    Ok(output)
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
