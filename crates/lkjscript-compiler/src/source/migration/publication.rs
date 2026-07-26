use std::path::Path;

use lkjscript_core::ResourceProfile;

use crate::source::{
    DiagnosticCategory, RevisionId, SourceDiagnostic, SourceEdition, SourceResult, SourceSpan,
    ValidatedSourceTree,
};

use super::{check_edition2_migration, diagnostic, EditionMigrationPlan};

pub fn diff_edition2_migration(
    root: &Path,
    expected_revision: RevisionId,
    limits: &lkjscript_core::Limits,
    profile: ResourceProfile,
) -> SourceResult<EditionMigrationPlan> {
    let tree = crate::source::load(root, limits)?;
    check_edition2_migration(&tree, expected_revision, limits, profile)
}

pub fn publish_edition2_migration(
    root: &Path,
    checked: &EditionMigrationPlan,
    limits: &lkjscript_core::Limits,
    profile: ResourceProfile,
) -> SourceResult<EditionMigrationPlan> {
    let guard = crate::semantic::transaction::begin(root)
        .map_err(|error| protocol_diagnostic(root, error))?;
    if guard.is_none() {
        return Err(path_diagnostic(
            root,
            "LKJ-SRC-MIGRATION-PUBLICATION",
            "migration root is not contained by a repository workspace",
        ));
    }
    let tree = crate::source::load(root, limits)?;
    let exact = check_edition2_migration(&tree, checked.old_revision(), limits, profile)?;
    if &exact != checked {
        return Err(diagnostic(
            &tree,
            "LKJ-SRC-MIGRATION-CONFLICT",
            "checked migration identities or replacement bytes changed before publication",
        ));
    }
    if exact.is_idempotent() {
        return Ok(exact);
    }
    let transaction = build_transaction(tree, &exact)?;
    crate::semantic::transaction::publish(&transaction, root)
        .map_err(|error| protocol_diagnostic(root, error))?;
    let published = crate::source::load(root, limits)?;
    if published.revision() != exact.new_revision()
        || published.identity() != exact.new_tree_identity()
        || published.edition() != SourceEdition::Edition2
    {
        return Err(diagnostic(
            &published,
            "LKJ-SRC-MIGRATION-PUBLICATION",
            "published closure identity does not match checked migration",
        ));
    }
    drop(guard);
    Ok(exact)
}

fn build_transaction(
    tree: ValidatedSourceTree,
    plan: &EditionMigrationPlan,
) -> SourceResult<crate::semantic::transaction::StagedTransaction> {
    let mut sources = Vec::with_capacity(plan.changes().len());
    let mut changes = Vec::with_capacity(plan.changes().len());
    for change in plan.changes() {
        let file = tree
            .files()
            .iter()
            .find(|file| file.origin.logical_path == change.path())
            .ok_or_else(|| {
                diagnostic(
                    &tree,
                    "LKJ-SRC-MIGRATION-CONFLICT",
                    "checked migration source disappeared",
                )
            })?;
        let new_bytes = change.replacement_source().as_bytes().to_vec();
        changes.push(crate::semantic::schema::ChangedSource {
            path: change.path().to_string(),
            old_sha256: crate::semantic::tree::hex(&file.exact_source_sha256),
            new_sha256: crate::semantic::tree::hex(&lkjscript_core::sha256(&new_bytes)),
            bytes: change.new_bytes(),
        });
        sources.push(crate::semantic::transaction::StagedSource {
            logical_path: change.path().to_string(),
            host_path: file.path.clone(),
            old_bytes: file.exact_source.as_bytes().to_vec(),
            new_bytes,
        });
    }
    Ok(crate::semantic::transaction::StagedTransaction {
        tree,
        sources,
        changes,
        identities: Vec::new(),
    })
}

#[cfg(test)]
pub(crate) fn simulate_checked_crash(
    root: &Path,
    checked: &EditionMigrationPlan,
    limits: &lkjscript_core::Limits,
) -> SourceResult<std::path::PathBuf> {
    let tree = crate::source::load(root, limits)?;
    let transaction = build_transaction(tree, checked)?;
    crate::semantic::transaction::simulate_prepared_crash(&transaction, root)
        .map_err(|error| protocol_diagnostic(root, error))
}

#[cfg(test)]
pub(crate) fn simulate_checked_rollback(
    root: &Path,
    checked: &EditionMigrationPlan,
    limits: &lkjscript_core::Limits,
) -> SourceResult<()> {
    let tree = crate::source::load(root, limits)?;
    let transaction = build_transaction(tree, checked)?;
    match crate::semantic::transaction::publish_with_install_failure(&transaction, root) {
        Err(_) => Ok(()),
        Ok(()) => Err(diagnostic(
            &transaction.tree,
            "LKJ-SRC-MIGRATION-PUBLICATION",
            "rollback simulation unexpectedly published",
        )),
    }
}

fn protocol_diagnostic(
    root: &Path,
    error: crate::semantic::schema::ProtocolError,
) -> SourceDiagnostic {
    path_diagnostic(root, "LKJ-SRC-MIGRATION-PUBLICATION", error.message)
}

fn path_diagnostic(
    root: &Path,
    code: &'static str,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        code,
        DiagnosticCategory::SourceLoading,
        message,
        crate::source::SourceOrigin::in_memory(&root.to_string_lossy()),
        SourceSpan::zero(),
    )
}
