mod directory_anchor;
mod files;
mod holes;
mod journal;
mod model;
mod nodes;
mod positions;
mod publication_lock;
mod publish;
mod recovery;
mod references;
mod relations;
mod rename;
mod replace;
mod stage;

pub(crate) use model::{ResolvedOperation, StagedSource, StagedTransaction};
pub(crate) use nodes::node_mut;
pub(crate) use positions::{is_expression_path, path_from_owner};
pub(crate) use publication_lock::PublicationGuard;
pub(crate) use publish::publish;
pub(crate) use stage::stage_with_ledger;

#[cfg(test)]
pub(crate) fn stage(
    tree: &crate::source::ValidatedSourceTree,
    requests: &[crate::semantic::schema::TransactionOperation],
    preconditions: &[crate::semantic::schema::FilePrecondition],
    profile: crate::semantic::schema::ResourceProfile,
) -> Result<StagedTransaction, crate::semantic::schema::ProtocolError> {
    let mut ledger = lkjscript_core::BudgetLedger::new(profile.core());
    stage_with_ledger(tree, requests, preconditions, &mut ledger)
}

pub(crate) fn begin(
    root: &std::path::Path,
) -> Result<Option<publication_lock::PublicationGuard>, crate::semantic::schema::ProtocolError> {
    publication_lock::acquire(root)
}

#[cfg(test)]
pub(crate) fn simulate_external_leaf_conflict(
    transaction: &StagedTransaction,
    root: &std::path::Path,
    external: &[u8],
) -> Result<(), crate::semantic::schema::ProtocolError> {
    let workspace = publication_lock::require_workspace(root)?;
    let id = "b".repeat(64);
    let record = journal::build(transaction, &workspace)?;
    let source = transaction
        .sources
        .first()
        .ok_or_else(|| journal::failure("test transaction has no source"))?;
    let item = record
        .files
        .first()
        .ok_or_else(|| journal::failure("test journal has no source"))?;
    let paths = journal::paths(&workspace, item, &id, 0)?;
    std::fs::write(&paths.temporary, &source.new_bytes)
        .map_err(|cause| journal::io_failure("write test staged source", cause))?;
    std::fs::rename(&paths.host, &paths.backup)
        .map_err(|cause| journal::io_failure("move test source backup", cause))?;
    std::fs::write(&paths.host, external)
        .map_err(|cause| journal::io_failure("write external conflict", cause))?;
    if std::fs::hard_link(&paths.temporary, &paths.host).is_ok() {
        return Err(journal::failure(
            "no-replace test unexpectedly overwrote leaf",
        ));
    }
    recovery::rollback(&workspace, &id, &record.files)
}

#[cfg(test)]
pub(crate) fn simulate_prepared_crash(
    transaction: &StagedTransaction,
    root: &std::path::Path,
) -> Result<std::path::PathBuf, crate::semantic::schema::ProtocolError> {
    let workspace = publication_lock::require_workspace(root)?;
    let staging = publication_lock::staging_root(&workspace);
    std::fs::create_dir_all(&staging)
        .map_err(|cause| journal::io_failure("create test staging", cause))?;
    let id = "a".repeat(64);
    let path = staging.join(format!("{id}.journal"));
    let record = journal::build(transaction, &workspace)?;
    journal::write(&path, &record)?;
    for (index, (source, item)) in transaction.sources.iter().zip(&record.files).enumerate() {
        let artifacts = journal::paths(&workspace, item, &id, index)?;
        std::fs::write(&artifacts.temporary, &source.new_bytes)
            .map_err(|cause| journal::io_failure("write test staged source", cause))?;
        std::fs::rename(&artifacts.host, &artifacts.backup)
            .map_err(|cause| journal::io_failure("move test source backup", cause))?;
        if index == 0 {
            std::fs::rename(&artifacts.temporary, &artifacts.host)
                .map_err(|cause| journal::io_failure("install test mixed source", cause))?;
        }
    }
    Ok(path)
}
