use crate::semantic::codec::error;
use crate::semantic::schema::{Expression, ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::ResolvedOperation;
use crate::source::{SourceFile, ValidatedSourceTree};

pub(crate) fn resolve(
    tree: &ValidatedSourceTree,
    key: &str,
    entity_fingerprint: &str,
    node_index: u32,
    node_fingerprint: &str,
    replacement: &Expression,
) -> Result<ResolvedOperation, ProtocolError> {
    let entity = crate::semantic::operations::entity::read(tree, key, Some(entity_fingerprint))?;
    let node = tree.nodes().get(node_index as usize).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("unknown node {node_index}"),
        )
    })?;
    let owner = crate::semantic::tree::containing_declaration(tree, node).ok_or_else(|| {
        error(
            ProtocolErrorCode::InvalidOperation,
            "node has no containing declaration",
        )
    })?;
    if owner.key().to_hex() != key {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "node is not in the preconditioned entity",
        ));
    }
    let source_nodes = crate::semantic::tree::source_nodes(tree);
    let source = source_nodes
        .get(node_index as usize)
        .ok_or_else(|| error(ProtocolErrorCode::UnknownNode, "node source is unavailable"))?;
    if crate::semantic::tree::fingerprint(source) != node_fingerprint {
        return Err(error(
            ProtocolErrorCode::PreconditionFailed,
            "node fingerprint is stale",
        ));
    }
    let root = source_nodes
        .get(owner.node().index() as usize)
        .ok_or_else(|| {
            error(
                ProtocolErrorCode::UnknownNode,
                "entity source is unavailable",
            )
        })?;
    let path = super::positions::path_from_owner(tree, owner.node().index(), node_index)?;
    if !super::positions::is_expression_path(root, &path) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "target is not an expression position",
        ));
    }
    if !replacement.supports_edition(tree.edition()) {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "replacement expression requires Edition 2",
        ));
    }
    let _ = replacement
        .to_source(source.span)
        .map_err(|message| error(ProtocolErrorCode::InvalidOperation, message))?;
    let _ = entity;
    Ok(ResolvedOperation::Replace {
        key: key.to_string(),
        node: node_index,
        path,
        replacement: replacement.clone(),
        relation: crate::semantic::schema::IdentityRelationKind::ReplacedExpression,
    })
}

pub(crate) fn apply(
    files: &mut [SourceFile],
    operation: &ResolvedOperation,
) -> Result<(), ProtocolError> {
    let ResolvedOperation::Replace {
        node, replacement, ..
    } = operation
    else {
        return Ok(());
    };
    let target = super::nodes::node_mut(files, *node).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            "resolved replacement node disappeared",
        )
    })?;
    let mut source = replacement
        .to_source(target.span)
        .map_err(|message| error(ProtocolErrorCode::InvalidOperation, message))?;
    source.leading_trivia = std::mem::take(&mut target.leading_trivia);
    source.before_close_trivia = std::mem::take(&mut target.before_close_trivia);
    *target = source;
    Ok(())
}
