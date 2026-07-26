use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::semantic::codec::error;
use crate::semantic::schema::{ProtocolError, ProtocolErrorCode};
use crate::semantic::transaction::StagedTransaction;

pub(super) const MAX_JOURNAL_BYTES: u64 = 1_048_576;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    schema: String,
    contract: String,
    pub state: JournalState,
    pub files: Vec<JournalFile>,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum JournalState {
    Prepared,
    Committed,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JournalFile {
    pub relative: String,
    pub old_sha256: String,
    pub new_sha256: String,
}

pub(super) struct ArtifactPaths {
    pub host: PathBuf,
    pub temporary: PathBuf,
    pub backup: PathBuf,
    _directory: super::directory_anchor::AnchoredDirectory,
}

pub(super) fn build(
    transaction: &StagedTransaction,
    workspace: &Path,
) -> Result<Journal, ProtocolError> {
    let mut files = Vec::with_capacity(transaction.sources.len());
    for source in &transaction.sources {
        let relative = source
            .host_path
            .strip_prefix(workspace)
            .map_err(|_| failure("changed source is outside the publication workspace"))?;
        let relative = relative
            .to_str()
            .ok_or_else(|| failure("source path is not UTF-8"))?;
        if !safe_relative(relative) {
            return Err(failure("source path is not a canonical relative path"));
        }
        files.push(JournalFile {
            relative: relative.to_string(),
            old_sha256: hex(&lkjscript_core::sha256(&source.old_bytes)),
            new_sha256: hex(&lkjscript_core::sha256(&source.new_bytes)),
        });
    }
    Ok(Journal {
        schema: "lkjscript.publication-journal".into(),
        contract: crate::semantic::CONTRACT.to_hex(),
        state: JournalState::Prepared,
        files,
    })
}

pub(super) fn mark_committed(path: &Path, journal: &mut Journal) -> Result<(), ProtocolError> {
    journal.state = JournalState::Committed;
    write(path, journal)
}

pub(super) fn write(path: &Path, journal: &Journal) -> Result<(), ProtocolError> {
    let estimated = journal.files.iter().try_fold(256usize, |total, record| {
        total.checked_add(record.relative.len().saturating_add(256))
    });
    let estimated = estimated.ok_or_else(|| failure("journal size overflow"))?;
    if u64::try_from(estimated).map_err(|_| failure("journal size overflow"))? > MAX_JOURNAL_BYTES {
        return Err(failure("publication journal exceeds byte limit"));
    }
    let bytes = serde_json::to_vec(journal).map_err(|cause| failure(&cause.to_string()))?;
    if u64::try_from(bytes.len()).map_err(|_| failure("journal size overflow"))? > MAX_JOURNAL_BYTES
    {
        return Err(failure("publication journal exceeds byte limit"));
    }
    let temporary = path.with_extension("journal.tmp");
    let _ = fs::remove_file(&temporary);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|cause| io_failure("create journal temporary", cause))?;
    file.write_all(&bytes)
        .and_then(|()| file.sync_all())
        .map_err(|cause| io_failure("write journal temporary", cause))?;
    fs::rename(&temporary, path).map_err(|cause| io_failure("replace journal", cause))?;
    sync_parent(path)
}

pub(super) fn paths(
    workspace: &Path,
    record: &JournalFile,
    id: &str,
    index: usize,
) -> Result<ArtifactPaths, ProtocolError> {
    if !safe_relative(&record.relative)
        || !is_hash(&record.old_sha256)
        || !is_hash(&record.new_sha256)
    {
        return Err(failure("publication journal record is invalid"));
    }
    let lexical_host = workspace.join(&record.relative);
    let lexical_directory = lexical_host
        .parent()
        .ok_or_else(|| failure("source has no parent"))?;
    let leaf = lexical_host
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| failure("source filename is not UTF-8"))?
        .to_string();
    let directory = super::directory_anchor::AnchoredDirectory::open(lexical_directory)?;
    Ok(ArtifactPaths {
        host: directory.join(&leaf),
        temporary: directory.join(format!(".{leaf}.lkjscript-stage-{id}-{index}")),
        backup: directory.join(format!(".{leaf}.lkjscript-backup-{id}-{index}")),
        _directory: directory,
    })
}

pub(super) fn validate_header(journal: &Journal) -> Result<(), ProtocolError> {
    if journal.schema == "lkjscript.publication-journal"
        && lkjscript_contracts::ContractDigest::from_hex(&journal.contract)
            == Some(crate::semantic::CONTRACT)
    {
        Ok(())
    } else {
        Err(failure(
            "publication journal contract mismatch; discard stale journal",
        ))
    }
}

fn safe_relative(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && Path::new(part)
                    .components()
                    .all(|item| matches!(item, Component::Normal(_)))
        })
}

pub(super) fn is_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn hex(bytes: &[u8; 32]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(super) fn sync_parent(path: &Path) -> Result<(), ProtocolError> {
    let parent = path.parent().ok_or_else(|| failure("path has no parent"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|cause| io_failure("synchronize directory", cause))
}

pub(super) fn io_failure(action: &str, cause: std::io::Error) -> ProtocolError {
    failure(&format!("{action}: {cause}"))
}

pub(super) fn failure(message: &str) -> ProtocolError {
    error(ProtocolErrorCode::PublicationFailed, message)
}
