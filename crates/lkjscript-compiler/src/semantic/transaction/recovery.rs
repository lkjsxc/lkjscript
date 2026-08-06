use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::semantic::schema::ProtocolError;

use super::journal::{self, Journal, JournalFile, JournalState};

const MAX_JOURNALS: usize = 64;
const SOURCE_DIGEST_CHUNK_BYTES: usize = 64 * 1024;

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

pub(super) fn digest_existing(path: &Path) -> Result<Option<String>, ProtocolError> {
    let metadata = match path.symlink_metadata() {
        Ok(metadata) => metadata,
        Err(cause) if cause.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(cause) => return Err(journal::io_failure("inspect publication leaf", cause)),
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(journal::failure("publication leaf is not a regular file"));
    }

    let mut file =
        File::open(path).map_err(|cause| journal::io_failure("open publication leaf", cause))?;
    let opened_metadata = file
        .metadata()
        .map_err(|cause| journal::io_failure("inspect opened publication leaf", cause))?;
    if !opened_metadata.is_file() || opened_metadata.len() != metadata.len() {
        return Err(journal::failure(
            "publication leaf identity or size changed before digesting",
        ));
    }

    let initial_capacity = usize::try_from(metadata.len())
        .unwrap_or(usize::MAX)
        .min(SOURCE_DIGEST_CHUNK_BYTES);
    let mut bytes = Vec::new();
    bytes
        .try_reserve(initial_capacity)
        .map_err(|_| journal::failure("host could not reserve publication digest memory"))?;
    let mut chunk = [0_u8; SOURCE_DIGEST_CHUNK_BYTES];
    loop {
        let read = match file.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => read,
            Err(cause) if cause.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(cause) => return Err(journal::io_failure("read publication leaf", cause)),
        };
        let next_len = bytes
            .len()
            .checked_add(read)
            .ok_or_else(|| journal::failure("publication digest size overflow"))?;
        bytes
            .try_reserve(read)
            .map_err(|_| journal::failure("host could not reserve publication digest memory"))?;
        bytes.extend_from_slice(&chunk[..read]);
        debug_assert_eq!(bytes.len(), next_len);
    }
    let actual_bytes = u64::try_from(bytes.len())
        .map_err(|_| journal::failure("publication digest size overflow"))?;
    let final_metadata = file
        .metadata()
        .map_err(|cause| journal::io_failure("reinspect publication leaf", cause))?;
    if !final_metadata.is_file()
        || actual_bytes != metadata.len()
        || final_metadata.len() != metadata.len()
    {
        return Err(journal::failure(
            "publication leaf size changed while digesting",
        ));
    }
    Ok(Some(journal::hex(&lkjscript_core::sha256(&bytes))))
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
