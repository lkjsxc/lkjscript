mod files;
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
pub(crate) use positions::{is_expression_path, path_from_owner};
pub(crate) use publish::publish;
pub(crate) use stage::stage;

pub(crate) fn begin(
    root: &std::path::Path,
) -> Result<Option<publication_lock::PublicationGuard>, crate::semantic::schema::ProtocolError> {
    publication_lock::acquire(root)
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
