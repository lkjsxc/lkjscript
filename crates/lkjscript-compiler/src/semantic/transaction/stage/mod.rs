mod preflight;

use std::collections::HashSet;

use lkjscript_core::{BudgetLedger, Limits};

use crate::semantic::codec::error;
use crate::semantic::schema::{
    FilePrecondition, ProtocolError, ProtocolErrorCode, TransactionOperation,
};
use crate::semantic::transaction::{ResolvedOperation, StagedTransaction};
use crate::source::ValidatedSourceTree;

pub(crate) fn stage_with_ledger(
    tree: &ValidatedSourceTree,
    requests: &[TransactionOperation],
    preconditions: &[FilePrecondition],
    ledger: &mut BudgetLedger,
) -> Result<StagedTransaction, ProtocolError> {
    if requests.is_empty() {
        return Err(error(
            ProtocolErrorCode::InvalidOperation,
            "transaction has no operations",
        ));
    }
    preflight::reserve(tree, requests, ledger)?;
    let resolved = resolve_all(tree, requests)?;
    reject_overlaps(&resolved)?;
    let mut files = tree.files().to_vec();
    for operation in &resolved {
        if let ResolvedOperation::Rename { key, .. } = operation {
            let kind = crate::semantic::operations::entity::find(tree, key)?.kind();
            super::rename::apply(&mut files, operation, kind)?;
        }
    }
    let mut mutations: Vec<_> = resolved
        .iter()
        .filter(|operation| {
            matches!(
                operation,
                ResolvedOperation::Replace { .. } | ResolvedOperation::DeleteHole { .. }
            )
        })
        .collect();
    mutations.sort_by_key(|operation| match operation {
        ResolvedOperation::Replace { node, .. } | ResolvedOperation::DeleteHole { node, .. } => {
            std::cmp::Reverse(*node)
        }
        ResolvedOperation::Rename { .. } => std::cmp::Reverse(0),
    });
    for operation in mutations {
        match operation {
            ResolvedOperation::Replace { .. } => super::replace::apply(&mut files, operation)?,
            ResolvedOperation::DeleteHole { .. } => {
                super::holes::apply_delete(&mut files, operation)?
            }
            ResolvedOperation::Rename { .. } => {}
        }
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
    let validation = if crate::semantic::tree::source_nodes(&rebuilt).iter().any(
        |node| matches!(&node.kind, crate::source::SyntaxKind::Call { name } if name == "hole"),
    ) {
        crate::semantic::operations::holes::validate::validate_incomplete(&rebuilt)
    } else {
        crate::analyze::analyze_program(&rebuilt)
            .map(|_| ())
            .map_err(|failure| failure.to_string())
    };
    if let Err(message) = validation {
        let mut protocol = error(ProtocolErrorCode::ValidationFailed, message);
        protocol.diagnostic = crate::semantic::operations::diagnostics::collect(&rebuilt, true)
            .into_iter()
            .next()
            .map(Box::new);
        return Err(protocol);
    }
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
            TransactionOperation::InsertHole { .. }
            | TransactionOperation::FillHole { .. }
            | TransactionOperation::RefineHole { .. }
            | TransactionOperation::DeleteHole { .. } => super::holes::resolve(tree, request),
        })
        .collect()
}

fn reject_overlaps(operations: &[ResolvedOperation]) -> Result<(), ProtocolError> {
    let mut declarations = HashSet::new();
    let mut replacements: Vec<&[usize]> = Vec::new();
    for operation in operations {
        match operation {
            ResolvedOperation::Rename { key, .. } if !declarations.insert(key) => {
                return Err(error(
                    ProtocolErrorCode::InvalidOperation,
                    "declaration renamed more than once",
                ));
            }
            ResolvedOperation::Replace { path, .. }
            | ResolvedOperation::DeleteHole { path, .. } => {
                if replacements
                    .iter()
                    .any(|prior| path.starts_with(prior) || prior.starts_with(path.as_slice()))
                {
                    return Err(error(
                        ProtocolErrorCode::InvalidOperation,
                        "expression replacements overlap",
                    ));
                }
                replacements.push(path);
            }
            _ => {}
        }
    }
    Ok(())
}
