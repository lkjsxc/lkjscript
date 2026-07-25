use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::StagedTransaction;

pub(crate) fn publish(transaction: &StagedTransaction, root: &Path) -> Result<(), ProtocolError> {
    let workspace = super::publication_lock::require_workspace(root)?;
    let staging = super::publication_lock::staging_root(&workspace);
    fs::create_dir_all(&staging).map_err(|cause| publication("create staging root", cause))?;
    let id = transaction_id(transaction);
    let journal_path = staging.join(format!("{id}.journal"));
    if journal_path.exists() {
        return Err(failure(
            "pending journal exists after recovery; publication is blocked",
        ));
    }
    let mut journal = super::journal::build(transaction, &workspace)?;
    super::journal::write(&journal_path, &journal)?;
    if let Err(cause) = prepare(transaction, &workspace, &id, &journal.files)
        .and_then(|()| install(transaction, &workspace, &id, &journal.files))
    {
        return rollback_failure(&workspace, &id, &journal_path, &journal.files, cause);
    }
    if let Err(cause) = super::journal::mark_committed(&journal_path, &mut journal) {
        return rollback_failure(&workspace, &id, &journal_path, &journal.files, cause);
    }
    if super::recovery::cleanup(&workspace, &id, &journal.files).is_ok() {
        let _ = fs::remove_file(&journal_path);
        let _ = super::journal::sync_parent(&journal_path);
    }
    Ok(())
}

fn prepare(
    transaction: &StagedTransaction,
    workspace: &Path,
    id: &str,
    records: &[super::journal::JournalFile],
) -> Result<(), ProtocolError> {
    for (index, (source, record)) in transaction.sources.iter().zip(records).enumerate() {
        let paths = super::journal::paths(workspace, record, id, index)?;
        if paths.temporary.exists() || paths.backup.exists() {
            return Err(failure("publication artifact already exists"));
        }
        let current = read_exact(&paths.host, source.old_bytes.len())?;
        if current != source.old_bytes {
            return Err(error(
                ProtocolErrorCode::PreconditionFailed,
                format!("source {} changed before publication", source.logical_path),
            ));
        }
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&paths.temporary)
            .map_err(|cause| publication("create staged source", cause))?;
        file.write_all(&source.new_bytes)
            .and_then(|()| file.sync_all())
            .map_err(|cause| publication("flush staged source", cause))?;
    }
    Ok(())
}

fn install(
    transaction: &StagedTransaction,
    workspace: &Path,
    id: &str,
    records: &[super::journal::JournalFile],
) -> Result<(), ProtocolError> {
    for (index, (source, record)) in transaction.sources.iter().zip(records).enumerate() {
        let paths = super::journal::paths(workspace, record, id, index)?;
        fs::rename(&paths.host, &paths.backup)
            .map_err(|cause| publication("rename source to recovery backup", cause))?;
        verify_backup(&paths.backup, source)?;
        fs::rename(&paths.temporary, &paths.host)
            .map_err(|cause| publication("install staged source", cause))?;
        super::journal::sync_parent(&paths.host)?;
    }
    for (index, (source, record)) in transaction.sources.iter().zip(records).enumerate() {
        let paths = super::journal::paths(workspace, record, id, index)?;
        verify_backup(&paths.backup, source)?;
    }
    Ok(())
}

fn verify_backup(backup: &Path, source: &super::StagedSource) -> Result<(), ProtocolError> {
    let bytes = read_exact(backup, source.old_bytes.len())?;
    if bytes == source.old_bytes {
        Ok(())
    } else {
        Err(error(
            ProtocolErrorCode::PreconditionFailed,
            format!("source {} changed during publication", source.logical_path),
        ))
    }
}

fn read_exact(path: &Path, expected: usize) -> Result<Vec<u8>, ProtocolError> {
    let expected_u64 = u64::try_from(expected).map_err(|_| failure("source size overflow"))?;
    let metadata = path
        .symlink_metadata()
        .map_err(|cause| publication("inspect source", cause))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() != expected_u64 {
        return Err(failure(
            "source identity or size changed during publication",
        ));
    }
    let mut file = File::open(path).map_err(|cause| publication("open source", cause))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take(expected_u64.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|cause| publication("read source", cause))?;
    if bytes.len() != expected {
        return Err(failure("source size changed during publication read"));
    }
    Ok(bytes)
}

fn rollback_failure(
    workspace: &Path,
    id: &str,
    journal: &Path,
    files: &[super::journal::JournalFile],
    cause: ProtocolError,
) -> Result<(), ProtocolError> {
    match super::recovery::rollback(workspace, id, files) {
        Ok(()) => {
            let _ = fs::remove_file(journal);
            let _ = super::journal::sync_parent(journal);
            Err(cause)
        }
        Err(rollback) => Err(error(
            ProtocolErrorCode::PublicationFailed,
            format!(
                "{}; rollback pending after: {}",
                cause.message, rollback.message
            ),
        )),
    }
}

fn transaction_id(transaction: &StagedTransaction) -> String {
    let mut bytes = transaction.tree.revision().as_bytes().to_vec();
    for source in &transaction.sources {
        bytes.extend_from_slice(source.logical_path.as_bytes());
        bytes.extend_from_slice(&lkjscript_core::sha256(&source.new_bytes));
    }
    super::journal::hex(&lkjscript_core::sha256(&bytes))
}

fn publication(action: &str, cause: std::io::Error) -> ProtocolError {
    failure(&format!("{action}: {cause}"))
}

fn failure(message: &str) -> ProtocolError {
    error(ProtocolErrorCode::PublicationFailed, message)
}
