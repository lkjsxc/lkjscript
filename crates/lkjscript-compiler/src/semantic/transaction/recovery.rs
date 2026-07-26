use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::semantic::schema::ProtocolError;

use super::journal::{self, Journal, JournalFile, JournalState};

const MAX_JOURNALS: usize = 64;

pub(super) fn recover_all(workspace: &Path, staging: &Path) -> Result<(), ProtocolError> {
    let mut journals = Vec::new();
    for item in
        fs::read_dir(staging).map_err(|cause| journal::io_failure("read staging root", cause))?
    {
        let path = item
            .map_err(|cause| journal::io_failure("read staging entry", cause))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("tmp") {
            fs::remove_file(path)
                .map_err(|cause| journal::io_failure("remove journal temporary", cause))?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("journal") {
            journals.push(path);
        }
        if journals.len() > MAX_JOURNALS {
            return Err(journal::failure("too many pending publication journals"));
        }
    }
    journals.sort();
    for path in journals {
        recover(workspace, &path)?;
    }
    Ok(())
}

fn recover(workspace: &Path, path: &Path) -> Result<(), ProtocolError> {
    let id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| journal::failure("journal filename is not UTF-8"))?;
    if !journal::is_hash(id) {
        return Err(journal::failure(
            "journal filename is not a transaction identity",
        ));
    }
    let file = File::open(path).map_err(|cause| journal::io_failure("open journal", cause))?;
    if file
        .metadata()
        .map_err(|cause| journal::io_failure("inspect journal", cause))?
        .len()
        > journal::MAX_JOURNAL_BYTES
    {
        return Err(journal::failure("publication journal exceeds byte limit"));
    }
    let mut bytes = Vec::new();
    file.take(journal::MAX_JOURNAL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|cause| journal::io_failure("read journal", cause))?;
    if u64::try_from(bytes.len()).map_err(|_| journal::failure("journal size overflow"))?
        > journal::MAX_JOURNAL_BYTES
    {
        return Err(journal::failure("publication journal exceeds byte limit"));
    }
    let record: Journal = serde_json::from_slice(&bytes)
        .map_err(|cause| journal::failure(&format!("strict journal decode failed: {cause}")))?;
    journal::validate_header(&record)?;
    match record.state {
        JournalState::Prepared => rollback(workspace, id, &record.files)?,
        JournalState::Committed => cleanup(workspace, id, &record.files)?,
    }
    fs::remove_file(path)
        .map_err(|cause| journal::io_failure("remove recovered journal", cause))?;
    journal::sync_parent(path)
}

pub(super) fn rollback(
    workspace: &Path,
    id: &str,
    files: &[JournalFile],
) -> Result<(), ProtocolError> {
    for (index, record) in files.iter().enumerate().rev() {
        let paths = journal::paths(workspace, record, id, index)?;
        if paths.backup.exists() {
            let backup = digest_existing(&paths.backup)?
                .ok_or_else(|| journal::failure("source backup disappeared"))?;
            if backup != record.old_sha256 {
                return Err(journal::failure("source backup hash changed"));
            }
            match digest_existing(&paths.host)? {
                None => restore_backup(&paths.backup, &paths.host)?,
                Some(hash) if hash == record.new_sha256 => {
                    fs::remove_file(&paths.host)
                        .map_err(|cause| journal::io_failure("remove staged source", cause))?;
                    restore_backup(&paths.backup, &paths.host)?;
                }
                Some(hash) if hash == record.old_sha256 => {
                    fs::remove_file(&paths.backup)
                        .map_err(|cause| journal::io_failure("remove duplicate backup", cause))?;
                }
                Some(_) => {
                    fs::remove_file(&paths.backup).map_err(|cause| {
                        journal::io_failure("preserve external source and remove backup", cause)
                    })?;
                }
            }
        }
        let _ = fs::remove_file(&paths.temporary);
        journal::sync_parent(&paths.host)?;
    }
    Ok(())
}

fn restore_backup(backup: &Path, host: &Path) -> Result<(), ProtocolError> {
    fs::rename(backup, host).map_err(|cause| journal::io_failure("restore source backup", cause))
}

fn digest_existing(path: &Path) -> Result<Option<String>, ProtocolError> {
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(journal::io_failure("inspect publication leaf", cause)),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > crate::source::FOUNDATION_MAX_SOURCE_FILE_BYTES
    {
        return Err(journal::failure(
            "publication leaf is not a bounded regular file",
        ));
    }
    let limit = crate::source::FOUNDATION_MAX_SOURCE_FILE_BYTES;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|cause| journal::io_failure("open publication leaf", cause))?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|cause| journal::io_failure("read publication leaf", cause))?;
    if u64::try_from(bytes.len()).map_err(|_| journal::failure("source size overflow"))? > limit {
        return Err(journal::failure("publication leaf exceeds source limit"));
    }
    Ok(Some(journal::hex(&lkjscript_core::sha256(&bytes))))
}

#[cfg(test)]
pub(crate) fn publish_with_install_failure(
    transaction: &super::StagedTransaction,
    root: &Path,
) -> Result<(), ProtocolError> {
    let workspace = super::publication_lock::require_workspace(root)?;
    let staging = super::publication_lock::staging_root(&workspace);
    fs::create_dir_all(&staging)
        .map_err(|cause| journal::failure(&format!("create staging root: {cause}")))?;
    let id = super::publish::transaction_id(transaction);
    let journal_path = staging.join(format!("{id}.journal"));
    let record = journal::build(transaction, &workspace)?;
    journal::write(&journal_path, &record)?;
    let result =
        super::publish::prepare(transaction, &workspace, &id, &record.files).and_then(|()| {
            super::publish::install_with_failure(
                transaction,
                &workspace,
                &id,
                &record.files,
                Some(1),
            )
        });
    let cause = result.err().ok_or_else(|| {
        crate::semantic::codec::error(
            crate::semantic::schema::ProtocolErrorCode::PublicationFailed,
            "injected publication failure unexpectedly succeeded",
        )
    })?;
    super::publish::rollback_failure(&workspace, &id, &journal_path, &record.files, cause)
}

pub(super) fn cleanup(
    workspace: &Path,
    id: &str,
    files: &[JournalFile],
) -> Result<(), ProtocolError> {
    for (index, record) in files.iter().enumerate() {
        let paths = journal::paths(workspace, record, id, index)?;
        let _ = fs::remove_file(paths.temporary);
        let _ = fs::remove_file(paths.backup);
        journal::sync_parent(&paths.host)?;
    }
    Ok(())
}
