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
            if paths.host.exists() {
                fs::remove_file(&paths.host)
                    .map_err(|cause| journal::io_failure("remove staged source", cause))?;
            }
            fs::rename(&paths.backup, &paths.host)
                .map_err(|cause| journal::io_failure("restore source backup", cause))?;
        }
        let _ = fs::remove_file(&paths.temporary);
        journal::sync_parent(&paths.host)?;
    }
    Ok(())
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
