use std::collections::HashSet;

use lkjscript_core::Limits;

use crate::semantic::codec::error;
use crate::semantic::schema::{
    FilePrecondition, ProtocolError, ProtocolErrorCode, TransactionOperation,
};
use crate::semantic::transaction::{ResolvedOperation, StagedTransaction};
use crate::source::ValidatedSourceTree;

pub(crate) fn stage(
    tree: &ValidatedSourceTree,
    requests: &[TransactionOperation],
    preconditions: &[FilePrecondition],
) -> Result<StagedTransaction, ProtocolError> {
    if requests.is_empty() {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "transaction has no operations",
        ));
    }
    let resolved = resolve_all(tree, requests)?;
    reject_overlaps(&resolved)?;
    let mut files = tree.files().to_vec();
    for operation in &resolved {
        if let ResolvedOperation::Rename { key, .. } = operation {
            let kind = crate::semantic::operations::entity::find(tree, key)?.kind();
            super::rename::apply(&mut files, operation, kind)?;
        }
    }
    let mut replacements: Vec<_> = resolved
        .iter()
        .filter(|operation| matches!(operation, ResolvedOperation::Replace { .. }))
        .collect();
    replacements.sort_by_key(|operation| match operation {
        ResolvedOperation::Replace { node, .. } => std::cmp::Reverse(*node),
        ResolvedOperation::Rename { .. } => std::cmp::Reverse(0),
    });
    for operation in replacements {
        super::replace::apply(&mut files, operation)?;
    }
    let mut rebuilt_sources = Vec::with_capacity(files.len());
    for file in &files {
        rebuilt_sources.push((
            file.path.clone(),
            file.origin.clone(),
            crate::source::format_file(file),
        ));
    }
    let rebuilt = crate::source::rebuild_staged_sources(
        &rebuilt_sources,
        tree.root_path().to_path_buf(),
        tree.root_origin().clone(),
        &Limits::default(),
    )
    .map_err(|failure| error(ProtocolErrorCode::ValidationFailed, failure.render_human()))?;
    crate::analyze::analyze_program(&rebuilt)
        .map_err(|failure| error(ProtocolErrorCode::ValidationFailed, failure.to_string()))?;
    let (sources, changes) = super::files::changed_sources(tree, &rebuilt_sources)?;
    super::files::check_preconditions(tree, &changes, preconditions)?;
    let identities = super::relations::identity_relations(tree, &rebuilt, &resolved)?;
    Ok(StagedTransaction {
        tree: rebuilt,
        sources,
        changes,
        identities,
    })
}

fn resolve_all(
    tree: &ValidatedSourceTree,
    requests: &[TransactionOperation],
) -> Result<Vec<ResolvedOperation>, ProtocolError> {
    requests
        .iter()
        .map(|request| match request {
            TransactionOperation::RenameDeclaration {
                declaration_key,
                entity_fingerprint,
                new_name,
            } => super::rename::resolve(tree, declaration_key, entity_fingerprint, new_name),
            TransactionOperation::ReplaceExpression {
                declaration_key,
                entity_fingerprint,
                node,
                node_fingerprint,
                expression,
            } => super::replace::resolve(
                tree,
                declaration_key,
                entity_fingerprint,
                *node,
                node_fingerprint,
                expression,
            ),
        })
        .collect()
}

fn reject_overlaps(operations: &[ResolvedOperation]) -> Result<(), ProtocolError> {
    let mut declarations = HashSet::new();
    let mut nodes = HashSet::new();
    for operation in operations {
        match operation {
            ResolvedOperation::Rename { key, .. } if !declarations.insert(key) => {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "declaration renamed more than once",
                ));
            }
            ResolvedOperation::Replace { node, .. } if !nodes.insert(node) => {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "expression replaced more than once",
                ));
            }
            _ => {}
        }
    }
    Ok(())
}
